const PREFIX = "ferroui.todo.graph.v1.";
const BINDING = `${PREFIX}document-account`;
const JOURNAL = `${PREFIX}commit`;
const DOCUMENT = "ferroui.todo.v1";

// Called only after a successful explicit restore, while the host serializes
// account operations. It forgets mappings locally; no Microsoft data is removed.
export function forgetDocumentMappings() {
  const keys = [];
  for (let index = 0; index < localStorage.length; index++) {
    const key = localStorage.key(index);
    if (key?.startsWith(PREFIX)) keys.push(key);
  }
  for (const key of keys) localStorage.removeItem(key);
}

export function identity() {
  const key = `${PREFIX}device`;
  let value = localStorage.getItem(key);
  if (!value) { value = crypto.randomUUID(); localStorage.setItem(key, value); }
  return value;
}

export function loadMetadata(account) {
  const bound = localStorage.getItem(BINDING);
  if (bound && bound !== account) throw new Error("These local tasks are linked to another Microsoft account. Reconnect that account to avoid uploading its tasks to a different person.");
  const key = `${PREFIX}${account}`;
  const saved = localStorage.getItem(key);
  const meta = saved ? JSON.parse(saved) : { version: 1, lists: {}, tasks: {}, pendingLists: {}, pendingDeletes: [] };
  if (meta.version !== 1 || !meta.lists || !meta.tasks) throw new Error("The Microsoft synchronization metadata is unreadable. Your tasks were not changed.");
  localStorage.setItem(BINDING, account);
  return { key, meta };
}

export function checkpoint(key, meta, original, initial = { lists: {}, tasks: {} }) {
  const lists = new Set([...original.lists.map(list => String(list.id)), ...Object.keys(initial.lists)]);
  const tasks = new Map(original.tasks.map(task => [String(task.id), task]));
  const knownTasks = new Set([...tasks.keys(), ...Object.keys(initial.tasks)]);
  const projections = [...(meta.pendingDeletes || []), ...Object.values(meta.pendingTasks || {}).map(pending => pending.move).filter(Boolean)];
  const durable = structuredClone(meta);
  durable.lists = Object.fromEntries(Object.entries(durable.lists).filter(([id]) => lists.has(id)));
  durable.tasks = Object.fromEntries(Object.entries(durable.tasks).filter(([id]) => knownTasks.has(id)).map(([id, task]) => {
    const local = tasks.get(id), previous = initial.tasks[id] || {};
    const originalSteps = new Map((local?.steps || []).map(step => [String(step.id), step]));
    const moves = projections.filter(plan => plan.localTaskId === Number(id));
    const steps = new Set([...originalSteps.keys(), ...Object.keys(previous.steps || {}), ...Object.keys(previous.pendingSteps || {}),
      ...moves.flatMap(plan => (plan.steps || []).map(step => String(step.id)))]);
    const files = new Map((local?.attachments || []).map(file => [String(file.id), file]));
    const knownFiles = new Set([...files.keys(), ...Object.keys(previous.attachments || {}), ...Object.keys(previous.pendingAttachments || {}),
      ...moves.flatMap(plan => (plan.extraAttachments || []).map(file => String(file.id)))]);
    task.steps = Object.fromEntries(Object.entries(task.steps || {}).filter(([step]) => steps.has(step)).map(([step, record]) => {
      const originalStep = originalSteps.get(step);
      if (originalStep) record.baseLocal = { title: originalStep.title, completed: !!originalStep.completed };
      return [step, record];
    }));
    task.attachments = Object.fromEntries(Object.entries(task.attachments || {}).filter(([file]) => knownFiles.has(file)).map(([file, record]) => {
      // Remote projections and cache-key migrations exist only in the draft
      // until its CAS commit. Preserve the actual document's local baseline.
      const originalFile = files.get(file);
      if (originalFile) {
        const { name, mime_type, size, storage_key } = originalFile;
        record.baseLocal = { name, mime_type, size, storage_key };
      }
      return [file, record];
    }));
    return [id, task];
  }));
  const serialized = JSON.stringify(durable);
  if (localStorage.getItem(key) !== serialized) localStorage.setItem(key, serialized);
}

function normalize(value) {
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === "object") return Object.fromEntries(Object.keys(value).sort().map(key => [key, normalize(value[key])]));
  return value;
}
const sameDocument = (a, b) => JSON.stringify(normalize(JSON.parse(a))) === JSON.stringify(normalize(JSON.parse(b)));

export function recoverCommit() {
  const raw = localStorage.getItem(JOURNAL);
  if (!raw) return;
  const journal = JSON.parse(raw);
  const current = localStorage.getItem(DOCUMENT) || "";
  if (current && sameDocument(current, journal.after)) localStorage.setItem(journal.key, journal.metadata);
  else if (current && !sameDocument(current, journal.before)) throw new Error("A previous sync was interrupted while another tab changed tasks. Export a backup before resolving the conflict.");
  localStorage.removeItem(JOURNAL);
}

export function commit(runtime, expected, draft, key, metadata) {
  // The Rust model fills serde defaults, repairs navigation order, and retains
  // delivered reminders. Journal its exact prospective bytes, not the JS draft.
  const after = runtime.prepareCloudData(JSON.stringify(draft));
  const serialized = JSON.stringify(metadata);
  if (sameDocument(expected, after)) {
    if (localStorage.getItem(key) !== serialized) localStorage.setItem(key, serialized);
    return false;
  }
  localStorage.setItem(JOURNAL, JSON.stringify({ key, before: expected, after, metadata: serialized }));
  try {
    runtime.replaceFromCloud(after, expected);
    localStorage.setItem(key, serialized);
    localStorage.removeItem(JOURNAL);
    return true;
  } catch (error) {
    // Recovery can finish a successful document write if only the sidecar write failed.
    recoverCommit();
    throw error;
  }
}

export function assertDocument(runtime, expected) {
  if (runtime.exportData() !== expected) throw new Error("Tasks changed on this device while syncing. Your edits are safe. Sync again to include them.");
  const saved = localStorage.getItem(DOCUMENT);
  if (saved && !sameDocument(saved, expected)) throw new Error("Another To Do tab changed your tasks. Export this tab's changes and reload before syncing.");
}

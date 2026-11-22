const PREFIX = "ferroui.todo.graph.v1.";
const BINDING = `${PREFIX}document-account`;
const JOURNAL = `${PREFIX}commit`;
const DOCUMENT = "ferroui.todo.v1";

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

export function checkpoint(key, meta, original) {
  const lists = new Set(original.lists.map(list => String(list.id)));
  const tasks = new Map(original.tasks.map(task => [String(task.id), new Set(task.steps.map(step => String(step.id)))]));
  const durable = structuredClone(meta);
  durable.lists = Object.fromEntries(Object.entries(durable.lists).filter(([id]) => lists.has(id)));
  durable.tasks = Object.fromEntries(Object.entries(durable.tasks).filter(([id]) => tasks.has(id)).map(([id, task]) => {
    task.steps = Object.fromEntries(Object.entries(task.steps || {}).filter(([step]) => tasks.get(id).has(step)));
    return [id, task];
  }));
  localStorage.setItem(key, JSON.stringify(durable));
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
  localStorage.setItem(JOURNAL, JSON.stringify({ key, before: expected, after, metadata: serialized }));
  try {
    runtime.replaceFromCloud(after, expected);
    localStorage.setItem(key, serialized);
    localStorage.removeItem(JOURNAL);
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

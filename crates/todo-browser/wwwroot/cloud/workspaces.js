// Documents and service sidecars have different lifetimes. An account switch
// checkpoints the document before changing its binding; Graph/Planner mappings
// already carry the account key and remain available when that account returns.
const PREFIX = "ferroui.todo.workspaces.v1.";
const ACTIVE = `${PREFIX}active`;
const JOURNAL = `${PREFIX}switch`;
const DOCUMENT = "ferroui.todo.v1";
const BINDING = "ferroui.todo.graph.v1.document-account";
const snapshotKey = account => `${PREFIX}document.${JSON.stringify(account)}`;

function identity(account) {
  if (account !== null && (typeof account !== "string" || !account || account.length > 2048))
    throw new Error("The workspace account identity is invalid.");
  return account;
}
function normalize(value) {
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === "object") return Object.fromEntries(Object.keys(value).sort().map(key => [key, normalize(value[key])]));
  return value;
}
const same = (left, right) => typeof left === "string" && typeof right === "string"
  && JSON.stringify(normalize(JSON.parse(left))) === JSON.stringify(normalize(JSON.parse(right)));

export function workspaceAccount(storage = localStorage) {
  const active = storage.getItem(ACTIVE);
  return active === null ? identity(storage.getItem(BINDING)) : identity(JSON.parse(active));
}

function bind(storage, account) {
  if (account === null) storage.removeItem(BINDING);
  else storage.setItem(BINDING, account);
  storage.setItem(ACTIVE, JSON.stringify(account));
}

// This only completes sidecars. The host's document store remains the sole
// writer of task data, including on desktop where it uses an atomic rename.
export function recoverWorkspaceSwitch(storage = localStorage) {
  const raw = storage.getItem(JOURNAL);
  if (raw === null) return;
  const entry = JSON.parse(raw);
  if (entry?.version !== 1 || typeof entry.before !== "string" || typeof entry.after !== "string")
    throw new Error("An interrupted account switch is unreadable. Export a backup before recovering this workspace.");
  identity(entry.from); identity(entry.to);
  const saved = storage.getItem(DOCUMENT);
  // Equal documents occur when the first account adopts the local workspace.
  // In that case the durable intent is sufficient to finish its new binding.
  if (same(saved, entry.after)) {
    storage.setItem(snapshotKey(entry.to), entry.after);
    bind(storage, entry.to);
  } else if (same(saved, entry.before) || (saved === null && entry.beforeMissing === true)) bind(storage, entry.from);
  else throw new Error("Tasks changed during an interrupted account switch. Export a backup and reload before switching accounts.");
  storage.removeItem(JOURNAL);
}

export function assertWorkspace(account, storage = localStorage) {
  if (storage.getItem(JOURNAL) !== null || workspaceAccount(storage) !== identity(account))
    throw new Error("The task workspace belongs to a different account. Switch accounts again before synchronizing.");
}

export function checkpointWorkspace(runtime, storage = localStorage) {
  recoverWorkspaceSwitch(storage);
  runtime.flushEdits?.();
  const json = runtime.exportData(), saved = storage.getItem(DOCUMENT);
  if (saved !== null && !same(saved, json)) throw new Error("Another tab changed tasks. Export this tab's edits and reload before switching accounts.");
  const account = workspaceAccount(storage);
  storage.setItem(snapshotKey(account), json);
  return { account, json };
}

export function activateWorkspace(runtime, account, { storage = localStorage, adoptLocal = false, afterCommit = () => {} } = {}) {
  identity(account);
  const firstWorkspace = storage.getItem(ACTIVE) === null && storage.getItem(BINDING) === null;
  const { account: from, json: before } = checkpointWorkspace(runtime, storage);
  if (from === account) { bind(storage, account); return false; }
  const saved = storage.getItem(snapshotKey(account));
  const candidate = saved ?? (account !== null && from === null && (adoptLocal || firstWorkspace)
    ? before : runtime.emptyAccountData());
  // Pure native validation precedes every durable intent and does not combine
  // reminder IDs or navigation IDs belonging to the previous account.
  const after = runtime.validateImportData(candidate);
  storage.setItem(JOURNAL, JSON.stringify({ version: 1, from, to: account, before, after, beforeMissing: storage.getItem(DOCUMENT) === null }));
  try {
    runtime.replaceAccountData(after, before);
    storage.setItem(snapshotKey(account), after);
    bind(storage, account);
    afterCommit(after);
    storage.removeItem(JOURNAL);
  } catch (error) {
    // Save failures leave the old document mounted. A sidecar failure after a
    // successful save can be finished safely; never switch the document back.
    try { recoverWorkspaceSwitch(storage); } catch { /* Keep the recovery intent. */ }
    throw error;
  }
  return true;
}

// Explicit import creates a local workspace. The caller checkpoints before
// import so other accounts keep their tasks and cloud histories intact.
export function detachWorkspace(runtime, { storage = localStorage, afterCommit = () => {} } = {}) {
  const json = runtime.exportData();
  storage.setItem(snapshotKey(null), json);
  bind(storage, null);
  afterCommit(json);
}

export function commitWorkspaceImport(runtime, json, apply, { storage = localStorage, beforeCommit = () => {}, afterCommit = () => {} } = {}) {
  const { account: from, json: before } = checkpointWorkspace(runtime, storage);
  const after = runtime.prepareImportData(json);
  storage.setItem(JOURNAL, JSON.stringify({ version: 1, from, to: null, before, after, beforeMissing: storage.getItem(DOCUMENT) === null }));
  try {
    beforeCommit();
    apply(after);
    storage.setItem(snapshotKey(null), after);
    bind(storage, null);
    afterCommit(after);
    storage.removeItem(JOURNAL);
  } catch (error) {
    try { recoverWorkspaceSwitch(storage); } catch { /* Keep the exact recovery intent. */ }
    throw error;
  }
}

export function workspaceFileKeys(storage = localStorage) {
  const keys = new Set();
  for (let index = 0; index < storage.length; index++) {
    const key = storage.key(index);
    if (!key?.startsWith(`${PREFIX}document.`)) continue;
    const document = JSON.parse(storage.getItem(key));
    if (!Array.isArray(document?.tasks)) throw new Error("An account workspace is unreadable. Attachment cleanup was paused.");
    for (const task of document.tasks) for (const file of task.attachments || [])
      if (typeof file.storage_key === "string") keys.add(file.storage_key);
  }
  return keys;
}

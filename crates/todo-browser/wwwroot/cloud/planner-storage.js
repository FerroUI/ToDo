import { loadMetadata } from "./metadata.js";
import { changedLocally, isPlanner } from "./planner-codec.js";

const PREFIX = "ferroui.todo.planner.v1.";

export function loadPlanner(accountId) {
  // Share the document/account binding with To Do, never upload another account's
  // cached assignments after selecting a different Microsoft identity.
  loadMetadata(accountId);
  const key = `${PREFIX}${accountId}`;
  const saved = localStorage.getItem(key);
  const meta = saved ? JSON.parse(saved) : { version: 1, tasks: {}, pendingSteps: {}, lastSync: 0 };
  if (meta.version !== 1 || !meta.tasks || typeof meta.tasks !== "object" || Array.isArray(meta.tasks)
    || !meta.pendingSteps || typeof meta.pendingSteps !== "object" || Array.isArray(meta.pendingSteps)) {
    throw new Error("The Planner synchronization history is unreadable. Your tasks were kept.");
  }
  return { key, meta };
}

export function shouldPoll(document, meta, force, now) {
  if (force || !meta.lastSync || !Number.isFinite(meta.lastSync) || now - meta.lastSync < 0 || now - meta.lastSync >= 60_000) return true;
  if (Object.keys(meta.pendingSteps).length) return true;
  const tasks = document.tasks.filter(isPlanner);
  return tasks.some(task => changedLocally(task, meta.tasks[task.external.id]))
    || Object.values(meta.tasks).some(record => !tasks.some(task => task.id === record.localId));
}

export function saveIntents(key, meta, pendingSteps) {
  localStorage.setItem(key, JSON.stringify({ ...meta, pendingSteps }));
}

export function forgetPlannerMappings(storage = localStorage) {
  const keys = Array.from({ length: storage.length }, (_, index) => storage.key(index));
  for (const key of keys) if (key?.startsWith(PREFIX)) storage.removeItem(key);
}

import { collection, graph } from "./client.js";
import { equal } from "./codec.js";
import { assertDocument, commit } from "./metadata.js";
import { adoptChecklist, planChecklist, recoverChecklist } from "./planner-checklist.js";
import { applyFields, changedLocally, importTask, isPlanner, localFields, mergeTask, plannerPath, remoteFields, requireEtag, taskPatch } from "./planner-codec.js";
import { loadPlanner, saveIntents, shouldPoll } from "./planner-storage.js";
import { checkpointDetails, checkpointTask } from "./planner-progress.js";

export { forgetPlannerMappings } from "./planner-storage.js";

export async function synchronizePlanner(runtime, accountId, today, { force = true } = {}) {
  const expected = runtime.exportData(), original = JSON.parse(expected);
  const { key, meta } = loadPlanner(accountId);
  if (!shouldPoll(original, meta, force, Date.now())) return { skipped: true };
  const draft = structuredClone(original), pending = structuredClone(meta.pendingSteps), records = {};
  const guard = () => assertDocument(runtime, expected), allocate = () => draft.next_id++;
  guard();
  const cloud = await readAssignments();
  const localByRemote = new Map();
  for (const task of draft.tasks.filter(isPlanner)) {
    if (!task.external.id || localByRemote.has(task.external.id)) throw new Error("Two local tasks refer to the same Planner assignment. Keep a local copy before synchronizing.");
    localByRemote.set(task.external.id, task);
  }
  const cloudIds = new Set(cloud.map(({ task }) => task.id));
  for (const local of localByRemote.values()) {
    if (cloudIds.has(local.external.id)) continue;
    const record = meta.tasks[local.external.id];
    if (changedLocally(local, record)) throw new Error(`“${local.title}” changed here and is no longer assigned to you in Planner. Keep a local copy before synchronizing.`);
    draft.tasks = draft.tasks.filter(task => task.id !== local.id);
    delete pending[local.external.id];
  }

  // Plan every merge before any remote mutation. Each task and its details use
  // independent revisions, and an absent revision never becomes an unconditional write.
  const plans = cloud.map(({ task, details }) => {
    let local = localByRemote.get(task.id), record = meta.tasks[task.id];
    if (!local) {
      local = importTask(allocate(), task, details, today);
      draft.tasks.push(local);
      record = { localId: local.id, baseLocal: localFields(local), baseRemote: remoteFields(task, details), steps: adoptChecklist(local, details, allocate) };
    } else if (!record || record.localId !== local.id) {
      if (!equal(localFields(local), remoteFields(task, details))) throw new Error(`Planner history for “${local.title}” is unavailable. Keep a local copy before reconnecting its task.`);
      record = { localId: local.id, baseLocal: localFields(local), baseRemote: remoteFields(task, details), steps: recoverChecklist(local, details) };
    }
    const before = localFields(local), originalSteps = structuredClone(local.steps);
    const fields = mergeTask(local, task, details, record);
    const taskUpdate = taskPatch(fields, task), detailUpdate = {};
    if (fields.notes !== (details.description || "")) detailUpdate.description = fields.notes;
    const intents = pending[task.id] || {};
    const checklist = planChecklist(local, details, record, intents, allocate);
    if (Object.keys(intents).length) pending[task.id] = intents;
    if (Object.keys(checklist.patch).length) detailUpdate.checklist = checklist.patch;
    if (Object.keys(taskUpdate).length) requireEtag(task["@odata.etag"], `“${local.title}”`);
    if (Object.keys(detailUpdate).length) requireEtag(details["@odata.etag"], `the details of “${local.title}”`);
    return { local, task, details, record, before, originalSteps, fields, taskUpdate, detailUpdate, checklist };
  });
  guard();
  saveIntents(key, meta, pending);
  const durable = structuredClone({ ...meta, pendingSteps: pending });
  const saveProgress = () => localStorage.setItem(key, JSON.stringify(durable));
  for (const plan of plans) {
    const { local, fields, taskUpdate, detailUpdate, checklist } = plan;
    let { task, details } = plan;
    try {
      if (Object.keys(taskUpdate).length) {
        guard(); await graph(plannerPath(task.id), "PATCH", taskUpdate, task["@odata.etag"]);
        checkpointTask(durable, plan); saveProgress();
      }
      if (Object.keys(detailUpdate).length) {
        guard(); await graph(`${plannerPath(task.id)}/details`, "PATCH", detailUpdate, details["@odata.etag"]);
        checkpointDetails(durable, plan); saveProgress();
      }
    } catch (error) { throw plannerError(error, local.title); }
    // Re-read mutations so later synchronization retains server-normalized values
    // and server-owned completion time, without broadening the fields we write.
    if (Object.keys(taskUpdate).length) task = await graph(plannerPath(task.id));
    if (Object.keys(detailUpdate).length) details = await graph(`${plannerPath(task.id)}/details`);
    applyFields(local, fields, task, details, today);
    local.steps = checklist.steps;
    records[task.id] = { localId: local.id, baseLocal: localFields(local), baseRemote: remoteFields(task, details), steps: checklist.mappings };
    delete pending[task.id];
  }
  guard();
  commit(runtime, expected, draft, key, { version: 1, tasks: records, pendingSteps: pending, lastSync: Date.now() });
  return { skipped: false, tasks: plans.length };
}

async function readAssignments() {
  let tasks;
  try { tasks = await collection("me/planner/tasks"); }
  catch (error) {
    if (![403, 404].includes(error.status)) throw error;
    const unavailable = new Error("Planner is unavailable for this account. Use a work or school account with access to Planner. Your local tasks were kept.");
    unavailable.status = error.status; unavailable.code = "planner_unavailable";
    throw unavailable;
  }
  const ids = new Set();
  for (const task of tasks) {
    if (typeof task.id !== "string" || !task.id || ids.has(task.id)) throw new Error("Planner returned an invalid or repeated task identity. Your tasks were kept.");
    ids.add(task.id);
  }
  const results = [];
  for (let index = 0; index < tasks.length; index += 4) {
    results.push(...await Promise.all(tasks.slice(index, index + 4).map(async task => ({ task, details: await graph(`${plannerPath(task.id)}/details`) }))));
  }
  return results;
}

function plannerError(error, title) {
  if ([409, 412].includes(error.status)) return Object.assign(new Error(`“${title}” changed in Planner during synchronization. Your local edits were kept; sync again to review the change.`), { status: error.status, code: "planner_conflict" });
  if ([400, 403].includes(error.status)) return Object.assign(new Error(`Planner could not apply changes to “${title}”. Project-derived tasks may only allow completion changes. Open Planner to review its permissions and supported fields. ${error.message}`), { status: error.status, code: "planner_restricted" });
  return error;
}

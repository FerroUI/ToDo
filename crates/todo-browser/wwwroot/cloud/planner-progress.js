import { stepFields } from "./planner-codec.js";

// Advance only fields whose PATCH was acknowledged. Unsaved remote imports and
// unrelated, not-yet-uploaded local edits retain their earlier merge baselines.
export function checkpointTask(meta, plan) {
  const record = meta.tasks[plan.task.id] ||= structuredClone(plan.record);
  const fields = { title: "title", percentComplete: "completed", dueDateTime: "due_date" };
  for (const remoteKey of Object.keys(plan.taskUpdate)) {
    const key = fields[remoteKey];
    record.baseLocal[key] = plan.before[key];
    record.baseRemote[key] = plan.fields[key];
  }
}

export function checkpointDetails(meta, plan) {
  const record = meta.tasks[plan.task.id] ||= structuredClone(plan.record);
  if (Object.hasOwn(plan.detailUpdate, "description")) {
    record.baseLocal.notes = plan.before.notes;
    record.baseRemote.notes = plan.fields.notes;
  }
  const patch = plan.detailUpdate.checklist || {};
  for (const [id, mapping] of Object.entries(record.steps)) {
    if (Object.hasOwn(patch, mapping.remoteId) && patch[mapping.remoteId] === null) delete record.steps[id];
  }
  for (const [id, mapping] of Object.entries(plan.checklist.mappings)) {
    if (!Object.hasOwn(patch, mapping.remoteId) || !patch[mapping.remoteId]) continue;
    const local = plan.originalSteps.find(step => step.id === Number(id));
    if (!local) continue;
    record.steps[id] = { ...mapping, baseLocal: stepFields(local) };
    if (meta.pendingSteps[plan.task.id]) delete meta.pendingSteps[plan.task.id][id];
  }
  if (meta.pendingSteps[plan.task.id] && !Object.keys(meta.pendingSteps[plan.task.id]).length) delete meta.pendingSteps[plan.task.id];
}

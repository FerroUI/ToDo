import { mergeFields } from "./codec.js";

const stepFields = step => ({ title: step.title, completed: !!step.completed });
const remoteStep = step => ({ title: step.displayName, completed: !!step.isChecked });

export function projectMoveSteps(task, remote, record, context) {
  const before = new Map(task.steps.map(step => [step.id, stepFields(step)]));
  const remaining = new Map((remote.checklistItems || []).map(step => [step.id, step]));
  for (const [id, saved] of Object.entries(record.steps || {})) {
    const local = task.steps.find(step => step.id === Number(id)), other = remaining.get(saved.remoteId);
    if (local && other) Object.assign(local, mergeFields(stepFields(local), remoteStep(other), saved.baseLocal, saved.baseRemote, local.title));
    else if (local) task.steps = task.steps.filter(step => step !== local);
    remaining.delete(saved.remoteId);
  }
  for (const other of remaining.values()) task.steps.push({ id: context.allocate(), ...remoteStep(other) });
  return task.steps.map(step => ({ id: step.id, fields: stepFields(step), base: before.get(step.id) || null }));
}

function restoreExtra(task, record, projection, fields, collection, pendingCollection, context) {
  const existing = task[collection].find(item => item.id === projection.id);
  if (existing && (collection === "steps" ? existing.title === fields.title && existing.completed === fields.completed : existing.storage_key === fields.storage_key)) return;
  // Draft identifiers were never reserved in the document: a user-created
  // entity may have reused one after the interrupted operation.
  const oldId = projection.id, local = { ...fields, id: context.allocate() };
  task[collection].push(local);
  for (const key of [collection, pendingCollection]) {
    if (!record[key]?.[oldId]) continue;
    record[key][local.id] = record[key][oldId]; delete record[key][oldId];
  }
  projection.id = local.id;
}

export function restoreMoveProjections(draft, metadata, context) {
  for (const plan of metadata.pendingDeletes) {
    const task = draft.tasks.find(task => task.id === plan.localTaskId), record = metadata.tasks[plan.localTaskId];
    if (!task || !record) continue;
    for (const projected of plan.steps || []) {
      const local = task.steps.find(step => step.id === projected.id);
      if (!projected.base) restoreExtra(task, record, projected, projected.fields, "steps", "pendingSteps", context);
      else if (local) Object.assign(local, mergeFields(stepFields(local), projected.fields, projected.base, projected.base, local.title));
    }
    for (const extra of plan.extraAttachments || []) {
      const saved = record.attachments?.[extra.id], pending = record.pendingAttachments?.[extra.id];
      const fields = saved ? { name: saved.baseRemote.name, mime_type: saved.baseRemote.contentType, size: saved.baseRemote.size,
        storage_key: saved.storageKey || saved.baseLocal.storage_key } : pending?.fields || extra;
      restoreExtra(task, record, extra, fields, "attachments", "pendingAttachments", context);
    }
  }
}

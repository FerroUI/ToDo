import { collection, graph, segment, taskPath } from "./client.js";
import { equal, mergeFields } from "./codec.js";

const localFields = item => ({ title: item.title, completed: !!item.completed });
const remoteFields = item => ({ title: item.displayName, completed: !!item.isChecked });

export function checkSteps(task, remote, mapping) {
  const remoteItems = new Map((remote.checklistItems || []).map(item => [item.id, item]));
  for (const [id, record] of Object.entries(mapping.steps || {})) {
    const local = task.steps.find(item => item.id === Number(id));
    const other = remoteItems.get(record.remoteId);
    if (!local && other && !equal(remoteFields(other), record.baseRemote)) throw new Error(`The step “${other.displayName}” was deleted here and changed in Microsoft.`);
    if (local && !other && !equal(localFields(local), record.baseLocal)) throw new Error(`The step “${local.title}” changed here and was deleted in Microsoft.`);
    if (local && other) mergeFields(localFields(local), remoteFields(other), record.baseLocal, record.baseRemote, local.title);
  }
}

export async function syncSteps(task, remote, mapping, context) {
  let changed = false;
  mapping.steps ||= {};
  mapping.pendingSteps ||= {};
  const path = `${taskPath(mapping.listId, mapping.remoteId)}/checklistItems`;
  const remoteItems = new Map((remote.checklistItems || []).map(item => [item.id, item]));
  for (const [id, pending] of Object.entries(mapping.pendingSteps)) {
    if (mapping.steps[id]) { delete mapping.pendingSteps[id]; continue; }
    const local = task.steps.find(step => step.id === Number(id));
    const found = [...remoteItems.values()].filter(item => !pending.before.includes(item.id) && equal(remoteFields(item), pending.fields));
    if (!found.length && local && local.title !== pending.fields.title) { delete mapping.pendingSteps[id]; context.save(); continue; }
    if (!local || found.length !== 1) throw new Error(`Uploading step “${pending.fields.title}” was interrupted. Check Microsoft; if it was not added, rename the step here and sync again.`);
    mapping.steps[id] = { remoteId: found[0].id, baseLocal: pending.fields, baseRemote: pending.fields };
    delete mapping.pendingSteps[id]; context.save();
  }
  for (const [id, record] of Object.entries(mapping.steps)) {
    const local = task.steps.find(item => item.id === Number(id));
    const other = remoteItems.get(record.remoteId);
    if (!local) {
      if (other) { context.guard(); await graph(`${path}/${segment(record.remoteId)}`, "DELETE", undefined, other["@odata.etag"]); changed = true; }
      delete mapping.steps[id];
    } else if (!other) {
      task.steps = task.steps.filter(item => item.id !== local.id);
      delete mapping.steps[id];
    } else {
      const localBefore = localFields(local);
      const merged = mergeFields(localBefore, remoteFields(other), record.baseLocal, record.baseRemote, local.title);
      const needsPatch = !equal(merged, remoteFields(other));
      if (needsPatch) {
        context.guard();
        await graph(`${path}/${segment(other.id)}`, "PATCH", { displayName: merged.title, isChecked: merged.completed }, other["@odata.etag"]);
        changed = true;
      }
      Object.assign(local, merged);
      const updated = !equal(record.baseLocal, localBefore) || !equal(record.baseRemote, merged);
      record.baseLocal = localBefore;
      record.baseRemote = merged;
      if (updated || needsPatch) context.save();
    }
    if (other) remoteItems.delete(other.id);
  }
  // Remote-only steps acquire stable local IDs without disturbing the existing step IDs.
  for (const other of remoteItems.values()) {
    const step = { id: context.allocate(), ...remoteFields(other) };
    task.steps.push(step);
    mapping.steps[step.id] = { remoteId: other.id, baseLocal: localFields(step), baseRemote: remoteFields(other) };
  }
  for (const local of task.steps) {
    if (mapping.steps[local.id]) continue;
    context.guard();
    mapping.pendingSteps[local.id] = { fields: localFields(local), before: (remote.checklistItems || []).map(item => item.id) };
    context.save();
    const other = await graph(path, "POST", { displayName: local.title, isChecked: local.completed });
    changed = true;
    mapping.steps[local.id] = { remoteId: other.id, baseLocal: localFields(local), baseRemote: remoteFields(other) };
    delete mapping.pendingSteps[local.id];
    context.save();
  }
  if (changed) remote.checklistItems = await collection(path);
  return changed;
}

export function adoptSteps(task, remote, mapping, allocate) {
  mapping.steps = {};
  task.steps = (remote.checklistItems || []).map(item => {
    const step = { id: allocate(), ...remoteFields(item) };
    mapping.steps[step.id] = { remoteId: item.id, baseLocal: localFields(step), baseRemote: remoteFields(item) };
    return step;
  });
}

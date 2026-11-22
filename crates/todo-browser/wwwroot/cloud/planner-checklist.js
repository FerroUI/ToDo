import { equal, mergeFields } from "./codec.js";
import { stepFields, remoteStepFields } from "./planner-codec.js";

export function remoteChecklist(details) {
  return Object.entries(details.checklist || {}).filter(([, step]) => step && typeof step === "object")
    .sort(([a, left], [b, right]) => compare(String(left.orderHint || ""), String(right.orderHint || "")) || compare(a, b));
}

const compare = (a, b) => a < b ? -1 : a > b ? 1 : 0;

export function adoptChecklist(task, details, allocate) {
  const mappings = {};
  task.steps = remoteChecklist(details).map(([remoteId, value]) => {
    const fields = remoteStepFields(value), id = allocate();
    mappings[id] = { remoteId, baseLocal: fields, baseRemote: fields };
    return { id, ...fields };
  });
  return mappings;
}

export function recoverChecklist(task, details) {
  const remote = remoteChecklist(details);
  if (!equal(task.steps.map(stepFields), remote.map(([, value]) => remoteStepFields(value)))) {
    throw new Error(`Planner checklist history for “${task.title}” is unavailable. Keep a local copy before reconnecting its task.`);
  }
  return Object.fromEntries(task.steps.map((step, index) => [step.id, {
    remoteId: remote[index][0], baseLocal: stepFields(step), baseRemote: remoteStepFields(remote[index][1]),
  }]));
}

/// Each new remote GUID is recorded before PATCH, so an uncertain response can
/// be retried without adding a second checklist item.
export function planChecklist(task, details, record, pending, allocate) {
  const patch = {}, mappings = structuredClone(record.steps || {}), steps = [], consumed = new Set();
  const remote = details.checklist || {};
  for (const [localId, intent] of Object.entries(pending)) {
    mappings[localId] ||= { remoteId: intent.remoteId, baseLocal: intent.fields, baseRemote: intent.fields, pending: true };
  }
  for (const [localId, mapping] of Object.entries(mappings)) {
    const local = task.steps.find(step => step.id === Number(localId));
    const value = Object.hasOwn(remote, mapping.remoteId) ? remote[mapping.remoteId] : null;
    if (!local && !value) { delete mappings[localId]; continue; }
    consumed.add(mapping.remoteId);
    if (!local) {
      if (!equal(remoteStepFields(value), mapping.baseRemote)) throw new Error(`Planner checklist conflict in “${task.title}”: a removed step changed online.`);
      patch[mapping.remoteId] = null; delete mappings[localId]; continue;
    }
    if (!value) {
      if (mapping.pending) {
        patch[mapping.remoteId] = writeStep(stepFields(local));
        pending[localId].fields = stepFields(local);
        steps.push({ ...local });
        mappings[localId] = { remoteId: mapping.remoteId, baseLocal: stepFields(local), baseRemote: stepFields(local) };
      } else {
        if (!equal(stepFields(local), mapping.baseLocal)) throw new Error(`Planner checklist conflict in “${task.title}”: an edited step was removed online.`);
        delete mappings[localId];
      }
      continue;
    }
    const merged = mergeFields(stepFields(local), remoteStepFields(value), mapping.baseLocal, mapping.baseRemote, `${task.title}: ${local.title}`);
    if (!equal(merged, remoteStepFields(value))) patch[mapping.remoteId] = writeStep(merged);
    if (mapping.pending) pending[localId].fields = merged;
    steps.push({ id: local.id, ...merged });
    mappings[localId] = { remoteId: mapping.remoteId, baseLocal: merged, baseRemote: merged };
  }
  for (const [remoteId, value] of remoteChecklist(details)) {
    if (consumed.has(remoteId)) continue;
    const id = allocate(), fields = remoteStepFields(value);
    steps.push({ id, ...fields }); mappings[id] = { remoteId, baseLocal: fields, baseRemote: fields };
  }
  for (const local of task.steps) {
    if (Object.hasOwn(record.steps || {}, local.id) || Object.hasOwn(pending, local.id)) continue;
    const remoteId = crypto.randomUUID(), fields = stepFields(local);
    pending[local.id] = { remoteId, fields };
    patch[remoteId] = writeStep(fields);
    steps.push({ ...local }); mappings[local.id] = { remoteId, baseLocal: fields, baseRemote: fields };
  }
  // Planner owns checklist order. Keep its ordinal order hints and append newly
  // created local steps until the server supplies their own order hints.
  const positions = new Map(remoteChecklist(details).map(([remoteId], index) => [remoteId, index]));
  steps.sort((a, b) => (positions.get(mappings[a.id]?.remoteId) ?? Infinity) - (positions.get(mappings[b.id]?.remoteId) ?? Infinity));
  return { patch, mappings, steps };
}

function writeStep(value) {
  return { "@odata.type": "#microsoft.graph.plannerChecklistItem", title: value.title, isChecked: value.completed };
}

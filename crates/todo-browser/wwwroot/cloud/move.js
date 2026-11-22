import { graph, graphBytes, hydrateTask, taskPath } from "./client.js";
import { readAttachment, storeAttachment } from "../attachments.js";
import { equal, remoteSnapshot } from "./codec.js";
import { adoptAttachments } from "./attachments.js";
import { attachmentPath, cloudAttachmentKey, fileDigest } from "./attachment-content.js";
import { projectMoveSteps } from "./move-projection.js";

const conflict = () => new Error("The original task changed in Microsoft during its move. Both copies were retained; review the original before syncing again.");
const canonical = value => Array.isArray(value) ? value.map(canonical)
  : value && typeof value === "object" ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])])) : value;
const moveSnapshot = task => ({ ...remoteSnapshot(task), writable: canonical(Object.fromEntries(
  ["body", "categories", "completedDateTime", "dueDateTime", "importance", "isReminderOn", "recurrence", "reminderDateTime", "startDateTime", "status", "title"]
    .filter(key => task[key] !== undefined).map(key => [key, task[key]]))) });

export async function prepareMove(task, remote, record, context, previous) {
  if (previous && remote && (!equal(moveSnapshot(remote), previous.snapshot) || (previous.etag && remote["@odata.etag"] && previous.etag !== remote["@odata.etag"]))) throw conflict();
  const source = remote || previous?.source;
  if (!source) throw new Error("The original Microsoft task is no longer available for the move.");
  const plan = previous || { path: taskPath(record.listId, record.remoteId), etag: source["@odata.etag"],
    localTaskId: task.id, sourceListId: record.listId, sourceTaskId: record.remoteId, snapshot: moveSnapshot(source), source: structuredClone(source), files: [] };
  plan.localTaskId = task.id;
  plan.steps = projectMoveSteps(task, source, record, context);
  const originalIds = new Set(task.attachments.map(file => file.id));
  const projected = structuredClone(record);
  const sourceFiles = new Map((source.attachments || []).map(file => [file.id, file]));
  for (const [id, mapped] of Object.entries(projected.attachments || {})) {
    const local = task.attachments.find(file => file.id === Number(id)), other = sourceFiles.get(mapped.remoteId);
    if (local && !other) task.attachments = task.attachments.filter(file => file.id !== local.id);
  }
  // Clone the mapping so identifiers allocated for imported files do not enter
  // the durable original mapping before the destination task has been created.
  adoptAttachments(task, source, projected, context);
  const digests = [];
  for (const file of source.attachments || []) {
    context.guard();
    const key = cloudAttachmentKey(context.accountId, record.listId, record.remoteId, file);
    const bytes = remote ? await graphBytes(`${attachmentPath(record.listId, record.remoteId, file.id)}/$value`) : await readAttachment(key);
    if (!bytes) throw new Error(`The original file “${file.name}” is required to recover this move.`);
    const digest = await fileDigest(bytes);
    if (previous && previous.files.find(item => item.id === file.id)?.digest !== digest) throw conflict();
    digests.push({ id: file.id, digest });
    await storeAttachment(key, new File([bytes], file.name || "attachment", { type: file.contentType || bytes.type || "application/octet-stream" }));
    const entry = Object.entries(projected.attachments || {}).find(([, value]) => value.remoteId === file.id);
    const local = entry && task.attachments.find(item => item.id === Number(entry[0]));
    if (local) Object.assign(local, { name: file.name || "attachment", mime_type: file.contentType || "application/octet-stream", size: file.size, storage_key: key });
  }
  plan.files = digests;
  plan.extraAttachments = structuredClone(task.attachments.filter(file => !originalIds.has(file.id)));
  return plan;
}

export async function deleteMovedSource(plan, context) {
  context.guard();
  if (!plan.sourceListId) return graph(plan.path, "DELETE", undefined, plan.etag);
  let current;
  try { current = await hydrateTask(plan.sourceListId, plan.sourceTaskId); }
  catch (error) { if (error.status === 404) return; throw error; }
  if (!equal(moveSnapshot(current), plan.snapshot) || (plan.etag && current["@odata.etag"] && plan.etag !== current["@odata.etag"])) throw conflict();
  for (const file of plan.files || []) {
    const bytes = await graphBytes(`${attachmentPath(plan.sourceListId, plan.sourceTaskId, file.id)}/$value`);
    if (await fileDigest(bytes) !== file.digest) throw conflict();
  }
  // Refresh the entity tag after copying/checking the files, before deleting the
  // original. A late change is rejected by the snapshot or conditional request.
  current = await hydrateTask(plan.sourceListId, plan.sourceTaskId);
  if (!equal(moveSnapshot(current), plan.snapshot) || (plan.etag && current["@odata.etag"] && plan.etag !== current["@odata.etag"])) throw conflict();
  context.guard();
  return graph(plan.path, "DELETE", undefined, current["@odata.etag"] || plan.etag);
}

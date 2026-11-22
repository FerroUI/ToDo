import { collection, graph, graphBytes, segment, taskPath } from "./client.js";
import { readAttachment, storeAttachment } from "../attachments.js";
import { equal } from "./codec.js";
import { attachmentPath, cloudAttachmentKey, fileDigest, materializeCloudAttachment, uploadAttachment } from "./attachment-content.js";

const localFields = file => ({ name: file.name, mime_type: file.mime_type, size: file.size, storage_key: file.storage_key });
export const attachmentFields = file => ({ name: file.name, contentType: file.contentType || "application/octet-stream", size: file.size,
  lastModifiedDateTime: file.lastModifiedDateTime || "", etag: file["@odata.etag"] || "" });
const mapping = (local, remote) => ({ remoteId: remote.id, baseLocal: localFields(local), baseRemote: attachmentFields(remote), storageKey: local.storage_key });

export function checkAttachments(task, remote, record) {
  const files = new Map((remote.attachments || []).map(file => [file.id, file]));
  for (const [id, saved] of Object.entries(record.attachments || {})) {
    const local = task.attachments.find(file => file.id === Number(id)), other = files.get(saved.remoteId);
    if (!local && other && !equal(attachmentFields(other), saved.baseRemote)) throw new Error(`File “${other.name}” was deleted here and changed in Microsoft.`);
    if (local && !other && !equal(localFields(local), saved.baseLocal)) throw new Error(`File “${local.name}” changed here and was deleted in Microsoft.`);
    if (local && other && !equal(localFields(local), saved.baseLocal)) throw new Error(`File “${local.name}” changed locally. Remove it and attach the updated file before syncing.`);
  }
}

export function adoptAttachments(task, remote, record, context) {
  record.attachments ||= {};
  for (const file of remote.attachments || []) {
    if (Object.values(record.attachments).some(saved => saved.remoteId === file.id)) continue;
    const local = { id: context.allocate(), name: file.name || "attachment", mime_type: file.contentType || "application/octet-stream",
      size: file.size, storage_key: cloudAttachmentKey(context.accountId, record.listId, record.remoteId, file) };
    task.attachments.push(local); record.attachments[local.id] = mapping(local, file);
  }
}

async function recoverUpload(files, pending, record) {
  const candidates = files.filter(file => !pending.before.includes(file.id) && file.name === pending.fields.name && file.size === pending.fields.size);
  const matching = [];
  for (const file of candidates) {
    const bytes = await graphBytes(`${attachmentPath(record.listId, record.remoteId, file.id)}/$value`);
    if (await fileDigest(bytes) === pending.digest) matching.push(file);
  }
  if (matching.length > 1) throw new Error(`More than one matching upload of “${pending.fields.name}” exists in Microsoft. Resolve the duplicates before syncing.`);
  return matching[0];
}

async function adoptUpload(local, uploaded, record, context, pending) {
  if (!uploaded?.id) throw new Error("Microsoft did not return an attachment identifier.");
  const bytes = await graphBytes(`${attachmentPath(record.listId, record.remoteId, uploaded.id)}/$value`);
  if (await fileDigest(bytes) !== pending.digest) throw new Error(`Microsoft did not confirm the contents of “${local.name}”. The original file was retained.`);
  if (local.storage_key.startsWith("graph:")) {
    const key = cloudAttachmentKey(context.accountId, record.listId, record.remoteId, uploaded);
    await storeAttachment(key, new File([bytes], local.name, { type: local.mime_type }));
    local.storage_key = key;
  }
  return mapping(local, uploaded);
}

export async function syncAttachments(task, remote, record, context) {
  record.attachments ||= {};
  record.pendingAttachments ||= {};
  const path = `${taskPath(record.listId, record.remoteId)}/attachments`;
  let files = remote.attachments || [], changed = false;
  for (const [id, pending] of Object.entries(record.pendingAttachments)) {
    if (record.attachments[id]) { delete record.pendingAttachments[id]; continue; }
    const local = task.attachments.find(file => file.id === Number(id));
    const other = await recoverUpload(files, pending, record);
    if (other) {
      // Keep the mapping when the local file was deleted during the upload; the
      // ordinary removal path below can then delete precisely the uploaded file.
      record.attachments[id] = local ? await adoptUpload(local, other, record, context, pending)
        : { remoteId: other.id, baseLocal: pending.fields, baseRemote: attachmentFields(other) };
      delete record.pendingAttachments[id]; context.save(); continue;
    }
    if (!local || !equal(localFields(local), pending.fields)) throw new Error(`Uploading “${pending.fields.name}” was interrupted and its local reference changed. Check Microsoft before retrying.`);
    if (!pending.session) throw new Error(`Uploading “${local.name}” was interrupted. Check Microsoft; if it was not added, remove and attach the file again.`);
    const blob = await readAttachment(local.storage_key);
    if (!blob || await fileDigest(blob) !== pending.digest) throw new Error(`The original file “${local.name}” is required to resume its upload.`);
    context.guard();
    let uploaded = await uploadAttachment(path, local, blob, pending, context);
    files = await collection(path);
    uploaded ||= await recoverUpload(files, pending, record);
    if (!uploaded) throw new Error(`Microsoft has not confirmed the upload of “${local.name}”. Sync again to recover it.`);
    record.attachments[id] = await adoptUpload(local, uploaded, record, context, pending); delete record.pendingAttachments[id]; changed = true; context.save();
  }
  checkAttachments(task, { attachments: files }, record);
  const remoteById = new Map(files.map(file => [file.id, file]));
  for (const [id, saved] of Object.entries(record.attachments)) {
    const local = task.attachments.find(file => file.id === Number(id)), other = remoteById.get(saved.remoteId);
    if (!local) {
      if (other) { context.guard(); await graph(`${path}/${segment(other.id)}`, "DELETE", undefined, other["@odata.etag"]); changed = true; }
      delete record.attachments[id]; context.save();
    } else if (!other) {
      task.attachments = task.attachments.filter(file => file.id !== local.id); delete record.attachments[id];
    } else {
      local.name = other.name || "attachment"; local.mime_type = other.contentType || "application/octet-stream"; local.size = other.size;
      local.storage_key = !equal(attachmentFields(other), saved.baseRemote)
        ? cloudAttachmentKey(context.accountId, record.listId, record.remoteId, other) : saved.storageKey || saved.baseLocal.storage_key || local.storage_key;
      record.attachments[id] = mapping(local, other);
    }
    if (other) remoteById.delete(other.id);
  }
  adoptAttachments(task, { attachments: [...remoteById.values()] }, record, context);
  for (const local of task.attachments) {
    if (record.attachments[local.id]) continue;
    const file = local.storage_key.startsWith("graph:") ? await materializeCloudAttachment(local.storage_key, context.accountId) : await readAttachment(local.storage_key);
    if (!file) throw new Error(`Attach “${local.name}” again on this device before uploading it to Microsoft.`);
    context.guard();
    const pending = { fields: localFields(local), before: files.map(file => file.id), digest: await fileDigest(file) };
    record.pendingAttachments[local.id] = pending; context.save();
    let uploaded;
    try { uploaded = await uploadAttachment(path, local, file, pending, context); }
    catch (error) {
      // These responses establish that creation was rejected; an unknown network
      // outcome stays checkpointed and is reconciled by digest on the next sync.
      if (!pending.session && [400, 401, 403, 404, 413, 429].includes(error.status)) { delete record.pendingAttachments[local.id]; context.save(); }
      throw error;
    }
    files = await collection(path);
    uploaded ||= await recoverUpload(files, pending, record);
    if (!uploaded) throw new Error(`Microsoft has not confirmed the upload of “${local.name}”. Sync again to recover it.`);
    record.attachments[local.id] = await adoptUpload(local, uploaded, record, context, pending); delete record.pendingAttachments[local.id]; changed = true; context.save();
  }
  remote.attachments = changed ? await collection(path) : files;
  return changed;
}

import { graph, graphBytes, graphUpload, graphUrl, segment, taskPath } from "./client.js";
import { readAttachment, storeAttachment, downloadBlob } from "../attachments.js";

export const MAX_ATTACHMENT = 25 * 1024 * 1024;
const CHUNK = 2 * 1024 * 1024;
export const attachmentPath = (list, task, id) => `${taskPath(list, task)}/attachments/${segment(id)}`;
export const cloudAttachmentKey = (account, list, task, file) => `graph:${[account, list, task, file.id, file.lastModifiedDateTime || ""].map(segment).join(":")}`;

export function parseCloudAttachmentKey(key) {
  if (!key.startsWith("graph:")) return null;
  const parts = key.slice(6).split(":");
  if (parts.length !== 5) throw new Error("The Microsoft file reference is invalid.");
  try {
    const [account, list, task, id, modified] = parts.map(decodeURIComponent);
    if (![account, list, task, id].every(Boolean)) throw new Error();
    return { account, list, task, id, modified };
  } catch { throw new Error("The Microsoft file reference is invalid."); }
}

export async function materializeCloudAttachment(key, accountId) {
  const remote = parseCloudAttachmentKey(key);
  if (!remote) return readAttachment(key);
  // Cached bytes are still private to the account that supplied them.
  if (!accountId || remote.account !== accountId) throw new Error("Connect the Microsoft account that owns this file to open it.");
  const cached = await readAttachment(key);
  if (cached) return cached;
  const path = attachmentPath(remote.list, remote.task, remote.id);
  const metadata = await graph(path);
  const blob = await graphBytes(`${path}/$value`);
  if (Number.isSafeInteger(metadata.size) && metadata.size !== blob.size) throw new Error("Microsoft returned an incomplete attachment. Try opening it again.");
  const file = new File([blob], metadata.name || "attachment", { type: metadata.contentType || blob.type || "application/octet-stream" });
  await storeAttachment(key, file);
  return file;
}

export async function openCloudAttachment(key, accountId) {
  const file = await materializeCloudAttachment(key, accountId);
  if (!file) throw new Error("The attachment is not available on this device.");
  downloadBlob(file, file.name || "attachment");
}

export async function fileDigest(file) {
  return [...new Uint8Array(await crypto.subtle.digest("SHA-256", await file.arrayBuffer()))].map(value => value.toString(16).padStart(2, "0")).join("");
}

function base64(bytes) {
  let text = "";
  for (let index = 0; index < bytes.length; index += 16_384) text += String.fromCharCode(...bytes.subarray(index, index + 16_384));
  return btoa(text);
}

function validateUploadLocation(url, path, kind) {
  const result = graphUrl(url);
  const expected = graphUrl(path).pathname.match(/\/todo\/lists\/([^/]+)\/tasks\/([^/]+)\/attachments$/i);
  const returned = result.pathname.match(new RegExp(`/todo/lists/([^/]+)/tasks/([^/]+)/${kind}/[^/]+$`, "i"));
  if (!expected || !returned || expected[1] !== returned[1] || expected[2] !== returned[2] || result.search) throw new Error("Microsoft returned an unexpected file upload location.");
  return result;
}

const sessionState = (value, previous = {}) => ({
  uploadUrl: value.uploadUrl || previous.uploadUrl,
  expirationDateTime: value.expirationDateTime || value.ExpirationDateTime || previous.expirationDateTime,
  nextExpectedRanges: value.nextExpectedRanges || value.NextExpectedRanges,
});

export async function uploadAttachment(path, local, file, pending, context) {
  if (!Number.isSafeInteger(file.size) || file.size < 0 || file.size > MAX_ATTACHMENT || file.size !== local.size) throw new Error("The attachment's stored size is invalid.");
  if (file.size < 3 * 1024 * 1024) {
    context.guard();
    return graph(path, "POST", { "@odata.type": "#microsoft.graph.taskFileAttachment", name: local.name,
      contentType: local.mime_type, size: file.size, contentBytes: base64(new Uint8Array(await file.arrayBuffer())) });
  }
  if (pending.session) {
    const url = validateUploadLocation(pending.session.uploadUrl, path, "attachmentSessions");
    // Query the accepted range before replaying any bytes after an interruption.
    try {
      const status = await graph(`${url.href}?$select=expirationDateTime,nextExpectedRanges`);
      pending.session = sessionState(status, pending.session); context.save();
    } catch (error) {
      if (error.status !== 404 || pending.finalizing) throw error;
      // An expired, incomplete session cannot have created an attachment.
      delete pending.session; context.save();
    }
  }
  if (pending.session && !pending.finalizing && Date.parse(pending.session.expirationDateTime) <= Date.now()) {
    context.guard(); await graph(pending.session.uploadUrl, "DELETE");
    delete pending.session; context.save();
  }
  if (!pending.session) {
    context.guard();
    pending.session = sessionState(await graph(`${path}/createUploadSession`, "POST", { attachmentInfo: {
      attachmentType: "file", name: local.name, size: file.size, contentType: local.mime_type,
    } }));
    validateUploadLocation(pending.session.uploadUrl, path, "attachmentSessions");
    context.save();
  }
  const session = pending.session;
  let offset = nextOffset(session, file.size);
  while (offset < file.size) {
    if (Date.parse(session.expirationDateTime) <= Date.now()) throw new Error("The Microsoft file upload expired. Retry synchronization to recover the uploaded file.");
    context.guard();
    const chunk = file.slice(offset, Math.min(offset + CHUNK, file.size));
    if (offset + chunk.size === file.size) { pending.finalizing = true; context.save(); }
    const response = await graphUpload(session.uploadUrl, chunk, offset, file.size);
    if (response.status === 201) {
      if (response.location) return graph(validateUploadLocation(response.location, path, "attachments").href);
      if (response.data?.id) return response.data;
      return null; // The caller identifies it from the attachment collection and content digest.
    }
    Object.assign(session, sessionState(response.data || {}, session));
    const next = nextOffset(session, file.size);
    if (next <= offset || next > offset + chunk.size) throw new Error("Microsoft returned an invalid file upload position.");
    offset = next; context.save();
  }
  return null;
}

function nextOffset(session, total) {
  const ranges = session.nextExpectedRanges || session.NextExpectedRanges;
  if (!Array.isArray(ranges) || ranges.length !== 1 || !/^\d+(?:-\d*)?$/.test(ranges[0])) throw new Error("Microsoft returned an invalid file upload range.");
  const offset = Number(ranges[0].split("-")[0]);
  if (!Number.isSafeInteger(offset) || offset < 0 || offset > total) throw new Error("Microsoft returned an invalid file upload position.");
  return offset;
}

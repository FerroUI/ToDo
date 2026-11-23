import { createHash, randomUUID } from "node:crypto";
import { unlinkSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { atomicWrite, readBounded } from "./storage.mjs";
import { MAX_ATTACHMENT_BYTES, RETENTION_MS, attachmentDirectory, attachmentPaths, readRecord, dataName, dataPath, attachmentGroups } from "./file-records.mjs";

function removeFile(path) {
  try { unlinkSync(path); } catch (error) { if (error.code !== "ENOENT") throw error; }
}

export async function storeAttachment(key, file) {
  if (!(file instanceof Blob) || file.size > MAX_ATTACHMENT_BYTES) throw new Error("Attachments must be 25 MiB or smaller.");
  const path = attachmentPaths(key);
  const bytes = Buffer.from(await file.arrayBuffer());
  if (bytes.length !== file.size || bytes.length > MAX_ATTACHMENT_BYTES) throw new Error("The attachment changed while it was being read.");
  const record = { version: 2, key, name: file.name || "attachment", type: file.type || "application/octet-stream", size: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex") };
  const serialized = JSON.stringify(record);
  if (Buffer.byteLength(serialized) > 64 * 1024) throw new Error("The attachment metadata exceeds its size limit.");
  // No await separates this read from publication, so overlapping Blob reads
  // observe the last committed generation before replacing the same key.
  const previous = readRecord(path, key);
  const payload = dataPath(path, record), previousPayload = previous && dataPath(path, previous);
  // Content generations are immutable. Publishing the metadata is the only
  // commit: cancellation or a failed rename cannot overwrite a good old file.
  // The native directory lock and serialized worker own this storage profile.
  atomicWrite(payload, bytes);
  try { atomicWrite(path.metadata, serialized); }
  catch (error) {
    if (payload !== previousPayload) { try { removeFile(payload); } catch { /* Startup cleanup reclaims an interrupted generation. */ } }
    throw error;
  }
  if (previousPayload && previousPayload !== payload) {
    try { removeFile(previousPayload); } catch { /* A committed replacement remains usable; cleanup retries later. */ }
  }
}

export async function saveAttachment(file) {
  const key = randomUUID(); await storeAttachment(key, file); return key;
}

export async function readAttachment(key) {
  const path = attachmentPaths(key), metadata = readRecord(path, key);
  if (!metadata) return null;
  const bytes = readBounded(dataPath(path, metadata), MAX_ATTACHMENT_BYTES);
  if (bytes.length !== metadata.size) throw new Error("The stored attachment is incomplete.");
  if (metadata.version === 2 && createHash("sha256").update(bytes).digest("hex") !== metadata.sha256) throw new Error("The stored attachment failed its integrity check.");
  if (metadata.deletedAt) { delete metadata.deletedAt; atomicWrite(path.metadata, JSON.stringify(metadata)); }
  return new File([bytes], metadata.name, { type: metadata.type });
}

export async function deleteAttachment(key) {
  const path = attachmentPaths(key), metadata = readRecord(path, key);
  if (!metadata) return;
  metadata.deletedAt = Date.now(); atomicWrite(path.metadata, JSON.stringify(metadata));
}

export async function purgeAttachment(key) {
  const path = attachmentPaths(key);
  const group = attachmentGroups(readdirSync(path.directory)).get(path.stem);
  removeFile(path.metadata);
  for (const name of [...(group?.data || []), ...(group?.temporary || [])]) removeFile(join(path.directory, name));
}

export function cleanTrash(document, synchronizationKeys = []) {
  const retained = new Set([...synchronizationKeys, ...document.tasks.flatMap(task => task.attachments.map(file => file.storage_key))]);
  const directory = attachmentDirectory(), now = Date.now();
  for (const [stem, group] of attachmentGroups(readdirSync(directory))) {
    let current;
    if (group.metadata) {
      const path = { directory, stem, metadata: join(directory, group.metadata) };
      const metadata = readRecord(path);
      const referenced = retained.has(metadata.key);
      if (!referenced && metadata.deletedAt && now - metadata.deletedAt > RETENTION_MS) {
        removeFile(path.metadata);
        for (const name of [...group.data, ...group.temporary]) removeFile(join(directory, name));
        continue;
      }
      if (referenced && metadata.deletedAt) {
        delete metadata.deletedAt; atomicWrite(path.metadata, JSON.stringify(metadata));
      } else if (!referenced && !metadata.deletedAt) {
        // UI deletion keeps bytes for undo without a destructive host request.
        // A subsequent launch reconciles them against the persisted document.
        metadata.deletedAt = now; atomicWrite(path.metadata, JSON.stringify(metadata));
      }
      current = dataName(path, metadata);
    }
    for (const name of [...group.data, ...group.temporary]) {
      const path = join(directory, name);
      if (name !== current && now - statSync(path).mtimeMs > RETENTION_MS) removeFile(path);
    }
  }
}

export function downloadBlob() { throw new Error("Use the native file picker to save this attachment."); }
export function chooseFile() { throw new Error("Use the native file picker to choose a file."); }

import { randomUUID } from "node:crypto";
import { unlinkSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { dataDirectory, privateDirectory, keyName, atomicWrite, readBounded } from "./storage.mjs";

const MAX = 25 * 1024 * 1024;
function paths(key) {
  if (typeof key !== "string" || !key || key.length > 8192) throw new Error("Invalid attachment storage key.");
  const directory = privateDirectory(join(dataDirectory(), "files"));
  const stem = join(directory, keyName(key));
  return { data: `${stem}.bin`, metadata: `${stem}.json` };
}

export async function storeAttachment(key, file) {
  if (!(file instanceof Blob) || file.size > MAX) throw new Error("Attachments must be 25 MiB or smaller.");
  const path = paths(key);
  atomicWrite(path.data, Buffer.from(await file.arrayBuffer()));
  atomicWrite(path.metadata, JSON.stringify({ key, name: file.name || "attachment", type: file.type || "application/octet-stream", size: file.size }));
}

export async function saveAttachment(file) {
  const key = randomUUID(); await storeAttachment(key, file); return key;
}

export async function readAttachment(key) {
  const path = paths(key);
  let metadata;
  try { metadata = JSON.parse(readBounded(path.metadata, 64 * 1024)); }
  catch (error) { if (error.code === "ENOENT") return null; throw error; }
  if (metadata.key !== key || !Number.isSafeInteger(metadata.size) || metadata.size < 0 || metadata.size > MAX || typeof metadata.name !== "string" || typeof metadata.type !== "string") throw new Error("Stored attachment metadata is invalid.");
  const bytes = readBounded(path.data, MAX);
  if (bytes.length !== metadata.size) throw new Error("The stored attachment is incomplete.");
  if (metadata.deletedAt) { delete metadata.deletedAt; atomicWrite(path.metadata, JSON.stringify(metadata)); }
  return new File([bytes], metadata.name, { type: metadata.type });
}

export async function deleteAttachment(key) {
  const path = paths(key);
  try {
    const metadata = JSON.parse(readBounded(path.metadata, 64 * 1024));
    metadata.deletedAt = Date.now(); atomicWrite(path.metadata, JSON.stringify(metadata));
  } catch (error) { if (error.code !== "ENOENT") throw error; }
}

export async function purgeAttachment(key) {
  for (const path of Object.values(paths(key))) {
    try { unlinkSync(path); } catch (error) { if (error.code !== "ENOENT") throw error; }
  }
}

export function cleanTrash(document) {
  const retained = new Set(document.tasks.flatMap(task => task.attachments.map(file => file.storage_key)));
  const directory = privateDirectory(join(dataDirectory(), "files"));
  for (const name of readdirSync(directory)) {
    if (!/^[a-f0-9]{64}\.json$/.test(name)) continue;
    const metadata = JSON.parse(readBounded(join(directory, name), 64 * 1024));
    if (metadata.deletedAt && !retained.has(metadata.key) && Date.now() - metadata.deletedAt > 30 * 24 * 60 * 60 * 1000) {
      // The function is synchronous except for its declared Promise; filesystem
      // removal completes before return, with rejection surfaced by the caller.
      return purgeAttachment(metadata.key);
    }
  }
}

export function downloadBlob() { throw new Error("Use the native file picker to save this attachment."); }
export function chooseFile() { throw new Error("Use the native file picker to choose a file."); }

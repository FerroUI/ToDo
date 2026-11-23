import { join } from "node:path";
import { dataDirectory, privateDirectory, keyName, readBounded } from "./storage.mjs";

export const MAX_ATTACHMENT_BYTES = 25 * 1024 * 1024;
export const RETENTION_MS = 30 * 24 * 60 * 60 * 1000;
export const DIGEST = /^[a-f0-9]{64}$/;

export const attachmentDirectory = () => privateDirectory(join(dataDirectory(), "files"));

export function attachmentPaths(key) {
  if (typeof key !== "string" || !key || key.length > 8192) throw new Error("Invalid attachment storage key.");
  const directory = attachmentDirectory(), stem = keyName(key);
  return { directory, stem, metadata: join(directory, `${stem}.json`) };
}

export function readRecord(path, key) {
  let record;
  try { record = JSON.parse(readBounded(path.metadata, 64 * 1024)); }
  catch (error) { if (error.code === "ENOENT") return null; throw error; }
  if (!record || typeof record.key !== "string" || !record.key || record.key.length > 8192 || keyName(record.key) !== path.stem
    || (key !== undefined && record.key !== key) || !Number.isSafeInteger(record.size) || record.size < 0 || record.size > MAX_ATTACHMENT_BYTES
    || typeof record.name !== "string" || typeof record.type !== "string"
    || (record.deletedAt !== undefined && (!Number.isSafeInteger(record.deletedAt) || record.deletedAt <= 0))
    || (record.version !== undefined && record.version !== 2)
    || (record.version === 2 && (typeof record.sha256 !== "string" || !DIGEST.test(record.sha256)))
    || (record.version === undefined && record.sha256 !== undefined)) throw new Error("Stored attachment metadata is invalid.");
  return record;
}

export function dataName(path, record) {
  return `${path.stem}${record.version === 2 ? `.${record.sha256}` : ""}.bin`;
}

export function dataPath(path, record) { return join(path.directory, dataName(path, record)); }

// Group once for linear cleanup even after many interrupted writes or removals.
// Every accepted filename is derived from hashes; metadata never supplies a path.
export function attachmentGroups(names) {
  const groups = new Map();
  for (const name of names) {
    const match = /^([a-f0-9]{64})(?:\.([a-f0-9]{64}))?\.(bin|json)(?:\.([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})\.tmp)?$/.exec(name);
    if (!match || (match[3] === "json" && match[2])) continue;
    const group = groups.get(match[1]) || { metadata: null, data: [], temporary: [] };
    if (match[4]) group.temporary.push(name);
    else if (match[3] === "json") group.metadata = name;
    else group.data.push(name);
    groups.set(match[1], group);
  }
  return groups;
}

import { createHash, randomUUID } from "node:crypto";
import { mkdirSync, readFileSync, readSync, writeFileSync, renameSync, unlinkSync, readdirSync, fstatSync, openSync, closeSync, fsyncSync, chmodSync } from "node:fs";
import { join } from "node:path";

let directory;
export const dataDirectory = () => directory;
export const keyName = key => createHash("sha256").update(String(key)).digest("hex");

export function privateDirectory(path) {
  mkdirSync(path, { recursive: true, mode: 0o700 });
  if (process.platform !== "win32") chmodSync(path, 0o700);
  return path;
}

export function atomicWrite(path, content) {
  const temporary = `${path}.${randomUUID()}.tmp`;
  let descriptor;
  try {
    descriptor = openSync(temporary, "wx", 0o600);
    writeFileSync(descriptor, content); fsyncSync(descriptor); closeSync(descriptor); descriptor = undefined;
    renameSync(temporary, path);
  } finally {
    if (descriptor !== undefined) closeSync(descriptor);
    try { unlinkSync(temporary); } catch (error) { if (error.code !== "ENOENT") throw error; }
  }
}

export function readBounded(path, limit) {
  const descriptor = openSync(path, "r");
  try {
    const size = fstatSync(descriptor).size;
    if (!Number.isSafeInteger(size) || size > limit) throw new Error("The selected file exceeds the supported size limit.");
    const bytes = Buffer.allocUnsafe(size + 1);
    let offset = 0;
    while (offset < bytes.length) {
      const count = readSync(descriptor, bytes, offset, bytes.length - offset);
      if (!count) break;
      offset += count;
    }
    if (offset !== size) throw new Error("The selected file changed while it was being read. Try again.");
    return bytes.subarray(0, size);
  } finally { closeSync(descriptor); }
}

export function installStorage(path) {
  directory = privateDirectory(path);
  const sidecars = privateDirectory(join(directory, "service-state"));
  const entries = new Map();
  for (const name of readdirSync(sidecars)) {
    if (!/^[a-f0-9]{64}\.json$/.test(name)) continue;
    const raw = readBounded(join(sidecars, name), 64 * 1024 * 1024);
    const entry = JSON.parse(raw);
    if (typeof entry.key !== "string" || typeof entry.value !== "string" || `${keyName(entry.key)}.json` !== name) throw new Error("Native synchronization metadata is invalid. Preserve the data directory before recovery.");
    entries.set(entry.key, entry.value);
  }
  globalThis.localStorage = {
    get length() { return entries.size; },
    key(index) { return [...entries.keys()][index] ?? null; },
    getItem(key) {
      if (key === "ferroui.todo.v1") {
        try { return readBounded(join(directory, "tasks.json"), 32 * 1024 * 1024).toString("utf8"); }
        catch (error) { if (error.code === "ENOENT") return null; throw error; }
      }
      return entries.get(key) ?? null;
    },
    setItem(key, value) {
      if (typeof key !== "string" || typeof value !== "string" || key.length > 4096 || value.length > 64 * 1024 * 1024) throw new Error("Invalid native synchronization metadata.");
      atomicWrite(join(sidecars, `${keyName(key)}.json`), JSON.stringify({ key, value })); entries.set(key, value);
    },
    removeItem(key) {
      try { unlinkSync(join(sidecars, `${keyName(key)}.json`)); } catch (error) { if (error.code !== "ENOENT") throw error; }
      entries.delete(key);
    },
  };
}

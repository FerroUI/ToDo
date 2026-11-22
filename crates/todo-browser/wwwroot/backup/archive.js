import { checkTextSize, decodeFile, digest, encodeFile, exactFields, MAX_ARCHIVE_BYTES } from "./codec.js";
import { documentFrom, references, rewriteKeys, independentCopy, documentGuard } from "./document.js";

const KIND = "ferroui.todo.backup";
const VERSION = 1;

export async function exportPortable(runtime, { resolveFile } = {}) {
  const expected = runtime.exportData();
  const guard = documentGuard(runtime, expected);
  const document = documentFrom(runtime, expected);
  const sources = references(document), keys = new Map(), files = [];
  for (const [key, reference] of sources) {
    guard();
    const file = typeof resolveFile === "function" ? await resolveFile(reference) : null;
    guard();
    if (!(file instanceof Blob)) throw new Error(`“${reference.name}” is unavailable. Connect its Microsoft account to download it, or export a task-only JSON backup.`);
    if (file.size !== reference.size) throw new Error(`“${reference.name}” has changed size. Refresh the task before exporting it.`);
    const bytes = new Uint8Array(await file.arrayBuffer());
    const sha256 = await digest(bytes);
    guard();
    const archiveKey = `backup:${keys.size + 1}`;
    keys.set(key, archiveKey);
    files.push({ key: archiveKey, name: reference.name, type: reference.mime_type,
      size: file.size, base64: encodeFile(bytes), sha256 });
  }
  // Local UUIDs and Graph attachment keys (which include account identifiers) are
  // implementation details. The archive uses opaque, document-scoped references.
  rewriteKeys(document, keys);
  guard();
  const archive = new Blob([JSON.stringify({ kind: KIND, version: VERSION, document, files })], { type: "application/json" });
  if (archive.size > MAX_ARCHIVE_BYTES) throw new Error("The portable backup exceeds 256 MB. Export a task-only JSON backup instead.");
  return archive;
}

function parse(text) {
  checkTextSize(text, MAX_ARCHIVE_BYTES, "The backup");
  let value;
  try { value = JSON.parse(text); } catch { throw new Error("The backup is not valid JSON."); }
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("The backup must contain a task document or a portable archive.");
  if (!Object.hasOwn(value, "kind")) return { legacy: true, document: value, files: [] };
  exactFields(value, ["kind", "version", "document", "files"], "The portable backup");
  if (value.kind !== KIND || value.version !== VERSION) throw new Error("This portable backup format or version is not supported. Open it with a compatible app version.");
  if (!Array.isArray(value.files)) throw new Error("The portable backup has no valid file collection.");
  return { ...value, legacy: false };
}

async function validateFiles(files, references, guard) {
  if (files.length !== references.size) throw new Error("The portable backup has missing or unused file entries.");
  const seen = new Set();
  for (const file of files) {
    exactFields(file, ["key", "name", "type", "size", "base64", "sha256"], "A backup file");
    const reference = references.get(file.key);
    if (typeof file.key !== "string" || !reference || seen.has(file.key)) throw new Error("The portable backup has duplicate or unused file entries.");
    seen.add(file.key);
    if (file.name !== reference.name || file.type !== reference.mime_type || file.size !== reference.size) throw new Error("A backup file does not match its task attachment metadata.");
    if (typeof file.sha256 !== "string" || !/^[0-9a-f]{64}$/i.test(file.sha256)) throw new Error("A backup file has no valid SHA-256 checksum.");
    const bytes = decodeFile(file);
    if (await digest(bytes) !== file.sha256.toLowerCase()) throw new Error(`“${file.name}” failed its checksum. The backup is damaged or incomplete.`);
    guard();
  }
}

export async function restorePortable(runtime, text, { storeFile, deleteFile } = {}) {
  const expected = runtime.exportData();
  const guard = documentGuard(runtime, expected);
  const archive = parse(text);
  const document = documentFrom(runtime, archive.document);
  const sources = references(document, !archive.legacy);
  if (!archive.legacy) await validateFiles(archive.files, sources, guard);
  const keys = new Map();
  if (!archive.legacy) for (const key of sources.keys()) keys.set(key, crypto.randomUUID());
  rewriteKeys(document, keys);
  const prepared = documentFrom(runtime, independentCopy(document));
  const json = JSON.stringify(prepared);
  guard();
  if (archive.files.length && typeof storeFile !== "function") throw new Error("This app cannot restore file contents.");
  const stored = [];
  try {
    for (const entry of archive.files) {
      guard();
      const key = keys.get(entry.key);
      const file = new File([decodeFile(entry)], entry.name, { type: entry.type });
      // Record before awaiting: a storage operation may have committed even if
      // its completion is interrupted. Fresh keys cannot overwrite existing files.
      stored.push(key);
      await storeFile(key, file, { name: entry.name, mime_type: entry.type, size: entry.size, storage_key: key });
      guard();
    }
    guard();
    runtime.importData(json);
  } catch (error) {
    if (typeof deleteFile === "function") await Promise.allSettled(stored.map(key => deleteFile(key)));
    throw error;
  }
  return { tasks: prepared.tasks.length, files: archive.files.length, legacy: archive.legacy };
}

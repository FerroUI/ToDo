import { chooseFile, downloadBlob, readAttachment, storeAttachment, deleteAttachment } from "../attachments.js";
import { exportPortable, restorePortable } from "./archive.js";
import { MAX_ARCHIVE_BYTES } from "./codec.js";

export async function readBackupFile(file) {
  if (!(file instanceof Blob) || file.size > MAX_ARCHIVE_BYTES) throw new Error("The backup must be 256 MB or smaller.");
  return file.text();
}

export async function downloadPortable(runtime, {
  resolveFile = reference => readAttachment(reference.storage_key),
  filename = "todo-portable-backup.json", download = downloadBlob,
} = {}) {
  const blob = await exportPortable(runtime, { resolveFile });
  download(blob, filename);
}

export async function chooseAndRestore(runtime, {
  choose = chooseFile, storeFile = storeAttachment, deleteFile = deleteAttachment,
} = {}) {
  const [file] = await choose("application/json,.json");
  if (!file) return null;
  const text = await readBackupFile(file);
  return restorePortable(runtime, text, { storeFile, deleteFile });
}

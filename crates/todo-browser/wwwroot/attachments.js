import { createAttachmentStore } from "./attachment-store.js";

const files = createAttachmentStore();

export async function saveAttachment(file) {
  const key = crypto.randomUUID();
  await storeAttachment(key, file);
  return key;
}

export async function storeAttachment(key, file) {
  if (typeof key !== "string" || !key || !(file instanceof Blob) || file.size > 25 * 1024 * 1024) {
    throw new Error("This attachment has invalid storage metadata or exceeds the 25 MB limit.");
  }
  await files.put(key, file);
}

export const deleteAttachment = key => files.remove(key);
export const readAttachment = key => files.read(key);

export async function openAttachment(key) {
  const file = await readAttachment(key);
  if (!file) throw new Error("This file is not stored on this device. Attach it again from the original file.");
  downloadBlob(file, file.name || "attachment");
}

export function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30_000);
}

export function chooseFile(accept, multiple = false) {
  return new Promise(resolve => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.multiple = multiple;
    input.style.display = "none";
    document.body.append(input);
    input.addEventListener("change", () => { const files = [...input.files]; input.remove(); resolve(files); }, { once: true });
    input.addEventListener("cancel", () => { input.remove(); resolve([]); }, { once: true });
    input.click();
  });
}

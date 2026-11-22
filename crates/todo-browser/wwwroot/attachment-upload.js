import { saveAttachment, deleteAttachment } from "./attachments.js";

export const MAX_TASK_ATTACHMENTS = 25 * 1024 * 1024;

export function checkAttachmentBudget(existing, incoming) {
  for (const file of [...existing, ...incoming]) {
    if (!Number.isSafeInteger(file.size) || file.size < 0) throw new Error("This attachment has an invalid size.");
    if (file.size > MAX_TASK_ATTACHMENTS) throw new Error("Each attachment must be 25 MB or smaller.");
  }
}

let pending = Promise.resolve();

export function attachFiles(runtime, taskId, files, { save = saveAttachment, remove = deleteAttachment } = {}) {
  if (!files.length) return Promise.resolve();
  // File pickers may overlap. Serialize their preflight and commits so a second
  // selection observes the first one's committed file references.
  const next = pending.catch(() => {}).then(async () => {
    const task = JSON.parse(runtime.exportData()).tasks?.find(task => String(task.id) === String(taskId));
    if (!task) throw new Error("This task no longer exists.");
    checkAttachmentBudget(task.attachments || [], files);
    for (const file of files) {
      const key = await save(file);
      try { runtime.attachFile(String(taskId), file.name, file.type, file.size, key); }
      catch (error) { await remove(key); throw error; }
    }
  });
  pending = next;
  return next;
}

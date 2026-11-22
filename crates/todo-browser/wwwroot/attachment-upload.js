import { saveAttachment, deleteAttachment } from "./attachments.js";

export const MAX_TASK_ATTACHMENTS = 25 * 1024 * 1024;

export function checkAttachmentBudget(existing, incoming) {
  let total = 0;
  for (const file of [...existing, ...incoming]) {
    if (!Number.isSafeInteger(file.size) || file.size < 0) throw new Error("This attachment has an invalid size.");
    total += file.size;
    if (total > MAX_TASK_ATTACHMENTS) throw new Error("Attachments for a task must total 25 MB or less.");
  }
}

let pending = Promise.resolve();

export function attachFiles(runtime, taskId, files, { save = saveAttachment, remove = deleteAttachment } = {}) {
  if (!files.length) return Promise.resolve();
  // File pickers may overlap. Serialize their preflight and commits so a second
  // selection includes the first one's saved metadata in its aggregate budget.
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

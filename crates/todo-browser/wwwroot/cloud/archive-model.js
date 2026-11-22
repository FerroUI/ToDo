import { MAX_FILE_BYTES, MAX_FILES_BYTES } from "../backup/codec.js";

export const archiveSourceKey = source => JSON.stringify([source.listId, source.taskId, source.attachmentId]);
const object = value => value && typeof value === "object" && !Array.isArray(value);

function identified(items, label) {
  if (!Array.isArray(items)) throw new Error(`The Microsoft archive has no complete ${label} collection.`);
  const seen = new Set();
  for (const item of items) {
    if (!object(item) || typeof item.id !== "string" || !item.id || /[\u0000-\u001f\u007f]/.test(item.id) || seen.has(item.id)) {
      throw new Error(`The Microsoft archive has invalid or duplicate ${label} identifiers.`);
    }
    seen.add(item.id);
  }
  return items;
}

export function archiveReferences(lists) {
  const references = new Map();
  let bytes = 0, defaults = 0;
  for (const list of identified(lists, "list")) {
    if (typeof list.displayName !== "string" || !list.displayName.trim()) throw new Error("A Microsoft archive list has no name.");
    if (list.wellknownListName === "defaultList" && ++defaults > 1) throw new Error("The Microsoft archive has more than one default task list.");
    for (const task of identified(list.tasks, "task")) {
      if (typeof task.title !== "string" || !task.title.trim()) throw new Error("A Microsoft archive task has no title.");
      if (task.body != null && (!object(task.body) || (task.body.content != null && typeof task.body.content !== "string"))) throw new Error("A Microsoft archive task has an invalid body.");
      if (task.recurrence != null && (!object(task.recurrence) || !object(task.recurrence.pattern) || !object(task.recurrence.range))) throw new Error("A Microsoft archive task has an invalid recurrence record.");
      if (task.categories != null && (!Array.isArray(task.categories) || task.categories.some(value => typeof value !== "string"))) throw new Error("A Microsoft archive task has invalid categories.");
      for (const step of identified(task.checklistItems, "checklist item")) {
        if (typeof step.displayName !== "string" || !step.displayName.trim() || typeof step.isChecked !== "boolean") throw new Error("A Microsoft archive checklist item is invalid.");
      }
      if (!Array.isArray(task.linkedResources) || task.linkedResources.some(link => !object(link))) throw new Error("A Microsoft archive task has no complete linked-resource collection.");
      identified(task.linkedResources.filter(link => link.id != null), "linked resource");
      for (const file of identified(task.attachments, "attachment")) {
        if (!Number.isSafeInteger(file.size) || file.size < 0 || file.size > MAX_FILE_BYTES) throw new Error("A Microsoft archive attachment exceeds 25 MB or has an invalid size.");
        if ((file.name != null && typeof file.name !== "string") || (file.contentType != null && typeof file.contentType !== "string")) throw new Error("A Microsoft archive attachment has invalid name or content type metadata.");
        bytes += file.size;
        if (bytes > MAX_FILES_BYTES) throw new Error("Microsoft account archives support up to 128 MB of file contents.");
        const source = { listId: list.id, taskId: task.id, attachmentId: file.id };
        references.set(archiveSourceKey(source), { source, file, name: file.name || "attachment", type: file.contentType || "application/octet-stream", size: file.size });
      }
    }
  }
  return references;
}

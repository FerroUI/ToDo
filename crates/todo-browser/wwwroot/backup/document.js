import { checkTextSize, MAX_DOCUMENT_BYTES, MAX_FILE_BYTES, MAX_FILES_BYTES } from "./codec.js";

export function documentFrom(runtime, value) {
  const json = typeof value === "string" ? value : JSON.stringify(value);
  checkTextSize(json, MAX_DOCUMENT_BYTES, "The task document");
  const validate = runtime.validateImportData || runtime.prepareCloudData;
  if (typeof validate !== "function") throw new Error("This app cannot validate the backup. Reload the latest version before restoring it.");
  // The Rust parser validates IDs, relationships, dates, source URLs and schema
  // version, and its serializer excludes unknown fields and host credentials.
  const canonical = validate.call(runtime, json);
  if (typeof canonical !== "string") throw new Error("Backup validation did not return a task document.");
  const document = JSON.parse(canonical);
  if (!document || !Array.isArray(document.tasks) || !Array.isArray(document.lists)) throw new Error("The backup has no valid task document.");
  return document;
}

export function references(document, includeContents = true) {
  const files = new Map();
  let size = 0;
  for (const task of document.tasks) {
    for (const file of task.attachments || []) {
      if (typeof file.storage_key !== "string" || !file.storage_key || typeof file.name !== "string" || !file.name.trim()
        || typeof file.mime_type !== "string" || !Number.isSafeInteger(file.size) || file.size < 0 || file.size > MAX_FILE_BYTES) {
        throw new Error("The backup has invalid attachment metadata.");
      }
      const previous = files.get(file.storage_key);
      if (previous) {
        if (previous.name !== file.name || previous.mime_type !== file.mime_type || previous.size !== file.size) {
          throw new Error("The same backup file is referenced with conflicting metadata.");
        }
      } else {
        size += file.size;
        if (includeContents && size > MAX_FILES_BYTES) throw new Error("Portable backups support up to 128 MB of file contents. Export smaller task collections or use a task-only JSON backup.");
        files.set(file.storage_key, file);
      }
    }
  }
  return files;
}

export function rewriteKeys(document, keys) {
  for (const task of document.tasks) for (const file of task.attachments || []) file.storage_key = keys.get(file.storage_key) || file.storage_key;
}

export function independentCopy(document) {
  document.settings ||= {};
  document.settings.auto_sync = false;
  document.settings.planner_enabled = false;
  document.settings.outlook_categories_enabled = false;
  document.settings.presentation_sync_enabled = false;
  document.settings.keep_running_in_background = false;
  for (const list of document.lists) delete list.cloud;
  for (const task of document.tasks) {
    const external = task.external;
    if (external?.url) {
      task.linked_resources ||= [];
      if (!task.linked_resources.some(link => link.web_url === external.url)) {
        task.linked_resources.push({ id: "", application_name: external.provider || "Microsoft",
          display_name: `Open in ${external.provider || "Microsoft"}`, web_url: external.url, external_id: external.id || "" });
      }
    }
    delete task.external;
    task.flagged_email = false;
    task.assigned_to = null;
  }
  return document;
}

export function documentGuard(runtime, expected) {
  return () => {
    if (runtime.exportData() !== expected) throw new Error("Tasks changed while the backup was being processed. Your edits were kept; try again.");
  };
}

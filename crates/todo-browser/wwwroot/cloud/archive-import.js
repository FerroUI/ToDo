import { remoteFields } from "./codec.js";
import { sourceLinks } from "./resources.js";
import { archiveReferences, archiveSourceKey } from "./archive-model.js";
import { exactFields } from "../backup/codec.js";
import { documentFrom } from "../backup/document.js";

const localList = (id, name, position) => ({ id, name, color: "#3E69E4", background: null,
  group_id: null, position, sort: "Manual", sort_reversed: false, show_completed: true });
const dateOf = (value, fallback) => typeof value === "string" && /^\d{4}-\d\d-\d\dT/.test(value) ? value.slice(0, 10) : fallback;

export function microsoftToPortable(runtime, archive) {
  exactFields(archive, ["kind", "version", "exportedAt", "lists", "files"], "The Microsoft archive");
  if (archive.kind !== "ferroui.microsoft-todo" || archive.version !== 1) throw new Error("This Microsoft archive format or version is not supported.");
  if (typeof archive.exportedAt !== "string" || !/^\d{4}-\d\d-\d\dT/.test(archive.exportedAt) || !Number.isFinite(Date.parse(archive.exportedAt))) throw new Error("The Microsoft archive has an invalid export date.");
  const references = archiveReferences(archive.lists);
  if (!Array.isArray(archive.files) || archive.files.length !== references.size) throw new Error("The Microsoft archive has missing or unused file entries.");
  const files = [], mapped = new Map(), keys = new Set();
  for (const file of archive.files) {
    exactFields(file, ["key", "source", "name", "type", "size", "base64", "sha256"], "A Microsoft archive file");
    exactFields(file.source, ["listId", "taskId", "attachmentId"], "A Microsoft archive file source");
    const source = archiveSourceKey(file.source), reference = references.get(source);
    if (!reference || mapped.has(source) || typeof file.key !== "string" || !file.key || keys.has(file.key)) throw new Error("The Microsoft archive has duplicate or orphaned file entries.");
    if (file.name !== reference.name || file.type !== reference.type || file.size !== reference.size) throw new Error("A Microsoft archive file disagrees with its original task metadata.");
    mapped.set(source, file); keys.add(file.key);
    const { source: _source, ...portable } = file;
    files.push(portable);
  }
  const current = documentFrom(runtime, runtime.exportData());
  const document = { schema_version: current.schema_version, tasks: [], lists: [], groups: [], sidebar_order: [],
    settings: { ...current.settings, auto_sync: false, planner_enabled: false }, next_id: 2 };
  const allocate = () => document.next_id++;
  const today = archive.exportedAt.slice(0, 10);
  if (!archive.lists.some(list => list.wellknownListName === "defaultList")) document.lists.push(localList(1, "Tasks", 0));
  for (const list of archive.lists) {
    const local = localList(list.wellknownListName === "defaultList" ? 1 : allocate(), list.displayName, document.lists.length);
    document.lists.push(local);
    for (const original of list.tasks) {
      const projected = remoteFields(original), id = allocate();
      const task = { id, list_id: local.id, ...projected,
        my_day: null, my_day_excluded: null, recurrence_generated: projected.completed && !!original.recurrence,
        reminder: projected.reminder ? { ...projected.reminder, notified: false } : null,
        steps: original.checklistItems.map(step => ({ id: allocate(), title: step.displayName, completed: step.isChecked })),
        attachments: original.attachments.map(file => {
          const entry = mapped.get(archiveSourceKey({ listId: list.id, taskId: original.id, attachmentId: file.id }));
          return { id: allocate(), name: entry.name, mime_type: entry.type, size: entry.size, storage_key: entry.key };
        }),
        assigned_to: null, flagged_email: false, linked_resources: sourceLinks(original),
        // The raw record retains HTML, full recurrence, start time, detailed status,
        // low importance, extensions, and other data outside the local projection.
        microsoft_data: structuredClone(original),
        created_on: dateOf(original.createdDateTime, today), updated_on: dateOf(original.lastModifiedDateTime, today),
        completed_on: projected.completed ? dateOf(original.completedDateTime?.dateTime, today) : null, position: id };
      document.tasks.push(task);
    }
  }
  return { kind: "ferroui.todo.backup", version: 1, document, files };
}

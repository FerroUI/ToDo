import { equal, mergeFields } from "./codec.js";
import { serviceEnvironment } from "./auth.js";
import { requireCloudService } from "./environment.js";

export const isPlanner = task => task.external?.provider === "Planner";
export const plannerPath = id => `planner/tasks/${encodeURIComponent(id)}`;
export const localFields = task => ({ title: task.title, notes: task.notes || "", completed: !!task.completed, due_date: task.due_date || null });
export const remoteFields = (task, details) => ({ title: task.title || "Untitled task", notes: details.description || "", completed: task.percentComplete === 100, due_date: dateOnly(task.dueDateTime) });
export const stepFields = step => ({ title: step.title, completed: !!step.completed });
export const remoteStepFields = step => ({ title: step.title || "Untitled step", completed: !!step.isChecked });

function plannerEntry() {
  const environment = serviceEnvironment();
  requireCloudService(environment, "planner");
  return environment.plannerUrl;
}

function dateOnly(value) {
  if (!value) return null;
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) throw new Error("Planner returned an invalid due date. Your tasks were kept.");
  return date.toISOString().slice(0, 10);
}

export function sourceLinks(details) {
  return Object.entries(details.references || {}).flatMap(([id, value]) => {
    if (!value || typeof value !== "object") return [];
    try {
      const url = new URL(decodeURIComponent(id));
      if (!["https:", "http:"].includes(url.protocol) || url.username || url.password) return [];
      return [{ id, application_name: "Planner", display_name: value.alias || url.hostname, web_url: url.href, external_id: "" }];
    } catch { return []; }
  });
}

export function importTask(id, remote, details, today) {
  return { id, list_id: 1, ...remoteFields(remote, details), important: false, my_day: null, my_day_excluded: null,
    reminder: null, recurrence: null, recurrence_generated: false, steps: [], attachments: [], tags: [],
    assigned_to: "me", flagged_email: false, created_on: dateOnly(remote.createdDateTime) || today,
    updated_on: today, completed_on: remote.percentComplete === 100 ? dateOnly(remote.completedDateTime) || today : null,
    position: id, linked_resources: sourceLinks(details),
    // The production Graph contract has no task webUrl. Keep this official entry
    // point stable instead of fabricating an undocumented task-deep-link route.
    external: { provider: "Planner", id: remote.id, url: plannerEntry() } };
}

export function taskPatch(next, remote) {
  const patch = {};
  if (next.title !== (remote.title || "Untitled task")) patch.title = next.title;
  if (next.completed !== (remote.percentComplete === 100)) patch.percentComplete = next.completed ? 100 : 0;
  if (next.due_date !== dateOnly(remote.dueDateTime)) patch.dueDateTime = next.due_date ? `${next.due_date}T00:00:00.000Z` : null;
  return patch;
}

export function mergeTask(local, remote, details, record) {
  return mergeFields(localFields(local), remoteFields(remote, details), record.baseLocal, record.baseRemote, local.title);
}

export function applyFields(local, fields, remote, details, today) {
  const previous = localFields(local);
  // Planner exposes a plain description. Preserve local formatting while its
  // text is unchanged, and retire it when a remote plain-text edit wins.
  if (previous.notes !== fields.notes) delete local.notes_html;
  Object.assign(local, fields);
  if (!equal(previous, fields)) local.updated_on = today;
  if (previous.completed !== fields.completed) local.completed_on = fields.completed ? dateOnly(remote.completedDateTime) || today : null;
  local.linked_resources = sourceLinks(details);
  local.assigned_to = "me";
  local.external = { provider: "Planner", id: remote.id, url: plannerEntry() };
}

export function requireEtag(value, label) {
  if (typeof value !== "string" || !value) throw new Error(`Planner did not return a revision for ${label}. No unconditional update was sent.`);
  return value;
}

export function changedLocally(task, record) {
  if (!record || task.id !== record.localId || !equal(localFields(task), record.baseLocal)) return true;
  const mappings = record.steps || {};
  return task.steps.some(step => !mappings[step.id] || !equal(stepFields(step), mappings[step.id].baseLocal))
    || Object.keys(mappings).some(id => !task.steps.some(step => step.id === Number(id)));
}

// App-owned presentation state, separate from Microsoft's private To Do state.
import { validBackground } from "../backgrounds.js";
export const PRESENTATION_KIND = "ferroui.todo.presentation";
export const MAX_PRESENTATION_BYTES = 4 * 1024 * 1024;
export const MAX_REPLICAS = 128;
export const PRESENTATION_SCOPE = Object.freeze(["Files.ReadWrite.AppFolder"]);
export const SETTING_FIELDS = Object.freeze(["theme", "confirm_delete", "completion_sound", "show_completed", "show_important", "show_planned", "hide_empty_smart_lists", "show_all",
  "show_completed_list", "show_assigned", "show_flagged_email", "week_starts_monday", "recognize_dates", "remove_recognized_dates", "smart_colors", "smart_backgrounds"]);
export const LIST_FIELDS = Object.freeze(["color", "background", "position", "sort", "sort_reversed", "show_completed"]);
export const TASK_FIELDS = Object.freeze(["my_day", "my_day_excluded", "position"]);
export const GROUP_FIELDS = Object.freeze(["name", "collapsed", "position"]);
export const fieldKey = (kind, id, field) => JSON.stringify([kind, id, field]);
export const same = (left, right) => JSON.stringify(left) === JSON.stringify(right);
export const validDevice = value => typeof value === "string" && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value);
const text = (value, length = 4096) => typeof value === "string" && value.length > 0 && value.length <= length && !/[\u0000-\u001f]/.test(value);
const color = value => typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value);
const date = value => value === null || typeof value === "string" && /^\d{4}-\d{2}-\d{2}$/.test(value)
  && Number.isFinite(Date.parse(`${value}T00:00:00Z`)) && new Date(`${value}T00:00:00Z`).toISOString().slice(0, 10) === value;
const object = value => value && typeof value === "object" && !Array.isArray(value);

export function parseField(key) {
  let parts;
  try { parts = JSON.parse(key); } catch { throw new Error("Device settings contain an invalid field."); }
  if (!Array.isArray(parts) || parts.length !== 3 || !parts.every(value => text(value, 6000))) throw new Error("Device settings contain an invalid field.");
  const [kind, id, field] = parts;
  const valid = kind === "settings" && id === "app" && SETTING_FIELDS.includes(field)
    || kind === "list" && (LIST_FIELDS.includes(field) || field === "group")
    || kind === "task" && (TASK_FIELDS.includes(field) || field === "important")
    || kind === "group" && validDevice(id) && (GROUP_FIELDS.includes(field) || field === "exists")
    || kind === "sidebar" && id === "root" && field === "order";
  if (!valid) throw new Error("Device settings contain an unsupported field.");
  return parts;
}

export function validFieldValue([kind, , field], value) {
  if (kind === "sidebar") return Array.isArray(value) && value.length <= 10_000 && value.every(item =>
    Array.isArray(item) && item.length === 2 && ["list", "group"].includes(item[0]) && text(item[1]));
  if (field === "theme") return ["System", "Light", "Dark"].includes(value);
  if (field === "smart_colors") return object(value) && Object.keys(value).length <= 64 && Object.entries(value).every(([name, hex]) => text(name, 128) && color(hex));
  if (field === "smart_backgrounds") return object(value) && Object.keys(value).length <= 64 && Object.entries(value).every(([name, image]) => text(name, 128) && validBackground(image));
  if (field === "color") return color(value);
  if (field === "background") return value === null || validBackground(value);
  if (field === "group") return value === null || validDevice(value);
  if (field === "sort") return ["Manual", "Importance", "DueDate", "MyDay", "Alphabetical", "Created"].includes(value);
  if (field === "position") return Number.isSafeInteger(value);
  if (field === "name") return text(value, 1024) && value.trim().length > 0;
  if (field === "my_day" || field === "my_day_excluded") return date(value);
  return typeof value === "boolean";
}

export function validateReplica(value) {
  if (!object(value) || value.kind !== PRESENTATION_KIND || value.version !== 1 || !validDevice(value.device)
    || !object(value.records) || Object.keys(value.records).length > 50_000) throw new Error("The OneDrive device settings format is invalid or unsupported.");
  if (new TextEncoder().encode(JSON.stringify(value)).length > MAX_PRESENTATION_BYTES) throw new Error("Device settings exceed the 4 MiB limit.");
  for (const [key, record] of Object.entries(value.records)) {
    const parts = parseField(key);
    if (!object(record) || !Array.isArray(record.clock) || record.clock.length !== 2
      || !Number.isSafeInteger(record.clock[0]) || record.clock[0] < 1 || record.clock[0] >= Number.MAX_SAFE_INTEGER
      || !validDevice(record.clock[1]) || typeof record.deleted !== "boolean"
      || !record.deleted && !validFieldValue(parts, record.value)) throw new Error("Device settings contain an invalid revision or value.");
  }
  return value;
}

export function mergeRecords(...sets) {
  const result = {};
  for (const records of sets) for (const [key, candidate] of Object.entries(records || {})) {
    const old = result[key];
    const compare = !old ? 1 : candidate.clock[0] - old.clock[0] || candidate.clock[1].localeCompare(old.clock[1], "en");
    if (compare > 0) result[key] = candidate;
    else if (compare === 0 && !same(candidate, old)) throw new Error("Two device settings records have the same revision but different contents.");
  }
  return result;
}

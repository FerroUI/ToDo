import { windowsZones } from "./windows-zones.js";
import { fromRecurrence, toRecurrence, recurrenceFields } from "./recurrence.js";
import { noteFields, noteBody, richNoteFields } from "../notes.js";

export const equal = (left, right) => JSON.stringify(left) === JSON.stringify(right);
const civil = value => value?.dateTime?.slice(0, 10) || null;
const optionalNote = ({ notes, notes_html }) => notes_html ? { notes, notes_html } : { notes };
export const fields = task => ({
  title: task.title, ...optionalNote(noteFields(task)), important: !!task.important, completed: !!task.completed,
  due_date: task.due_date || null,
  start_date: task.start_date || civil(task.microsoft_data?.startDateTime),
  reminder: task.reminder ? { date: task.reminder.date, minutes_after_midnight: task.reminder.minutes_after_midnight } : null,
  recurrence: recurrenceFields(task.recurrence), tags: [...(task.tags || [])].sort(),
});

const bodyFields = body => String(body?.contentType).toLowerCase() === "html"
  ? optionalNote(richNoteFields(body.content || "")) : { notes: body?.content || "" };

function reminder(value) {
  if (!value) return null;
  const date = instantInZone(value.dateTime, windowsZones[value.timeZone] || value.timeZone || "UTC");
  if (!Number.isFinite(date.getTime())) throw new Error("Microsoft returned an invalid reminder date.");
  return { date: `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`,
    minutes_after_midnight: date.getHours() * 60 + date.getMinutes() };
}

function instantInZone(text, zone) {
  const parts = text.match(/^(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)/);
  if (!parts) throw new Error("Microsoft returned an invalid reminder date.");
  const wall = Date.UTC(+parts[1], +parts[2] - 1, +parts[3], +parts[4], +parts[5], +parts[6]);
  let timestamp = wall;
  let format;
  try { format = new Intl.DateTimeFormat("en-GB", { timeZone: zone, year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", second: "2-digit", hourCycle: "h23" }); }
  catch { throw new Error(`This browser cannot interpret Microsoft's reminder time zone “${zone}”. Sync stopped to preserve the reminder.`); }
  for (let pass = 0; pass < 4; pass++) {
    const p = Object.fromEntries(format.formatToParts(new Date(timestamp)).map(part => [part.type, part.value]));
    const represented = Date.UTC(+p.year, +p.month - 1, +p.day, +p.hour, +p.minute, +p.second);
    const next = wall - (represented - timestamp);
    if (next === timestamp) return new Date(timestamp);
    timestamp = next;
  }
  throw new Error("A Microsoft reminder falls in a daylight-saving time gap. Adjust that reminder before syncing.");
}

export function remoteFields(task) {
  return { title: task.title || "Untitled task", ...bodyFields(task.body), important: task.importance === "high",
    completed: task.status === "completed", due_date: civil(task.dueDateTime),
    start_date: civil(task.startDateTime),
    reminder: task.isReminderOn ? reminder(task.reminderDateTime) : null,
    recurrence: fromRecurrence(task.recurrence), tags: [...(task.categories || [])].sort() };
}

export function patchFor(next, previous, today) {
  const patch = {};
  for (const key of Object.keys(next)) {
    if (key === "notes_html" && previous && !next.notes_html && !previous.notes_html) continue;
    if (previous && equal(next[key], previous[key])) continue;
    switch (key) {
      case "title": patch.title = next.title; break;
      case "notes": case "notes_html": patch.body = noteBody(next); break;
      case "important": patch.importance = next.important ? "high" : "normal"; break;
      case "completed": patch.status = next.completed ? "completed" : "notStarted"; break;
      case "due_date": patch.dueDateTime = next.due_date ? { dateTime: `${next.due_date}T00:00:00`, timeZone: "UTC" } : null; break;
      case "start_date": patch.startDateTime = next.start_date ? { dateTime: `${next.start_date}T00:00:00`, timeZone: "UTC" } : null; break;
      case "tags": patch.categories = next.tags; break;
      case "recurrence": patch.recurrence = toRecurrence(next.recurrence, next.due_date, today); break;
      case "reminder": {
        patch.isReminderOn = !!next.reminder;
        if (next.reminder) {
          const value = next.reminder;
          const instant = new Date(`${value.date}T${String(Math.floor(value.minutes_after_midnight / 60)).padStart(2, "0")}:${String(value.minutes_after_midnight % 60).padStart(2, "0")}:00`);
          patch.reminderDateTime = { dateTime: instant.toISOString().replace("Z", ""), timeZone: "UTC" };
        } else patch.reminderDateTime = null;
        break;
      }
    }
  }
  if (previous && !equal(next.notes_html || null, previous.notes_html || null)) patch.body = noteBody(next);
  return patch;
}

export function mergeFields(local, remote, baseLocal, baseRemote, label) {
  const merged = {};
  for (const key of Object.keys(local)) {
    if (key === "notes_html") continue;
    if (key === "notes" && [local, remote, baseLocal, baseRemote].some(value => Object.hasOwn(value, "notes_html"))) {
      // Text and formatting are one logical field. Never merge a changed body
      // with the other side's stale markup; legacy journals have no HTML field.
      const left = noteFields(local), right = noteFields(remote);
      const localChanged = !equal(left, noteFields(baseLocal));
      const remoteChanged = !equal(right, noteFields(baseRemote));
      if (localChanged && remoteChanged && !equal(left, right)) throw new Error(`Sync conflict in “${label}” (notes). This device and Microsoft both changed it. Resolve the difference before syncing again.`);
      Object.assign(merged, optionalNote(localChanged ? left : right));
      continue;
    }
    const localChanged = !equal(local[key], baseLocal[key]);
    const remoteChanged = !equal(remote[key], baseRemote[key]);
    if (localChanged && remoteChanged && !equal(local[key], remote[key])) throw new Error(`Sync conflict in “${label}” (${key}). This device and Microsoft both changed it. Resolve the difference before syncing again.`);
    merged[key] = localChanged ? local[key] : remote[key];
  }
  return merged;
}

export function remoteSnapshot(task) {
  return { ...remoteFields(task), steps: (task.checklistItems || []).map(step => ({ id: step.id, title: step.displayName, completed: step.isChecked })).sort((a, b) => a.id.localeCompare(b.id)),
    modified: task.lastModifiedDateTime || "", bodyModified: task.bodyLastModifiedDateTime || "",
    files: (task.attachments || []).map(file => ({ id: file.id, name: file.name, type: file.contentType || "", size: file.size, modified: file.lastModifiedDateTime || "", etag: file["@odata.etag"] || "" })).sort((a, b) => a.id.localeCompare(b.id)),
    resources: (task.linkedResources || []).map(link => ({ id: link.id, applicationName: link.applicationName, displayName: link.displayName, externalId: link.externalId, webUrl: link.webUrl })).sort((a, b) => String(a.id).localeCompare(String(b.id))) };
}

import { windowsZones } from "./windows-zones.js";

export const equal = (left, right) => JSON.stringify(left) === JSON.stringify(right);
const civil = value => value?.dateTime?.slice(0, 10) || null;
const escape = text => text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
export const fields = task => ({
  title: task.title, notes: task.notes || "", important: !!task.important, completed: !!task.completed,
  due_date: task.due_date || null,
  reminder: task.reminder ? { date: task.reminder.date, minutes_after_midnight: task.reminder.minutes_after_midnight } : null,
  recurrence: task.recurrence || null, tags: [...(task.tags || [])].sort(),
});

function textBody(body) {
  if (String(body?.contentType).toLowerCase() !== "html") return body?.content || "";
  const doc = new DOMParser().parseFromString(body.content || "", "text/html");
  for (const element of doc.querySelectorAll("br")) element.replaceWith("\n");
  for (const element of doc.querySelectorAll("p,div,li")) element.append("\n");
  return (doc.body.textContent || "").trimEnd();
}

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
  return { title: task.title || "Untitled task", notes: textBody(task.body), important: task.importance === "high",
    completed: task.status === "completed", due_date: civil(task.dueDateTime),
    reminder: task.isReminderOn ? reminder(task.reminderDateTime) : null,
    recurrence: fromRecurrence(task.recurrence), tags: [...(task.categories || [])].sort() };
}

function fromRecurrence(value) {
  if (!value) return null;
  const pattern = value.pattern;
  const types = { daily: "Day", weekly: "Week", absoluteMonthly: "Month", absoluteYearly: "Year" };
  const weekdays = pattern.type === "weekly" && equal([...(pattern.daysOfWeek || [])].sort(), ["friday", "monday", "thursday", "tuesday", "wednesday"]);
  const unit = weekdays ? "Weekday" : types[pattern.type];
  // Keep unsupported server patterns in Graph: patches only send fields the user changed.
  if (!unit || value.range?.type !== "noEnd" || (weekdays && pattern.interval !== 1) || (pattern.type === "weekly" && !weekdays && pattern.daysOfWeek?.length !== 1)) return null;
  const anchor = new Date(`${value.range.startDate}T12:00:00Z`);
  if (!Number.isFinite(anchor.getTime())) return null;
  const days = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
  // The local model anchors a period on its original date. A Graph pattern whose
  // day differs from its range start cannot be projected without changing phase.
  if (unit === "Week" && pattern.daysOfWeek[0] !== days[anchor.getUTCDay()]) return null;
  if (["Month", "Year"].includes(unit) && pattern.dayOfMonth !== anchor.getUTCDate()) return null;
  if (unit === "Year" && pattern.month !== anchor.getUTCMonth() + 1) return null;
  return { unit, every: weekdays ? 1 : pattern.interval || 1, anchor: value.range?.startDate || null };
}

function toRecurrence(repeat, due, today) {
  if (!repeat) return null;
  const anchor = repeat.anchor || due || today;
  const date = new Date(`${anchor}T12:00:00Z`);
  const pattern = { type: { Day: "daily", Weekday: "weekly", Week: "weekly", Month: "absoluteMonthly", Year: "absoluteYearly" }[repeat.unit], interval: repeat.every };
  if (repeat.unit === "Weekday") {
    if (repeat.every !== 1) throw new Error("Microsoft sync supports the every-weekday schedule; custom business-day intervals remain local.");
    pattern.daysOfWeek = ["monday", "tuesday", "wednesday", "thursday", "friday"];
  }
  if (repeat.unit === "Week") pattern.daysOfWeek = [["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"][date.getUTCDay()]];
  if (["Month", "Year"].includes(repeat.unit)) pattern.dayOfMonth = date.getUTCDate();
  if (repeat.unit === "Year") pattern.month = date.getUTCMonth() + 1;
  return { pattern, range: { type: "noEnd", startDate: anchor } };
}

export function patchFor(next, previous, today) {
  const patch = {};
  for (const key of Object.keys(next)) {
    if (previous && equal(next[key], previous[key])) continue;
    switch (key) {
      case "title": patch.title = next.title; break;
      case "notes": patch.body = { contentType: "html", content: escape(next.notes).replaceAll("\n", "<br>") }; break;
      case "important": patch.importance = next.important ? "high" : "normal"; break;
      case "completed": patch.status = next.completed ? "completed" : "notStarted"; break;
      case "due_date": patch.dueDateTime = next.due_date ? { dateTime: `${next.due_date}T00:00:00`, timeZone: "UTC" } : null; break;
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
  return patch;
}

export function mergeFields(local, remote, baseLocal, baseRemote, label) {
  const merged = {};
  for (const key of Object.keys(local)) {
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

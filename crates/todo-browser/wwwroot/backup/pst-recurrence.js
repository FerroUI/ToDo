import { recurrenceDate, recurrenceDays, recurrenceFields } from "../cloud/recurrence.js";
import { recurrenceOccurrence, recurrenceLength, recurrenceIndexOnOrAfter } from "../cloud/recurrence-dates.js";

const epoch = Date.UTC(1601, 0, 1);
const minutes = value => { const date = recurrenceDate(value); return date ? (date.getTime() - 12 * 3600000 - epoch) / 60000 : NaN; };
const validMinute = value => Number.isInteger(value) && value >= 0 && value <= 0xffffffff;
const mod = (a, b) => (a % b + b) % b;

/** MS-OXOTASK PidLidTaskRecurrence, containing MS-OXOCAL RecurrencePattern.
 * Native projection is returned for edit detection; richer Graph/range identity
 * is restored from metadata only if Outlook has not changed that projection. */
export function encodePstRecurrence(task) {
  if (!task.recurrence) return null;
  const source = recurrenceFields(task.recurrence);
  const fallback = source.anchor || task.due_date || task.created_on;
  const firstIndex = recurrenceIndexOnOrAfter(source, task.due_date || fallback, fallback);
  if (firstIndex == null) return null;
  const start = recurrenceOccurrence(source, firstIndex, fallback), date = recurrenceDate(start);
  const oldAnchor = recurrenceDate(fallback);
  if (!date || !oldAnchor || date.getUTCFullYear() < 1601) return null;
  if (source.ordinal && source.days_of_week?.length > 1 && source.relative_mode !== "nthMatchingDay" && source.ordinal !== "first") return null;
  const projected = { ...source, anchor: start };
  if (["Month", "Year"].includes(source.unit) && !source.ordinal) projected.day_of_month = source.day_of_month ?? oldAnchor.getUTCDate();
  if (source.unit === "Year") projected.month = source.month ?? oldAnchor.getUTCMonth() + 1;
  if (source.unit === "Week" && !source.days_of_week?.length) projected.days_of_week = [recurrenceDays[oldAnchor.getUTCDay()]];
  delete projected.time_zone;
  if (source.end?.kind === "after") projected.end = { kind: "after", value: source.end.value - firstIndex };
  let frequency, kind, period, specific = [];
  if (source.unit === "Day") { frequency = 0x200a; kind = 0; period = source.every * 1440; if (source.every > 999) return null; }
  else if (source.unit === "Week") {
    frequency = 0x200b; kind = 1; period = source.every; if (period > 99) return null;
    specific = [(projected.days_of_week || [recurrenceDays[date.getUTCDay()]]).reduce((mask, day) => mask | 1 << recurrenceDays.indexOf(day), 0)];
  } else if (["Month", "Year"].includes(source.unit)) {
    period = source.every * (source.unit === "Year" ? 12 : 1); if (period > 99) return null;
    frequency = source.unit === "Year" && source.every === 1 ? 0x200d : 0x200c;
    if (source.unit === "Year" && source.every > 1) { projected.unit = "Month"; projected.every = period; delete projected.month; }
    if (source.ordinal) {
      kind = 3;
      specific = [source.days_of_week.reduce((mask, day) => mask | 1 << recurrenceDays.indexOf(day), 0), ["first", "second", "third", "fourth", "last"].indexOf(source.ordinal) + 1];
    } else { kind = 2; specific = [projected.day_of_month]; }
  } else return null; // Arbitrary business-day intervals have no native format.
  const count = recurrenceLength(projected, start);
  if (projected.end && (!count || count > 0x7fffffff)) return null;
  const end = count ? recurrenceOccurrence(projected, count - 1, start) : null;
  if (projected.end?.kind === "on") projected.end = { kind: "on", value: end };
  const startMinutes = minutes(start), endMinutes = end ? minutes(end) : 0x5ae980df;
  if (!validMinute(startMinutes) || !validMinute(endMinutes)) return null;
  const weekStart = recurrenceDays.indexOf(source.week_start || "sunday");
  let first;
  if (kind === 0) first = mod(startMinutes, period);
  else if (kind === 1) first = mod(startMinutes - mod(date.getUTCDay() - weekStart, 7) * 1440, period * 10080);
  else {
    const offset = mod((date.getUTCFullYear() - 1601) * 12 + date.getUTCMonth(), period);
    first = (Date.UTC(1601 + Math.floor(offset / 12), offset % 12, 1) - epoch) / 60000;
  }
  const data = new Uint8Array(50 + specific.length * 4), view = new DataView(data.buffer);
  let at = 0;
  const u16 = value => { view.setUint16(at, value, true); at += 2; };
  const u32 = value => { view.setUint32(at, value, true); at += 4; };
  [0x3004, 0x3004, frequency, kind, 0].forEach(u16);
  [first, period, 0, ...specific, projected.end?.kind === "on" ? 0x2021 : projected.end?.kind === "after" ? 0x2022 : 0x2023, count || 10, weekStart, 0, 0, startMinutes, endMinutes].forEach(u32);
  return { data, rendered: recurrenceFields(projected), dead: !!task.completed || !!task.recurrence_generated };
}

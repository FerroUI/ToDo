// Microsoft Graph patternedRecurrence uses civil dates. Never convert its range
// through the browser time zone: that would move a series across calendar days.
export const recurrenceDays = Object.freeze(["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"]);
const weekdays = recurrenceDays.slice(1, 6);
const indices = ["first", "second", "third", "fourth", "last"];
const units = ["Day", "Weekday", "Week", "Month", "Year"];
const int = (value, min, max) => Number.isInteger(value) && value >= min && value <= max;
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

export function recurrenceDate(value) {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(value) || value.startsWith("0000")) return null;
  const date = new Date(`${value}T12:00:00Z`);
  return Number.isFinite(date.getTime()) && date.toISOString().slice(0, 10) === value ? date : null;
}

function selectedDays(value) {
  if (!Array.isArray(value) || !value.length || value.length > 7 || value.some(day => !recurrenceDays.includes(day))) return null;
  return recurrenceDays.filter(day => value.includes(day));
}

/** Stable semantic field projection for sync journals and recurring successors. */
export function recurrenceFields(value) {
  if (!value) return null;
  const anchor = value.anchor || null, date = recurrenceDate(anchor);
  const every = value.every;
  const unit = value.unit === "Weekday" && every === 1 ? "Week" : value.unit;
  const result = { unit, every, anchor };
  let days = value.unit === "Weekday" && every === 1 ? weekdays : selectedDays(value.days_of_week);
  if (days && !(unit === "Week" && date && same(days, [recurrenceDays[date.getUTCDay()]]))) result.days_of_week = days;
  if (unit === "Week" && value.week_start && value.week_start !== "sunday") result.week_start = value.week_start;
  if (value.day_of_month != null && (!date || value.day_of_month !== date.getUTCDate())) result.day_of_month = value.day_of_month;
  if (value.month != null && (!date || value.month !== date.getUTCMonth() + 1)) result.month = value.month;
  if (value.ordinal) result.ordinal = value.ordinal;
  if (value.relative_mode === "nthMatchingDay" && days?.length > 1 && value.ordinal !== "first") result.relative_mode = value.relative_mode;
  if (value.end && value.end.kind !== "never") result.end = { kind: value.end.kind, value: value.end.value };
  if (value.time_zone?.trim()) result.time_zone = value.time_zone.trim();
  return result;
}

export function fromRecurrence(value) {
  if (!value || typeof value !== "object") return null;
  const { pattern, range } = value;
  if (!pattern || !range || !int(pattern.interval, 1, 65535) || !recurrenceDate(range.startDate)) return null;
  const types = { daily: "Day", weekly: "Week", absoluteMonthly: "Month", relativeMonthly: "Month", absoluteYearly: "Year", relativeYearly: "Year" };
  const unit = types[pattern.type];
  if (!unit) return null;
  const rule = { unit, every: pattern.interval, anchor: range.startDate };
  if (pattern.type === "weekly" || pattern.type.startsWith("relative")) {
    const days = selectedDays(pattern.daysOfWeek);
    if (!days) return null;
    rule.days_of_week = days;
  }
  if (unit === "Week") {
    if (pattern.firstDayOfWeek != null && !recurrenceDays.includes(pattern.firstDayOfWeek)) return null;
    rule.week_start = pattern.firstDayOfWeek || "sunday";
  }
  if (pattern.type.startsWith("relative")) {
    if (!indices.includes(pattern.index)) return null;
    rule.ordinal = pattern.index;
  } else if (unit === "Month" || unit === "Year") {
    if (!int(pattern.dayOfMonth, 1, 31)) return null;
    rule.day_of_month = pattern.dayOfMonth;
  }
  if (unit === "Year") {
    if (!int(pattern.month, 1, 12)) return null;
    rule.month = pattern.month;
  }
  switch (range.type) {
    case "noEnd": break;
    case "endDate":
      if (!recurrenceDate(range.endDate) || range.endDate < range.startDate) return null;
      rule.end = { kind: "on", value: range.endDate }; break;
    case "numbered":
      if (!int(range.numberOfOccurrences, 1, 2147483647)) return null;
      rule.end = { kind: "after", value: range.numberOfOccurrences }; break;
    default: return null;
  }
  if (range.recurrenceTimeZone != null && range.recurrenceTimeZone !== "") {
    if (typeof range.recurrenceTimeZone !== "string" || range.recurrenceTimeZone.length > 128 || /[\x00-\x1f\x7f]/.test(range.recurrenceTimeZone)) return null;
    rule.time_zone = range.recurrenceTimeZone;
  }
  return recurrenceFields(rule);
}

/** Preserve a previously unprojected archive seed during a version upgrade.
 * Explicit removal sets the source recurrence to null in the domain action. */
export function legacySeedRecurrence(task, source) {
  if (task.recurrence || !source?.recurrence || task.microsoft_data?.recurrence === null) return null;
  const value = source.recurrence, local = fromRecurrence(value);
  if (!local) return null;
  const { pattern, range } = value, date = recurrenceDate(range.startDate);
  const legacy = range.type === "noEnd" && (
    pattern.type === "daily" ||
    pattern.type === "weekly" && (same(selectedDays(pattern.daysOfWeek), [recurrenceDays[date.getUTCDay()]]) || pattern.interval === 1 && same(selectedDays(pattern.daysOfWeek), weekdays)) ||
    ["absoluteMonthly", "absoluteYearly"].includes(pattern.type) && pattern.dayOfMonth === date.getUTCDate() && (pattern.type !== "absoluteYearly" || pattern.month === date.getUTCMonth() + 1)
  );
  return legacy ? null : local;
}

export function toRecurrence(value, due, today) {
  if (!value) return null;
  if (!units.includes(value.unit) || !int(value.every, 1, 65535)) throw new Error("Choose a valid repeat pattern and positive interval before syncing.");
  if (value.unit === "Weekday" && value.every !== 1) throw new Error("Microsoft sync supports the every-weekday schedule; custom business-day intervals remain local.");
  const repeat = recurrenceFields(value);
  const anchor = repeat.anchor || due || today, date = recurrenceDate(anchor);
  if (!date) throw new Error("Microsoft recurrence requires a valid start date between years 0001 and 9999.");
  const relative = !!repeat.ordinal;
  if (repeat.relative_mode === "nthMatchingDay") throw new Error("Microsoft Graph cannot represent this combined-weekday ordinal. Keep it in a local or Outlook PST list, or choose a single weekday.");
  const pattern = { type: { Day: "daily", Week: "weekly", Month: relative ? "relativeMonthly" : "absoluteMonthly", Year: relative ? "relativeYearly" : "absoluteYearly" }[repeat.unit], interval: repeat.every };
  if (repeat.unit === "Week" || relative) {
    const days = repeat.days_of_week || (repeat.unit === "Week" ? [recurrenceDays[date.getUTCDay()]] : null);
    if (!selectedDays(days)) throw new Error("Choose at least one valid weekday for this repeat pattern.");
    pattern.daysOfWeek = days;
  }
  if (repeat.unit === "Week") {
    if (repeat.week_start && !recurrenceDays.includes(repeat.week_start)) throw new Error("Choose a valid first day of the recurrence week.");
    pattern.firstDayOfWeek = repeat.week_start || "sunday";
  }
  if (relative) {
    if (!indices.includes(repeat.ordinal) || !["Month", "Year"].includes(repeat.unit)) throw new Error("Choose a valid relative monthly or yearly repeat pattern.");
    pattern.index = repeat.ordinal;
  } else if (["Month", "Year"].includes(repeat.unit)) {
    pattern.dayOfMonth = repeat.day_of_month ?? date.getUTCDate();
    if (!int(pattern.dayOfMonth, 1, 31)) throw new Error("Choose a repeat day from 1 to 31.");
  }
  if (repeat.unit === "Year") {
    pattern.month = repeat.month ?? date.getUTCMonth() + 1;
    if (!int(pattern.month, 1, 12)) throw new Error("Choose a valid repeat month.");
  }
  let range = { type: "noEnd", startDate: anchor };
  if (repeat.end?.kind === "on") {
    if (!recurrenceDate(repeat.end.value) || repeat.end.value < anchor) throw new Error("The repeat end date must be on or after its start date.");
    range = { type: "endDate", startDate: anchor, endDate: repeat.end.value };
  } else if (repeat.end?.kind === "after") {
    if (!int(repeat.end.value, 1, 2147483647)) throw new Error("Choose a positive recurrence occurrence count.");
    range = { type: "numbered", startDate: anchor, numberOfOccurrences: repeat.end.value };
  } else if (repeat.end) throw new Error("Choose a valid recurrence end range.");
  if (repeat.time_zone) range.recurrenceTimeZone = repeat.time_zone;
  return { pattern, range };
}

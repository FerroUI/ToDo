import { recurrenceDate, recurrenceDays, recurrenceFields } from "./recurrence.js";

const dayMs = 86400000;
const iso = date => Number.isFinite(date?.getTime()) && date.getUTCFullYear() >= 1 && date.getUTCFullYear() <= 9999 ? date.toISOString().slice(0, 10) : null;
const addDays = (date, days) => new Date(date.getTime() + days * dayMs);
const make = (year, month, day) => { const date = new Date(0); date.setUTCHours(12, 0, 0, 0); date.setUTCFullYear(year, month - 1, day); return date; };

function monthDate(rule, year, month, anchor) {
  if (year < 1 || year > 9999) return null;
  const last = make(year, month + 1, 0).getUTCDate();
  if (!rule.ordinal) return make(year, month, Math.min(rule.day_of_month ?? anchor.getUTCDate(), last));
  const days = rule.days_of_week || [], index = ["first", "second", "third", "fourth", "last"].indexOf(rule.ordinal);
  if (index < 0 || !days.length) return null;
  if (rule.relative_mode === "nthMatchingDay") {
    const matches = [];
    for (let day = 1; day <= last; day++) {
      const date = make(year, month, day);
      if (days.includes(recurrenceDays[date.getUTCDay()])) matches.push(date);
    }
    return index === 4 ? matches.at(-1) : matches[index];
  }
  const firstWeekday = make(year, month, 1).getUTCDay(), lastWeekday = make(year, month, last).getUTCDay();
  const day = Math.min(...days.map(name => {
    const weekday = recurrenceDays.indexOf(name);
    return index === 4 ? last - (lastWeekday + 7 - weekday) % 7 : 1 + (weekday + 7 - firstWeekday) % 7 + index * 7;
  }));
  return make(year, month, day);
}

/** Bounded, indexed calendar arithmetic shared by PST export and codec tests. */
export function recurrenceOccurrence(value, index, fallback) {
  if (!Number.isSafeInteger(index) || index < 0) return null;
  const rule = recurrenceFields(value), anchor = recurrenceDate(rule?.anchor || fallback);
  if (!anchor || !Number.isInteger(rule.every) || rule.every < 1) return null;
  let date;
  if (rule.unit === "Day") date = addDays(anchor, index * rule.every);
  else if (rule.unit === "Weekday") {
    if (!index) date = anchor;
    else {
      let base = anchor, weekday = (base.getUTCDay() + 6) % 7;
      if (weekday >= 5) base = addDays(base, 4 - weekday);
      const count = index * rule.every;
      date = addDays(base, Math.floor(count / 5) * 7);
      for (let rest = count % 5; rest; rest--) { do { date = addDays(date, 1); } while ([0, 6].includes(date.getUTCDay())); }
    }
  } else if (rule.unit === "Week") {
    const start = recurrenceDays.indexOf(rule.week_start || "sunday");
    const days = rule.days_of_week || [recurrenceDays[anchor.getUTCDay()]];
    const offsets = days.map(day => (recurrenceDays.indexOf(day) + 7 - start) % 7).sort((a, b) => a - b);
    const offset = (anchor.getUTCDay() + 7 - start) % 7, first = offsets.filter(value => value >= offset);
    const rest = index - first.length;
    const cycle = rest < 0 ? 0 : Math.floor(rest / offsets.length) + 1;
    const selected = rest < 0 ? first[index] : offsets[rest % offsets.length];
    date = addDays(anchor, cycle * rule.every * 7 + selected - offset);
  } else if (["Month", "Year"].includes(rule.unit)) {
    const first = monthDate(rule, anchor.getUTCFullYear(), rule.month || anchor.getUTCMonth() + 1, anchor);
    if (!first) return null;
    const step = (index + Number(first < anchor)) * rule.every;
    const month = rule.unit === "Year" ? (anchor.getUTCFullYear() + step) * 12 + (rule.month || anchor.getUTCMonth() + 1) - 1 : anchor.getUTCFullYear() * 12 + anchor.getUTCMonth() + step;
    date = monthDate(rule, Math.floor(month / 12), month % 12 + 1, anchor);
  }
  const result = iso(date);
  if (!result || rule.end?.kind === "after" && index >= rule.end.value || rule.end?.kind === "on" && result > rule.end.value) return null;
  return result;
}

/** Index of the first occurrence on/after a civil date, including finite limits. */
export function recurrenceIndexOnOrAfter(rule, date, fallback) {
  const first = recurrenceOccurrence(rule, 0, fallback);
  if (!first) return null;
  if (first >= date) return 0;
  let lo = 0, hi = 1;
  while (hi < 0x80000000) {
    const value = recurrenceOccurrence(rule, hi, fallback);
    if (!value || value >= date) break;
    lo = hi; hi *= 2;
  }
  while (hi - lo > 1) {
    const mid = lo + Math.floor((hi - lo) / 2), value = recurrenceOccurrence(rule, mid, fallback);
    if (value && value < date) lo = mid; else hi = mid;
  }
  return recurrenceOccurrence(rule, hi, fallback) ? hi : null;
}

export function recurrenceLength(rule, fallback) {
  if (!rule.end || rule.end.kind === "never") return null;
  if (rule.end.kind === "after") return recurrenceOccurrence(rule, rule.end.value - 1, fallback) ? rule.end.value : null;
  let lo = -1, hi = 1;
  while (hi < 0x80000000 && recurrenceOccurrence(rule, hi, fallback)) { lo = hi; hi *= 2; }
  while (hi - lo > 1) { const mid = lo + Math.floor((hi - lo) / 2); if (recurrenceOccurrence(rule, mid, fallback)) lo = mid; else hi = mid; }
  return lo + 1;
}

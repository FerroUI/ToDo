import test from "node:test";
import assert from "node:assert/strict";
import { fromRecurrence, toRecurrence, recurrenceFields, legacySeedRecurrence } from "../crates/todo-browser/wwwroot/cloud/recurrence.js";
import { fields, remoteFields, patchFor } from "../crates/todo-browser/wwwroot/cloud/codec.js";

const start = "2026-11-23";
const patterns = [
  { type: "daily", interval: 3 },
  { type: "weekly", interval: 2, daysOfWeek: ["monday", "thursday", "saturday"], firstDayOfWeek: "monday" },
  { type: "absoluteMonthly", interval: 2, dayOfMonth: 31 },
  { type: "relativeMonthly", interval: 3, daysOfWeek: ["monday", "friday"], index: "last" },
  { type: "absoluteYearly", interval: 4, dayOfMonth: 29, month: 2 },
  { type: "relativeYearly", interval: 1, month: 11, daysOfWeek: ["thursday"], index: "fourth" },
];
test("all Graph patterns and finite ranges round-trip their selectors and time zone", () => {
  for (const pattern of patterns) for (const range of [
    { type: "noEnd", startDate: start },
    { type: "endDate", startDate: start, endDate: "2032-12-31" },
    { type: "numbered", startDate: start, numberOfOccurrences: 25 },
  ]) {
    range.recurrenceTimeZone = "Pacific Standard Time";
    const graph = { pattern, range }, local = fromRecurrence(graph);
    assert(local, JSON.stringify(graph));
    assert.deepEqual(toRecurrence(local), graph);
    assert.deepEqual(fromRecurrence(toRecurrence(local)), local);
  }
});

test("canonical projection aligns defaults, weekday presets, and reordered weekday sets", () => {
  const implicit = { unit: "Month", every: 1, anchor: "2027-01-31" };
  assert.deepEqual(recurrenceFields({ ...implicit, day_of_month: 31, end: { kind: "never" }, time_zone: "" }), implicit);
  const local = { unit: "Weekday", every: 1, anchor: "2026-11-28", end: { kind: "after", value: 3 } };
  const remote = { pattern: { type: "weekly", interval: 1, daysOfWeek: ["friday", "thursday", "wednesday", "tuesday", "monday"] }, range: { type: "numbered", startDate: local.anchor, numberOfOccurrences: 3 } };
  assert.deepEqual(recurrenceFields(local), fromRecurrence(remote));
  const previous = remoteFields({ title: "Weekdays", recurrence: remote });
  assert.deepEqual(patchFor(fields({ title: "Weekdays", recurrence: local }), previous, start), {});
});

test("malformed or unsupported remote patterns stay unprojected and cannot be fabricated", () => {
  const good = { pattern: patterns[0], range: { type: "noEnd", startDate: start } };
  for (const edit of [
    x => x.pattern.interval = 0, x => x.pattern.interval = 65536,
    x => x.range.startDate = "2026-02-30", x => x.pattern.type = "futurePattern",
    x => x.range.type = "futureRange", x => x.range = { type: "numbered", startDate: start, numberOfOccurrences: 0 },
    x => x.range = { type: "endDate", startDate: start, endDate: "2026-11-22" },
    x => x.pattern = { type: "weekly", interval: 1, daysOfWeek: ["noday"] },
    x => x.pattern = { type: "relativeMonthly", interval: 1, daysOfWeek: ["monday"], index: "fifth" },
  ]) { const graph = structuredClone(good); edit(graph); assert.equal(fromRecurrence(graph), null); }
  assert.equal(fromRecurrence({}), null);
  assert.throws(() => toRecurrence({ unit: "Weekday", every: 2, anchor: start }), /business-day/);
  assert.throws(() => toRecurrence({ unit: "Day", every: 1, anchor: "2026-02-30" }), /start date/);
});

test("date-only range keeps its date across process time zones", () => {
  const graph = { pattern: patterns[5], range: { type: "endDate", startDate: start, endDate: "2030-11-30", recurrenceTimeZone: "Europe/Warsaw" } };
  const old = process.env.TZ;
  try { for (const zone of ["Pacific/Kiritimati", "America/Los_Angeles", "Europe/Warsaw"]) {
    process.env.TZ = zone;
    assert.deepEqual(toRecurrence(fromRecurrence(graph)), graph);
  } } finally { if (old == null) delete process.env.TZ; else process.env.TZ = old; }
});

test("legacy unprojected seeds migrate while explicit removal wins, including task moves", () => {
  const source = { recurrence: { pattern: patterns[3], range: { type: "numbered", startDate: start, numberOfOccurrences: 4 } } };
  assert.deepEqual(legacySeedRecurrence({ recurrence: null, microsoft_data: source }, source), fromRecurrence(source.recurrence));
  assert.equal(legacySeedRecurrence({ recurrence: null, microsoft_data: { recurrence: null } }, source), null);
  assert.equal(legacySeedRecurrence({ recurrence: null }, { recurrence: { pattern: patterns[0], range: { type: "noEnd", startDate: start } } }), null);
  assert.throws(() => toRecurrence({ unit: "Month", every: 1, anchor: start, ordinal: "last", days_of_week: ["monday", "friday"], relative_mode: "nthMatchingDay" }), /combined-weekday/);
});

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { encodePstRecurrence } from "../crates/todo-browser/wwwroot/backup/pst-recurrence.js";
import { recurrenceOccurrence, recurrenceLength } from "../crates/todo-browser/wwwroot/cloud/recurrence-dates.js";
import { recurrencePst, recurrenceCases } from "./tests/recurrence-pst-fixture.mjs";

test("recurrence PST fixture is deterministic and parsed independently by the Rust reader", async () => {
  const old = process.env.TZ; process.env.TZ = "UTC";
  try { assert.deepEqual(new Uint8Array(await recurrencePst().arrayBuffer()), new Uint8Array(await readFile(new URL("../crates/todo-interop/tests/fixtures/recurrence.pst", import.meta.url)))); }
  finally { if (old == null) delete process.env.TZ; else process.env.TZ = old; }
});

test("native calendar arithmetic preserves month ends, weekday phase, combined ordinals and inclusive ends", () => {
  const expected = [
    ["2026-11-23", "2026-11-26", "2026-11-29", "2026-12-02"],
    ["2026-11-23", "2026-11-26", "2026-12-07", "2026-12-10", "2026-12-21", "2026-12-24"],
    ["2027-01-31", "2027-02-28", "2027-03-31", "2027-04-30"],
    ["2024-02-29", "2025-02-28", "2026-02-28"],
    ["2026-11-30", "2026-12-28", "2027-01-29"],
    ["2026-11-26", "2027-11-25", "2028-11-23", "2029-11-22"],
  ];
  recurrenceCases.forEach(([, rule], i) => {
    assert.equal(recurrenceLength(rule), expected[i].length);
    assert.deepEqual(expected[i].map((_, index) => recurrenceOccurrence(rule, index)), expected[i]);
    assert.equal(recurrenceOccurrence(rule, expected[i].length), null);
  });
});

test("MS-OXOCAL wire fields use minutes, native ordinal masks, finite last date and required sentinel", () => {
  const weekly = encodePstRecurrence({ recurrence: { unit: "Week", every: 1, anchor: "2007-04-02", days_of_week: ["monday", "thursday", "friday"], end: { kind: "after", value: 12 } } });
  const view = new DataView(weekly.data.buffer);
  assert.equal(weekly.data.length, 54);
  assert.equal(view.getUint16(0, true), 0x3004); assert.equal(view.getUint16(4, true), 0x200b);
  assert.equal(view.getUint16(6, true), 1); assert.equal(view.getUint32(10, true), 8640);
  assert.equal(view.getUint32(14, true), 1); assert.equal(view.getUint32(18, true), 0);
  assert.equal(view.getUint32(22, true), 0x32); assert.equal(view.getUint32(26, true), 0x2022); assert.equal(view.getUint32(30, true), 12);
  assert.equal(view.getUint32(50, true) - view.getUint32(46, true), 25 * 1440);
  const daily = encodePstRecurrence({ recurrence: { unit: "Day", every: 3, anchor: "2011-04-07" } });
  const plain = new DataView(daily.data.buffer);
  assert.equal(plain.getUint32(10, true), 1440); assert.equal(plain.getUint32(14, true), 4320);
  assert.equal(plain.getUint32(46, true), 0x5ae980df);
});

test("native export resumes numbered ranges and rejects unrepresentable formats without guessing", () => {
  const source = recurrenceCases[2][1];
  const resumed = encodePstRecurrence({ recurrence: source, due_date: "2027-02-28" });
  assert.deepEqual(resumed.rendered, { unit: "Month", every: 1, anchor: "2027-02-28", day_of_month: 31, end: { kind: "after", value: 3 } });
  for (const recurrence of [
    { unit: "Day", every: 1000, anchor: "2026-11-23" },
    { unit: "Weekday", every: 2, anchor: "2026-11-23" },
    { unit: "Week", every: 100, anchor: "2026-11-23" },
    { unit: "Month", every: 1, anchor: "2026-11-23", ordinal: "second", days_of_week: ["monday", "friday"] },
  ]) assert.equal(encodePstRecurrence({ recurrence }), null);
});

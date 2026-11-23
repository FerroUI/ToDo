import test from "node:test";
import assert from "node:assert/strict";
import { reminderInstant, reminderIdentity } from "../crates/todo-browser/wwwroot/push-plan.js";

test("spring gaps, repeated clock minutes and skipped dates match civil-time delivery semantics", () => {
  const previous = process.env.TZ;
  try {
    process.env.TZ = "America/New_York";
    assert.equal(new Date(reminderInstant("2026-03-08", 150)).toISOString(), "2026-03-08T07:00:00.000Z");
    assert.equal(new Date(reminderInstant("2026-11-01", 90)).toISOString(), "2026-11-01T05:30:00.000Z");
    process.env.TZ = "Pacific/Apia";
    assert.equal(reminderInstant("2011-12-30", 600), null);
  } finally { if (previous === undefined) delete process.env.TZ; else process.env.TZ = previous; }
});

test("timezone changes replace a delivery instant while retaining its civil occurrence receipt identity", async () => {
  const previous = process.env.TZ;
  try {
    const workspace = "a".repeat(43);
    process.env.TZ = "UTC"; const utc = await reminderIdentity(workspace, 3, "2026-11-23", 720);
    process.env.TZ = "America/New_York"; const local = await reminderIdentity(workspace, 3, "2026-11-23", 720);
    assert.notEqual(utc.at, local.at); assert.notEqual(utc.id, local.id); assert.equal(utc.occurrence, local.occurrence);
  } finally { if (previous === undefined) delete process.env.TZ; else process.env.TZ = previous; }
});

import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { runReviewGroups } from "./ui/review-groups.mjs";

async function harness(t) {
  t.mock.method(console, "log", () => {});
  t.mock.method(console, "error", () => {});
  const output = await mkdtemp(path.join(tmpdir(), "todo-reviews-"));
  t.after(() => rm(output, { recursive: true, force: true }));
  const calls = [], view = { width: 390, height: 844, elements: [] };
  const ui = {
    page: { screenshot: filename => writeFile(filename, "test image") },
    snapshot: async () => view,
    healthy() {},
    navigate: async (command, args) => calls.push([command, args]),
    resize: async (...size) => calls.push(["resize", ...size]),
  };
  return { ui, output, calls, view,
    report: async () => JSON.parse(await readFile(path.join(output, "reviews.json"), "utf8")) };
}

test("passing review groups require no navigation or recovery", async t => {
  const h = await harness(t);
  const ran = [];
  const failures = await runReviewGroups(h.ui, h.output,
    ["detail", "settings"].map(name => ({ name, run: async () => ran.push(name) })));
  assert.deepEqual(failures, []);
  assert.deepEqual(ran, ["detail", "settings"]);
  assert.deepEqual(h.calls, []);
  assert.deepEqual((await h.report()).map(result => result.status), ["passed", "passed"]);
});

test("independent failures retain their own evidence and original errors", async t => {
  const h = await harness(t);
  const detail = new assert.AssertionError({ message: "original detail assertion" });
  const touch = new assert.AssertionError({ message: "original touch assertion" });
  const ran = [];
  const failures = await runReviewGroups(h.ui, h.output, [
    { name: "detail", run: async () => { throw detail; } },
    { name: "settings", run: async () => ran.push("settings") },
    { name: "touch", run: async () => { throw touch; } },
    { name: "sidebar", run: async () => ran.push("sidebar") },
  ]);
  assert.deepEqual(failures.map(failure => failure.error), [detail, touch]);
  assert.deepEqual(ran, ["settings", "sidebar"]);
  assert.deepEqual(h.calls, [
    ["Page.reload", { ignoreCache: true }], ["resize", 1280, 800],
    ["Page.reload", { ignoreCache: true }], ["resize", 1280, 800],
  ]);
  for (const [name, error] of [["detail", detail], ["touch", touch]]) {
    assert.equal(await readFile(path.join(h.output, `failure-${name}.txt`), "utf8"), error.stack);
    assert.equal(await readFile(path.join(h.output, `failure-${name}.png`), "utf8"), "test image");
    assert.deepEqual(JSON.parse(await readFile(path.join(h.output, `failure-${name}.json`), "utf8")), h.view);
  }
  const report = await h.report();
  assert.deepEqual(report.map(result => result.status), ["failed", "passed", "failed", "passed"]);
  assert.deepEqual(report.at(-1).precedingFailures, ["detail", "touch"]);
});

test("failed recovery blocks its review without hiding the original assertion", async t => {
  const h = await harness(t);
  const original = new Error("detail failed"), reload = new Error("reload failed");
  let attempts = 0, blockedRan = false, laterRan = false;
  h.ui.navigate = async () => { if (++attempts === 1) throw reload; };
  // Losing a diagnostic must not replace either original error.
  h.ui.page.screenshot = () => { throw new Error("screenshot unavailable"); };
  const failures = await runReviewGroups(h.ui, h.output, [
    { name: "detail", run: async () => { throw original; } },
    { name: "settings", run: async () => { blockedRan = true; } },
    { name: "list", run: async () => { laterRan = true; } },
  ]);
  assert.equal(blockedRan, false);
  assert.equal(laterRan, true);
  assert.deepEqual(failures.map(failure => failure.error), [original, reload]);
  const report = await h.report();
  assert.deepEqual(report.map(result => result.status), ["failed", "blocked", "passed"]);
  assert.equal(report[1].phase, "recovery");
  assert.equal(report[0].diagnostics[0].artifact, "screenshot");
  assert.equal(await readFile(path.join(h.output, "failure-detail.txt"), "utf8"), original.stack);
});

test("nested gesture reviews retain failures and use only their own recovery and report", async t => {
  const h = await harness(t);
  const original = new Error("held drag failed"), recovery = new Error("fixture navigation failed");
  let attempts = 0, blockedRan = false, finalRan = false;
  await writeFile(path.join(h.output, "reviews.json"), "parent report");
  const failures = await runReviewGroups(h.ui, h.output, [
    { name: "touch-drag", run: async () => { throw original; } },
    { name: "touch-refresh", run: async () => { blockedRan = true; } },
    { name: "touch-sidebar", run: async () => { finalRan = true; } },
  ], {
    recover: async () => { if (++attempts === 1) throw recovery; },
    reportName: "touch-reviews.json",
  });
  assert.deepEqual(failures.map(failure => failure.error), [original, recovery]);
  assert.equal(blockedRan, false);
  assert.equal(finalRan, true);
  assert.equal(attempts, 2);
  assert.deepEqual(h.calls, [], "custom recovery must not also run the generic reload path");
  assert.equal(await readFile(path.join(h.output, "reviews.json"), "utf8"), "parent report");
  const report = JSON.parse(await readFile(path.join(h.output, "touch-reviews.json"), "utf8"));
  assert.deepEqual(report.map(result => result.status), ["failed", "blocked", "passed"]);
  assert.deepEqual(report.at(-1).precedingFailures, ["touch-drag", "touch-refresh"]);
  assert.equal(await readFile(path.join(h.output, "failure-touch-drag.txt"), "utf8"), original.stack);
});

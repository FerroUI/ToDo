import test from "node:test";
import assert from "node:assert/strict";
import { createCategoryCatalog } from "../crates/todo-browser/wwwroot/cloud/categories.js";
import { categoryColor, normalizeCategories } from "../crates/todo-browser/wwwroot/cloud/category-colors.js";
import { cloudEnvironment } from "../crates/todo-browser/wwwroot/cloud/environment.js";
import { Storage } from "./tests/graph-fixture.mjs";

function fixture() {
  let account = "account-a", now = 1000, cloud = "global", rows = [{ id: "blue", displayName: "Work", color: "preset7" }], failure, gate;
  const storage = new Storage(), requests = [];
  const catalogue = createCategoryCatalog({
    collection: async (path, options) => { requests.push({ path, options }); if (gate) await gate; if (failure) throw failure; return structuredClone(rows); },
    accountKey: () => account, environment: () => cloudEnvironment(cloud), storage: () => storage, now: () => now,
  });
  return { catalogue, requests, storage,
    set account(value) { account = value; }, set cloud(value) { cloud = value; }, set now(value) { now = value; },
    set rows(value) { rows = value; }, set failure(value) { failure = value; }, set gate(value) { gate = value; },
  };
}

test("all Outlook presets map to bounded semantic swatches and unknown values remain inert", () => {
  for (let i = 0; i < 25; i++) {
    const value = categoryColor(`Preset${i}`);
    assert.equal(value.preset, `preset${i}`); assert.match(value.color, /^#[0-9A-F]{6}$/); assert(value.color_name);
  }
  for (const value of ["preset25", "url(https://attacker.test)", null, "__proto__", "preset00"]) {
    assert.deepEqual(categoryColor(value), { preset: "none", color_name: "No color", color: "#8A8886" });
  }
  const rows = normalizeCategories([{ id: "1", displayName: "#Blue, Green", color: "preset7" }, { id: "2", displayName: "#blue, green", color: "preset0" }]);
  assert.equal(rows.length, 1); assert.equal(rows[0].name, "#Blue, Green");
  assert.throws(() => normalizeCategories([{ id: "same", displayName: "A" }, { id: "same", displayName: "B" }]), /invalid/);
});

test("catalogue requests only optional mailbox read scope and caches per account", async () => {
  const f = fixture(), first = await f.catalogue.read("account-a");
  assert.deepEqual(f.requests[0].options, { scopes: ["MailboxSettings.Read"] });
  assert.equal(first[0].name, "Work"); assert.equal(first[0].color_name, "Blue");
  assert.deepEqual(await f.catalogue.read("account-a"), first); assert.equal(f.requests.length, 1);
  f.account = "account-b";
  assert.deepEqual(f.catalogue.cached("account-a"), []);
  await assert.rejects(f.catalogue.read("account-a"), error => error.code === "account_changed");
  assert.deepEqual(f.catalogue.cached("account-b"), []);
  await f.catalogue.read("account-b"); assert.equal(f.requests.length, 2);
  f.account = "account-a"; f.now = 400_000;
  await f.catalogue.read("account-a"); assert.equal(f.requests.length, 3);
  await f.catalogue.read("account-a", { force: true }); assert.equal(f.requests.length, 4);
});

test("failed refresh retains the last catalogue and concurrent reads share a request", async () => {
  const f = fixture(); const first = await f.catalogue.read("account-a");
  f.failure = new Error("offline");
  await assert.rejects(f.catalogue.read("account-a", { force: true }), /offline/);
  assert.deepEqual(f.catalogue.cached("account-a"), first);
  f.failure = null; const gate = Promise.withResolvers(); f.gate = gate.promise;
  const a = f.catalogue.read("account-a", { force: true }), b = f.catalogue.read("account-a", { force: true });
  gate.resolve(); await Promise.all([a, b]); assert.equal(f.requests.length, 3);
});

test("account changes and disconnects cannot repopulate category cache from a late request", async () => {
  for (const invalidate of [f => { f.account = "account-b"; }, f => f.catalogue.forget("account-a"), f => f.catalogue.forget()]) {
    const f = fixture(), gate = Promise.withResolvers(); f.gate = gate.promise;
    const request = f.catalogue.read("account-a");
    const rejected = assert.rejects(request, error => error.code === "account_changed");
    invalidate(f); gate.resolve(); await rejected;
    assert.equal(f.storage.length, 0);
  }
});

test("malformed cached or remote categories do not replace a valid catalogue", async () => {
  const f = fixture(); const first = await f.catalogue.read("account-a");
  f.rows = [{ id: "invalid", displayName: 42 }];
  await assert.rejects(f.catalogue.read("account-a", { force: true }), /invalid/);
  assert.deepEqual(f.catalogue.cached("account-a"), first);
  f.storage.setItem(f.storage.key(0), "not-json");
  assert.deepEqual(f.catalogue.cached("account-a"), []); assert.equal(f.storage.length, 0);
});

test("unsupported national catalogue is rejected before Graph and does not use another account cache", async () => {
  const f = fixture(); f.cloud = "china";
  await assert.rejects(f.catalogue.read("account-a"), error => error.code === "cloud_service_unavailable");
  assert.equal(f.requests.length, 0);
});

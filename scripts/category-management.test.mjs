import test from "node:test";
import assert from "node:assert/strict";
import { createCategoryManager, categoryName, categoryPreset } from "../crates/todo-browser/wwwroot/cloud/category-management.js";
import { cloudEnvironment, graphScopes } from "../crates/todo-browser/wwwroot/cloud/environment.js";
import { normalizeCategories } from "../crates/todo-browser/wwwroot/cloud/category-colors.js";

function fixture() {
  const f = { account: "a", cloud: "global", rows: [{ id: "one/a", displayName: "Work", color: "preset7" }], requests: [], invalidations: 0 };
  f.manage = createCategoryManager({
    accountKey: () => f.account, environment: () => cloudEnvironment(f.cloud),
    forget: () => f.invalidations++, read: async () => {
      if (f.readFailure) throw f.readFailure;
      if (f.gate) await f.gate;
      return normalizeCategories(structuredClone(f.rows));
    },
    graph: async (path, method, body, etag, options) => {
      f.requests.push({ path, method, body, etag, options });
      if (f.beforeWrite) f.beforeWrite();
      if (f.writeFailure) throw f.writeFailure;
      if (method === "POST") f.rows.push({ id: "new", ...body });
      if (method === "PATCH") f.rows.find(item => item.id === decodeURIComponent(path.split("/").at(-1))).color = body.color;
      if (method === "DELETE") f.rows = f.rows.filter(item => item.id !== decodeURIComponent(path.split("/").at(-1)));
      if (f.afterWrite) f.afterWrite();
    },
  });
  return f;
}

test("category names and presets validate before any Microsoft write", async () => {
  assert.equal(categoryName(" #Zażółć, team "), "#Zażółć, team");
  for (const value of ["", "\n", "bad\nname", "x".repeat(256), null]) assert.throws(() => categoryName(value));
  for (const value of ["none", ...Array.from({ length: 25 }, (_, i) => `preset${i}`)]) assert.equal(categoryPreset(value), value);
  for (const value of ["preset25", "preset00", "red", "url(https://example.com)", null]) assert.throws(() => categoryPreset(value));
  const f = fixture();
  await assert.rejects(f.manage("a", { kind: "createOutlookCategory", name: "", color: "preset7" }));
  assert.equal(f.requests.length, 0);
});

test("create, recolor and delete use explicit mailbox-write consent and refresh the catalogue", async () => {
  const f = fixture();
  let rows = await f.manage("a", { kind: "createOutlookCategory", name: "Personal", color: "preset4" });
  assert.equal(rows.find(item => item.id === "new").preset, "preset4");
  rows = await f.manage("a", { kind: "updateOutlookCategory", id: "one/a", color: "preset0", expected_color: "preset7" });
  assert.equal(rows.find(item => item.id === "one/a").preset, "preset0");
  rows = await f.manage("a", { kind: "deleteOutlookCategory", id: "one/a" });
  assert.deepEqual(rows.map(item => item.name), ["Personal"]);
  assert.equal(f.requests[1].path, "me/outlook/masterCategories/one%2Fa");
  assert.deepEqual(f.requests.map(item => item.method), ["POST", "PATCH", "DELETE"]);
  assert(f.requests.every(item => JSON.stringify(item.options) === JSON.stringify({ scopes: ["MailboxSettings.ReadWrite"] })));
  assert.deepEqual(f.requests[1].body, { color: "preset0" }, "displayName is immutable in Graph");
  assert.equal(f.requests[2].body, undefined);
});

test("duplicate names and stale color edits do not overwrite the master catalogue", async () => {
  const f = fixture();
  await assert.rejects(f.manage("a", { kind: "createOutlookCategory", name: "WORK", color: "preset1" }), /already exists/);
  await assert.rejects(f.manage("a", { kind: "updateOutlookCategory", id: "one/a", color: "preset1", expected_color: "preset2" }), /changed in Outlook/);
  await f.manage("a", { kind: "deleteOutlookCategory", id: "already-removed" });
  assert.equal(f.requests.length, 0);
});

test("an uncertain create reads back the unique name rather than replaying POST", async () => {
  const f = fixture();
  f.afterWrite = () => { throw new TypeError("response lost"); };
  const result = await f.manage("a", { kind: "createOutlookCategory", name: "New category", color: "preset2" });
  assert.equal(result.find(item => item.name === "New category").preset, "preset2");
  assert.equal(f.requests.length, 1);
});

test("a different account or unsupported cloud cannot receive a delayed category mutation", async () => {
  const f = fixture(), gate = Promise.withResolvers();
  f.gate = gate.promise;
  const operation = f.manage("a", { kind: "createOutlookCategory", name: "Private", color: "preset2" });
  f.account = "b"; gate.resolve();
  await assert.rejects(operation, error => error.code === "account_changed");
  assert.equal(f.requests.length, 0);
  f.gate = null; f.account = "a"; f.cloud = "china";
  await assert.rejects(f.manage("a", { kind: "deleteOutlookCategory", id: "one/a" }), error => error.code === "cloud_service_unavailable");
  assert.equal(f.requests.length, 0);
});

test("category write scopes stay bound to the selected national cloud", () => {
  assert.deepEqual(graphScopes(["MailboxSettings.ReadWrite"], "usgov"), ["https://graph.microsoft.us/MailboxSettings.ReadWrite"]);
  assert.throws(() => graphScopes(["https://graph.microsoft.com/MailboxSettings.ReadWrite"], "usgov"), /not supported/);
});

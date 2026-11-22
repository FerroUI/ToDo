import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { configureMicrosoft } from "./configure-microsoft.mjs";

const CLIENT = "11111111-1111-1111-1111-111111111111";
async function configuration(t, value = { clientId: "", tenant: "common", redirectUri: "", cloud: "global" }) {
  const root = await mkdtemp(path.join(os.tmpdir(), "todo-public-config-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const file = path.join(root, "microsoft-config.json");
  await writeFile(file, JSON.stringify(value));
  return { root, file, read: async () => JSON.parse(await readFile(file, "utf8")) };
}

test("deployment cloud variables use runtime validation and retain installation-derived redirects", async t => {
  const fixture = await configuration(t);
  assert.equal(await configureMicrosoft(fixture.root, { TODO_MICROSOFT_CLIENT_ID: CLIENT, TODO_MICROSOFT_CLOUD: "usgovdod" }), true);
  assert.deepEqual(await fixture.read(), { clientId: CLIENT, tenant: "organizations", redirectUri: "", cloud: "usgovdod" });
  await configureMicrosoft(fixture.root, { TODO_MICROSOFT_REDIRECT_URI: "https://example.test/ToDo/" });
  assert.equal((await fixture.read()).redirectUri, "https://example.test/ToDo/");
});

test("invalid or secret deployment configuration fails before writing output", async t => {
  for (const value of [
    { clientId: CLIENT, clientSecret: "not-for-publication" },
    { clientId: CLIENT, cloud: "usgov", tenant: "consumers" },
    { clientId: CLIENT, cloud: "https://attacker.test" },
    { clientId: CLIENT, redirectUri: "https://example.test/ToDo/?code=private" },
    { clientId: CLIENT, redirectUri: "http://example.test/ToDo/" },
  ]) {
    const fixture = await configuration(t, value);
    await assert.rejects(configureMicrosoft(fixture.root, {}));
    assert.deepEqual(await fixture.read(), value);
  }
});

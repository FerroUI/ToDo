import test from "node:test";
import assert from "node:assert/strict";
import { normalizeConfiguration } from "../crates/todo-browser/wwwroot/cloud/auth-config.js";
import { cleanQuery, hasAuthResponse } from "../crates/todo-browser/wwwroot/cloud/auth-return.js";
import { authenticationError } from "../crates/todo-browser/wwwroot/cloud/auth-errors.js";
import { cloudEnvironment, graphScopes } from "../crates/todo-browser/wwwroot/cloud/environment.js";

const address = new URL("https://ferroui.com/ToDo/?nooffline=1#section");
const clientId = "11111111-1111-1111-1111-111111111111";

test("public configuration derives the exact deployment URI and accepts the supported tenant authorities", () => {
  for (const tenant of ["common", "organizations", "consumers", clientId]) {
    assert.deepEqual(normalizeConfiguration({ clientId, tenant }, address), { clientId, tenant, redirectUri: "https://ferroui.com/ToDo/", cloud: "global" });
  }
  assert.equal(normalizeConfiguration({}, new URL("https://ferroui.com/ToDo/index.html")).redirectUri, "https://ferroui.com/ToDo/");
  assert.equal(normalizeConfiguration({}, new URL("http://localhost:8080/")).redirectUri, "http://localhost:8080/");
});

test("national clouds use a fixed identity/resource pair and organizational tenants", () => {
  for (const [cloud, authority, graph] of [
    ["usgov", "https://login.microsoftonline.us", "https://graph.microsoft.us"],
    ["usgovdod", "https://login.microsoftonline.us", "https://dod-graph.microsoft.us"],
    ["china", "https://login.chinacloudapi.cn", "https://microsoftgraph.chinacloudapi.cn"],
  ]) {
    const settings = normalizeConfiguration({ clientId, cloud }, address);
    assert.equal(settings.tenant, "organizations");
    assert.equal(normalizeConfiguration({ cloud, tenant: "common" }, address).tenant, "organizations");
    assert.equal(cloudEnvironment(cloud).authority, authority);
    assert.equal(cloudEnvironment(cloud).graph, graph);
    assert.deepEqual(graphScopes(["User.Read", "MailboxSettings.Read"], cloud), [`${graph}/User.Read`, `${graph}/MailboxSettings.Read`]);
    assert.throws(() => normalizeConfiguration({ cloud, tenant: "consumers" }, address), /organizational accounts/);
  }
  for (const cloud of ["https://attacker.test", "__proto__", "constructor", 3]) assert.throws(() => normalizeConfiguration({ cloud }, address));
  assert.equal(cloudEnvironment("china").todo, false);
  for (const value of [["https://attacker.test/User.Read"], ["Mail.Read"], ["https://graph.microsoft.com/User.Read"], []]) {
    assert.throws(() => graphScopes(value, "usgov"));
  }
});

test("configuration rejects secrets, malformed IDs and redirects outside this application", () => {
  for (const configuration of [null, [], { clientSecret: "must-never-be-published" }, { clientId: 7 }, { clientId: "not-a-guid" }, { tenant: "https://attacker.test" },
    { redirectUri: "https://attacker.test/ToDo/" }, { redirectUri: "https://ferroui.com/Other/" }, { redirectUri: "https://ferroui.com/ToDo/?code=leak" },
    { redirectUri: "https://ferroui.com/ToDo/#token" }, { redirectUri: "https://user:password@ferroui.com/ToDo/" }]) {
    assert.throws(() => normalizeConfiguration(configuration, address));
  }
  assert.throws(() => normalizeConfiguration({}, new URL("http://example.test/ToDo/")), /exact SPA redirect/);
});

test("OAuth parameters never survive application query restoration", () => {
  assert.equal(cleanQuery("?nooffline=1&code=private&STATE=private&id_token=private&error_description=private&theme=dark"), "?nooffline=1&theme=dark");
  assert.equal(hasAuthResponse(new URL("https://ferroui.com/ToDo/#code=code&state=state")), true);
  assert.equal(hasAuthResponse(new URL("https://ferroui.com/ToDo/?error=access_denied")), true);
  assert.equal(hasAuthResponse(new URL("https://ferroui.com/ToDo/?nooffline=1")), false);
});

test("authentication errors give actionable guidance without echoing token response contents", () => {
  assert.match(authenticationError({ message: "AADSTS50011 private data" }).message, /Single-page application redirect/);
  assert.match(authenticationError({ message: "AADSTS65001 private data" }).message, /administrator approval/);
  assert.equal(authenticationError({ errorCode: "consent_required" }).requiresInteraction, true);
  assert.doesNotMatch(authenticationError({ message: "private-token-and-account-data" }).message, /private-token/);
});

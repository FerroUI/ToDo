import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { Storage } from "./tests/graph-fixture.mjs";

// Each scenario gets a fresh module graph while exercising the shipped modules.
registerHooks({
  resolve(specifier, context, next) {
    const result = next(specifier, context);
    const parent = context.parentURL && new URL(context.parentURL);
    if (parent?.searchParams.has("auth-test") && result.url.includes("/cloud/auth-")) {
      const url = new URL(result.url); url.search = parent.search;
      return { ...result, url: url.href };
    }
    return result;
  },
});
let sequence = 0;
const CLIENT = "11111111-1111-1111-1111-111111111111";
const OTHER = "22222222-2222-2222-2222-222222222222";
const ACCOUNT = { homeAccountId: "selected-account", username: "selected@example.test", name: "Selected user", tenantId: "9188040d-6c67-4c5b-b112-36a304b66dad" };
const BASE = "https://example.test/ToDo/";

async function fixture({ config = { clientId: CLIENT }, saved = false, response = null, accounts = [], failure } = {}) {
  globalThis.sessionStorage = new Storage();
  globalThis.location = new URL(BASE);
  const calls = [], navigation = [];
  let active = accounts[0] || null, tokenFailure, tokenGate, configFailure;
  const app = {
    async handleRedirectPromise(options) { calls.push(["redirect", options]); if (failure) throw failure; return response; },
    getAllAccounts: () => accounts,
    getActiveAccount: () => active,
    setActiveAccount: value => { active = value; },
    async loginRedirect(options) { calls.push(["login", options]); },
    async acquireTokenRedirect(options) { calls.push(["reconnect", options]); },
    async acquireTokenSilent(options) { calls.push(["token", options]); if (tokenFailure) throw tokenFailure; if (tokenGate) await tokenGate; return { accessToken: "test-access-token" }; },
    async clearCache(options) { calls.push(["clear", options]); },
    async logoutRedirect(options) { calls.push(["logout", options]); },
  };
  globalThis.msal = {
    LogLevel: { Error: 0 }, CacheLookupPolicy: { AccessTokenAndRefreshToken: 3, RefreshToken: 4 },
    async createStandardPublicClientApplication(options) { calls.push(["initialize", options]); return app; },
  };
  globalThis.document = {
    createElement: () => ({ remove() {} }),
    head: { append(script) { calls.push(["script", script.src]); queueMicrotask(() => script.onload()); } },
  };
  globalThis.history = { replaceState(_state, _title, url) { navigation.push(url); globalThis.location = new URL(url, BASE); } };
  globalThis.fetch = async (url, options) => {
    calls.push(["fetch", String(url), options]);
    if (configFailure) throw configFailure;
    return { status: 200, ok: true, json: async () => config };
  };
  if (saved) {
    sessionStorage.setItem("ferroui.todo.microsoft.client", CLIENT);
    sessionStorage.setItem("ferroui.todo.microsoft.account", ACCOUNT.homeAccountId);
    if (typeof saved === "object") sessionStorage.setItem("ferroui.todo.microsoft.configuration", JSON.stringify(saved));
  }
  const auth = await import(`../crates/todo-browser/wwwroot/cloud/auth.js?auth-test=${++sequence}`);
  return {
    auth, app, calls, navigation,
    set tokenFailure(value) { tokenFailure = value; }, set tokenGate(value) { tokenGate = value; },
    set configFailure(value) { configFailure = value; },
  };
}

test("deployment configuration enables sign-in without loading MSAL or choosing a cached account on startup", async () => {
  const { auth, calls } = await fixture();
  assert.equal(await auth.restore(), null);
  assert.deepEqual(await auth.configuration(), { clientId: CLIENT, tenant: "common", redirectUri: BASE, cloud: "global", configured: true, source: "deployment" });
  assert.equal(calls.some(([kind]) => kind === "script"), false);
  await auth.connect("");
  const options = calls.find(([kind]) => kind === "initialize")[1];
  assert.equal(options.auth.authority, "https://login.microsoftonline.com/common");
  assert.equal(options.auth.redirectUri, BASE);
  assert.equal(options.cache.cacheLocation, "sessionStorage");
  assert.deepEqual(calls.find(([kind]) => kind === "login")[1], { scopes: ["Tasks.ReadWrite", "User.Read"], prompt: "select_account", redirectUri: BASE });
  assert.equal(auth.account(), null);
});

test("successful redirect selects its account and restores only the application's query", async () => {
  const { auth, navigation } = await fixture({ saved: true, response: { account: ACCOUNT } });
  location.search = "?code=sensitive-code&state=sensitive-state";
  location.hash = "#access_token=old-token";
  sessionStorage.setItem("ferroui.todo.microsoft.return-query", "?nooffline=1&STATE=stale&access_token=never-restore");
  assert.equal(await auth.restore(), ACCOUNT);
  assert.deepEqual(navigation, ["/ToDo/?nooffline=1"]);
  assert.equal(sessionStorage.getItem("ferroui.todo.microsoft.return-query"), null);
  assert.equal(auth.sessionStatus().accountType, "personal");
});

test("cancelled redirect removes OAuth response data and allows a new sign-in", async () => {
  const { auth, navigation, app } = await fixture({ saved: true, failure: { errorCode: "access_denied", message: "raw-sensitive-auth-error" } });
  location.search = "?error=access_denied&error_description=private&nooffline=1";
  await assert.rejects(auth.restore(), error => error.code === "access_denied" && !error.message.includes("private") && !error.message.includes("raw-sensitive"));
  assert.deepEqual(navigation, ["/ToDo/?nooffline=1"]);
  app.handleRedirectPromise = async () => null;
  await auth.connect();
  assert.equal(auth.account(), null);
});

test("restore matches the remembered account and never adopts a different cached account", async () => {
  const otherAccount = { ...ACCOUNT, homeAccountId: "other" };
  const first = await fixture({ saved: true, accounts: [otherAccount, ACCOUNT] });
  assert.equal(await first.auth.restore(), ACCOUNT);
  const second = await fixture({ saved: true, accounts: [otherAccount] });
  assert.equal(await second.auth.restore(), null);
  assert.equal(sessionStorage.getItem("ferroui.todo.microsoft.account"), null);
});

test("token renewal bypasses cached access tokens on a 401 retry and exposes interaction-required status", async () => {
  const fixtureValue = await fixture({ saved: true, accounts: [ACCOUNT] });
  const { auth, calls } = fixtureValue;
  await auth.restore();
  assert.equal(await auth.accessToken(), "test-access-token");
  assert.equal(await auth.accessToken(true), "test-access-token");
  assert.deepEqual(calls.filter(([kind]) => kind === "token").map(([, request]) => request.cacheLookupPolicy), [3, 4]);
  fixtureValue.tokenFailure = { errorCode: "interaction_required", message: "do not expose account claims" };
  await assert.rejects(auth.accessToken(), error => error.requiresInteraction && error.message.includes("Reconnect"));
  assert.equal(auth.sessionStatus().needsInteraction, true);
  await auth.reconnect();
  const request = calls.find(([kind]) => kind === "reconnect")[1];
  assert.equal(request.account, ACCOUNT);
  assert.equal(request.loginHint, ACCOUNT.username);
  fixtureValue.tokenFailure = null;
  await auth.accessToken();
  assert.equal(auth.sessionStatus().needsInteraction, false);
});

test("disconnect prevents an outstanding token result from being used and clears only this account", async () => {
  const value = await fixture({ saved: true, accounts: [ACCOUNT] });
  const gate = Promise.withResolvers();
  await value.auth.restore(); value.tokenGate = gate.promise;
  const token = value.auth.accessToken();
  const rejected = assert.rejects(token, error => error.code === "account_changed");
  await value.auth.disconnect(); gate.resolve(); await rejected;
  assert.equal(value.auth.account(), null);
  assert.deepEqual(value.calls.find(([kind]) => kind === "clear")[1], { account: ACCOUNT });
  assert.equal(sessionStorage.getItem("ferroui.todo.microsoft.client"), null);
  assert.equal(sessionStorage.getItem("ferroui.todo.microsoft.configuration"), null);
});

test("explicit Microsoft sign-out uses MSAL with the selected account and registered return URI", async () => {
  const { auth, calls } = await fixture({ saved: true, accounts: [ACCOUNT] });
  await auth.restore(); await auth.logout();
  assert.deepEqual(calls.find(([kind]) => kind === "logout")[1], { account: ACCOUNT, postLogoutRedirectUri: BASE });
  assert.equal(auth.account(), null);
});

test("a separate client override does not inherit a deployment's tenant restriction", async () => {
  const { auth, calls } = await fixture({ config: { clientId: CLIENT, tenant: "organizations" } });
  await auth.connect(OTHER);
  assert.equal(calls.find(([kind]) => kind === "initialize")[1].auth.authority, "https://login.microsoftonline.com/common");
  assert.equal((await auth.configuration()).clientId, OTHER);
});

test("a saved session can restore offline without refetching deployment configuration", async () => {
  const value = await fixture({ saved: true, accounts: [ACCOUNT] });
  value.configFailure = new TypeError("offline");
  assert.equal(await value.auth.restore(), ACCOUNT);
  assert.equal(value.calls.some(([kind]) => kind === "fetch"), false);
});

test("concurrent restore awaits one initialized MSAL instance and one redirect handler", async () => {
  const { auth, calls } = await fixture({ saved: true, accounts: [ACCOUNT] });
  assert.deepEqual(await Promise.all([auth.restore(), auth.restore()]), [ACCOUNT, ACCOUNT]);
  assert.equal(calls.filter(([kind]) => kind === "initialize").length, 1);
  assert.equal(calls.filter(([kind]) => kind === "redirect").length, 1);
});

test("disconnect during redirect processing cannot be undone by its late account result", async () => {
  const { auth, app, calls } = await fixture({ saved: true });
  const gate = Promise.withResolvers();
  app.handleRedirectPromise = async () => { await gate.promise; return { account: ACCOUNT }; };
  const restore = auth.restore();
  const rejected = assert.rejects(restore, error => error.code === "account_changed");
  await new Promise(resolve => setImmediate(resolve));
  await auth.disconnect(); gate.resolve(); await rejected;
  assert.equal(auth.account(), null);
  assert.deepEqual(calls.find(([kind]) => kind === "clear")[1], { account: ACCOUNT });
  assert.equal(sessionStorage.getItem("ferroui.todo.microsoft.account"), null);
});

test("Planner availability requires a known organizational tenant, including ID-token fallback", async () => {
  for (const [selected, expected] of [
    [ACCOUNT, false],
    [{ ...ACCOUNT, tenantId: OTHER }, true],
    [{ ...ACCOUNT, tenantId: undefined, idTokenClaims: { tid: OTHER } }, true],
    [{ ...ACCOUNT, tenantId: undefined }, false],
  ]) {
    const { auth } = await fixture({ saved: true, accounts: [selected] });
    await auth.restore(); assert.equal(auth.isWorkAccount(), expected);
  }
});

test("a configuration failure still removes sensitive callback parameters and an explicit client can recover", async () => {
  const { auth, navigation } = await fixture();
  sessionStorage.setItem("ferroui.todo.microsoft.configuration", "malformed");
  location.search = "?code=private&state=private&nooffline=1";
  await assert.rejects(auth.restore(), /saved Microsoft sign-in configuration/);
  assert.deepEqual(navigation, ["/ToDo/?nooffline=1"]);
  await auth.connect(CLIENT);
  assert.equal((await auth.configuration()).clientId, CLIENT);
});

test("optional service consent uses the selected account without broadening ordinary sync scopes", async () => {
  const value = await fixture({ saved: true, accounts: [ACCOUNT] });
  await value.auth.restore();
  await value.auth.requestScopes(["MailboxSettings.Read"]);
  const request = value.calls.find(([kind]) => kind === "reconnect")[1];
  assert.deepEqual(request.scopes, ["MailboxSettings.Read"]);
  assert.equal(request.account, ACCOUNT);
  value.tokenFailure = { errorCode: "consent_required" };
  await assert.rejects(value.auth.accessToken(false, ["MailboxSettings.Read"]), error => error.requiresInteraction && /optional Microsoft service/.test(error.message));
  assert.equal(value.auth.sessionStatus().needsInteraction, false);
  await assert.rejects(value.auth.accessToken(false, ["Files.ReadWrite.AppFolder"]));
  assert.equal(value.auth.sessionStatus().needsInteraction, false);
  value.tokenFailure = null;
  await value.auth.accessToken();
  assert.deepEqual(value.calls.filter(([kind]) => kind === "token").at(-1)[1].scopes, ["Tasks.ReadWrite", "User.Read"]);
  await assert.rejects(value.auth.requestScopes(["Mail.Read"]), /not supported/);
});

test("optional token success cannot hide a base task session needing reconnection", async () => {
  const value = await fixture({ saved: true, accounts: [ACCOUNT] });
  await value.auth.restore();
  value.tokenFailure = { errorCode: "interaction_required" };
  await assert.rejects(value.auth.accessToken());
  assert.equal(value.auth.sessionStatus().needsInteraction, true);
  value.tokenFailure = null;
  await value.auth.accessToken(false, ["MailboxSettings.Read"]);
  assert.equal(value.auth.sessionStatus().needsInteraction, true);
  await value.auth.accessToken();
  assert.equal(value.auth.sessionStatus().needsInteraction, false);
});

test("sovereign sessions acquire only their resource scopes and isolate account bindings", async () => {
  const value = await fixture({ saved: { clientId: CLIENT, cloud: "usgovdod", tenant: OTHER }, accounts: [{ ...ACCOUNT, tenantId: OTHER }] });
  await value.auth.restore(); await value.auth.accessToken();
  assert.equal(value.calls.find(([kind]) => kind === "initialize")[1].auth.authority, `https://login.microsoftonline.us/${OTHER}`);
  assert.deepEqual(value.calls.find(([kind]) => kind === "token")[1].scopes, ["https://dod-graph.microsoft.us/Tasks.ReadWrite", "https://dod-graph.microsoft.us/User.Read"]);
  assert.equal(value.auth.accountKey(), `usgovdod:${ACCOUNT.homeAccountId}`);
  assert.equal(value.auth.serviceEnvironment().graph, "https://dod-graph.microsoft.us");
  await value.auth.requestScopes(["Files.ReadWrite.AppFolder"]);
  assert.deepEqual(value.calls.find(([kind]) => kind === "reconnect")[1].scopes, ["https://dod-graph.microsoft.us/Files.ReadWrite.AppFolder"]);
});

test("client override preserves national cloud and refuses to guess a cloud after configuration failure", async () => {
  const value = await fixture({ config: { clientId: CLIENT, cloud: "usgov", tenant: OTHER } });
  await value.auth.connect(OTHER);
  assert.equal(value.calls.find(([kind]) => kind === "initialize")[1].auth.authority, "https://login.microsoftonline.us/organizations");
  const failed = await fixture(); failed.configFailure = new TypeError("offline");
  await assert.rejects(failed.auth.connect(OTHER), /offline/);
  assert.equal(failed.calls.some(([kind]) => kind === "initialize"), false);
});

test("unsupported China To Do connection stops before MSAL or token work", async () => {
  const value = await fixture({ config: { clientId: CLIENT, cloud: "china" } });
  await assert.rejects(value.auth.connect(), error => error.code === "cloud_service_unavailable" && /21Vianet/.test(error.message));
  assert.equal(value.calls.some(([kind]) => kind === "script" || kind === "initialize" || kind === "login"), false);
});

const OPTIONAL_KEY = "ferroui.todo.microsoft.optional-consent";
async function optionalConsentMarker() {
  const value = await fixture({ saved: true, accounts: [ACCOUNT] });
  await value.auth.restore(); await value.auth.requestScopes(["MailboxSettings.Read"]);
  return sessionStorage.getItem(OPTIONAL_KEY);
}

test("declined optional consent preserves only the previous selected account and its task authorization", async () => {
  const marker = await optionalConsentMarker();
  const other = { ...ACCOUNT, homeAccountId: "unselected" };
  const value = await fixture({ saved: true, accounts: [other, ACCOUNT], failure: { errorCode: "access_denied", message: "private callback details" } });
  sessionStorage.setItem(OPTIONAL_KEY, marker); location.search = "?error=access_denied&error_description=private";
  await assert.rejects(value.auth.restore(), error => error.optionalConsent === true && /task connection is still available/.test(error.message) && !error.message.includes("private"));
  assert.equal(value.auth.account(), ACCOUNT); assert.equal(value.auth.sessionStatus().needsInteraction, false);
  assert.equal(sessionStorage.getItem(OPTIONAL_KEY), null);
  assert.equal(value.calls.some(([kind]) => kind === "clear"), false);
  assert.equal(await value.auth.accessToken(), "test-access-token");
  assert.deepEqual(value.calls.find(([kind]) => kind === "token")[1].scopes, ["Tasks.ReadWrite", "User.Read"]);
  assert.deepEqual(value.navigation, ["/ToDo/"]);
});

test("optional callback recovery rejects stale, changed-registration, changed-account, and malformed markers", async () => {
  const marker = JSON.parse(await optionalConsentMarker());
  for (const value of [
    { ...marker, createdAt: Date.now() - 31 * 60_000 },
    { ...marker, configuration: JSON.stringify([OTHER, "common", "global", BASE]) },
    { ...marker, configuration: JSON.stringify([CLIENT, "organizations", "usgov", BASE]) },
    { ...marker, account: "another-account" },
    { ...marker, scopes: ["Mail.Read"] },
  ]) {
    const h = await fixture({ saved: true, accounts: [ACCOUNT], failure: { errorCode: "access_denied" } });
    sessionStorage.setItem(OPTIONAL_KEY, JSON.stringify(value)); location.search = "?error=access_denied";
    await assert.rejects(h.auth.restore(), error => error.code === "access_denied" && !error.optionalConsent);
    assert.equal(h.auth.account(), null); assert.equal(sessionStorage.getItem(OPTIONAL_KEY), null);
  }
  const missing = await fixture({ saved: true, accounts: [{ ...ACCOUNT, homeAccountId: "other" }], failure: { errorCode: "user_cancelled" } });
  sessionStorage.setItem(OPTIONAL_KEY, JSON.stringify(marker)); location.search = "?error=access_denied";
  await assert.rejects(missing.auth.restore(), error => !error.optionalConsent);
  assert.equal(missing.auth.account(), null);
});

test("disconnect during an optional redirect failure cannot resurrect the cached task account", async () => {
  const marker = await optionalConsentMarker();
  const h = await fixture({ saved: true, accounts: [ACCOUNT] }), gate = Promise.withResolvers();
  sessionStorage.setItem(OPTIONAL_KEY, marker); location.search = "?error=access_denied";
  h.app.handleRedirectPromise = async () => { await gate.promise; throw { errorCode: "access_denied" }; };
  const restored = h.auth.restore(), rejected = assert.rejects(restored, error => error.code === "account_changed");
  await new Promise(resolve => setImmediate(resolve));
  await h.auth.disconnect(); gate.resolve(); await rejected;
  assert.equal(h.auth.account(), null); assert.equal(sessionStorage.getItem(OPTIONAL_KEY), null);
  assert.equal(sessionStorage.getItem("ferroui.todo.microsoft.account"), null);
});

test("successful callbacks and explicit base reconnects consume optional consent markers", async () => {
  const marker = await optionalConsentMarker();
  const h = await fixture({ saved: true, accounts: [ACCOUNT], response: { account: ACCOUNT } });
  sessionStorage.setItem(OPTIONAL_KEY, marker); location.search = "?code=opaque&state=opaque";
  await h.auth.restore(); assert.equal(sessionStorage.getItem(OPTIONAL_KEY), null);
  await h.auth.requestScopes(["Files.ReadWrite.AppFolder"]); assert(sessionStorage.getItem(OPTIONAL_KEY));
  await h.auth.reconnect(); assert.equal(sessionStorage.getItem(OPTIONAL_KEY), null);
  await assert.rejects(h.auth.requestScopes(["Tasks.ReadWrite"]), /Use Reconnect/);
});

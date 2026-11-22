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
  assert.deepEqual(await auth.configuration(), { clientId: CLIENT, tenant: "common", redirectUri: BASE, configured: true, source: "deployment" });
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

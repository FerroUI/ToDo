import test from "node:test";
import assert from "node:assert/strict";
import { nativeAuth, deferred } from "./tests/native-auth-fixture.mjs";

const CLIENT = "11111111-1111-1111-1111-111111111111";
const OTHER = "33333333-3333-3333-3333-333333333333";

test("native optional consent nonce failure restores the same account's original MSAL token cache", async t => {
  const f = await nativeAuth(t); await f.auth.connect(CLIENT);
  const before = f.cache().serialize(), selected = f.auth.account(), saved = await f.saved();
  f.state.plan = { badNonce: true, secret: "must-not-replace-valid-tokens" };
  await assert.rejects(f.auth.requestScopes(["MailboxSettings.Read"]), /different sign-in request/);
  assert.equal(f.auth.account(), selected);
  assert.equal(f.cache().serialize(), before);
  assert.deepEqual(await f.saved(), saved);
});

test("native optional consent cancellation and wrong tenant profile preserve the previous selected session", async t => {
  const f = await nativeAuth(t); await f.auth.connect(CLIENT);
  const before = f.cache().serialize(), selected = f.auth.account();
  for (const plan of [
    { authorizationError: true },
    { account: { ...selected, tenantId: "44444444-4444-4444-4444-444444444444", localAccountId: "guest-local" } },
    { account: { ...selected, homeAccountId: "another.home" } },
  ]) {
    f.state.plan = plan;
    await assert.rejects(f.auth.requestScopes(["Files.ReadWrite.AppFolder"]), /cancelled|differs/);
    assert.equal(f.auth.account(), selected);
    assert.equal(f.cache().serialize(), before);
    assert.equal((await f.saved()).cache, before);
  }
});

test("native disconnect cancels a received authorization before exchanging its code", async t => {
  const f = await nativeAuth(t); await f.auth.connect(CLIENT);
  const entered = deferred(), release = deferred(), exchanges = f.state.exchanges.length;
  f.state.plan = { beforeAuthorization: async () => { entered.resolve(); await release.promise; } };
  const flow = f.auth.requestScopes(["MailboxSettings.Read"]); await entered.promise;
  await f.auth.disconnect(); release.resolve();
  await assert.rejects(flow, /cancelled/);
  assert.equal(f.state.exchanges.length, exchanges);
  assert.equal(f.auth.account(), null); assert.equal(await f.saved(), null);
});

test("a late native authorization response cannot overwrite a replacement application's selected session", async t => {
  const f = await nativeAuth(t); await f.auth.connect(CLIENT);
  const entered = deferred(), release = deferred();
  f.state.plan = { afterExchange: async () => { entered.resolve(); await release.promise; } };
  const flow = f.auth.requestScopes(["MailboxSettings.Read"]); await entered.promise;
  const replacement = { ...f.state.account, homeAccountId: "replacement.home", localAccountId: "replacement-local" };
  f.state.plan = { account: replacement };
  await f.auth.connect(OTHER);
  const before = f.cache().serialize(); release.resolve();
  await assert.rejects(flow, /cancelled/);
  assert.equal(f.auth.account(), replacement); assert.equal(f.cache().serialize(), before);
  assert.equal((await f.saved()).account.homeAccountId, replacement.homeAccountId);
});

test("native restart uses the real MSAL cache plugin and a late restore cannot resurrect a disconnected account", async t => {
  const f = await nativeAuth(t); await f.auth.connect(CLIENT); f.auth.disposeSession();
  assert.equal((await f.auth.restore()).homeAccountId, f.state.account.homeAccountId);
  assert.equal((await f.cache().getAllAccounts()).length, 1);
  f.auth.disposeSession();
  const entered = deferred(), release = deferred();
  f.state.beforeRead = async () => { entered.resolve(); await release.promise; };
  const restoring = f.auth.restore(); await entered.promise;
  const disconnecting = f.auth.disconnect(); release.resolve();
  await disconnecting; assert.equal(await restoring, null);
  assert.equal(f.auth.account(), null); assert.equal(await f.saved(), null);
});

test("explicit native client overrides persist while environment-derived defaults follow deployment changes", async t => {
  const f = await nativeAuth(t);
  process.env.TODO_MICROSOFT_CLIENT_ID = CLIENT;
  await f.auth.connect(OTHER); f.auth.disposeSession();
  assert.equal((await f.auth.restore()).homeAccountId, f.state.account.homeAccountId);
  assert.equal((await f.auth.configuration()).clientId, OTHER);
  await f.auth.connect("");
  assert.equal((await f.auth.configuration()).clientId, CLIENT);
  assert.equal((await f.auth.configuration()).clientIdExplicit, false);
  f.auth.disposeSession(); process.env.TODO_MICROSOFT_CLIENT_ID = OTHER;
  await f.auth.restore(); assert.equal((await f.auth.configuration()).clientId, OTHER);
  assert.equal((await f.auth.configuration()).clientIdExplicit, false);
});

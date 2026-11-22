import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";
import { SecureTokenCache } from "../crates/todo-desktop/native/token-cache.mjs";
import { accountIdentity, cacheScope, decryptCache, encryptCache, selectedAccount, MAX_CACHE_BYTES } from "../crates/todo-desktop/native/cache-format.mjs";
import { cacheFiles } from "../crates/todo-desktop/native/cache-storage.mjs";

const configuration = { clientId: "11111111-1111-1111-1111-111111111111", tenant: "common", cloud: "global" };
const account = { homeAccountId: "account.home", tenantId: "tenant-1", environment: "login.microsoftonline.com", localAccountId: "local-1" };
const secret = JSON.stringify({ AccessToken: { example: { secret: "test-access-token-never-real" } }, RefreshToken: { example: { secret: "test-refresh-token-never-real" } } });
const deferred = () => { let resolve; const promise = new Promise(complete => { resolve = complete; }); return { promise, resolve }; };

function setup(t) {
  const directory = mkdtempSync(join(tmpdir(), "todo-secure-cache-")), values = new Map(), calls = [];
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const state = { unavailable: false, beforeRead: null, beforeWrite: null };
  const vault = async (operation, scope, value) => {
    calls.push(operation);
    if (state.unavailable) throw new Error("VAULT ERROR CONTAINING A SECRET THAT MUST NOT BE REPORTED");
    if (operation === "read") { await state.beforeRead?.(); return values.get(scope) ?? null; }
    if (operation === "write") { await state.beforeWrite?.(); values.set(scope, value); return true; }
    if (operation === "delete") { values.delete(scope); return true; }
    throw new Error("unexpected operation");
  };
  return { directory, values, calls, state, vault, create: (config = configuration, options = {}) => new SecureTokenCache(directory, config, { vault, ...options }) };
}

test("native cached sign-in round-trips as authenticated ciphertext with no token or selection in marker files", async t => {
  const f = setup(t), first = f.create();
  assert.equal(await first.select(account, secret), true);
  first.dispose(); // Normal process exit preserves the encrypted account.
  const second = f.create(), loaded = await second.load();
  assert.deepEqual(loaded, { account: accountIdentity(account), cache: secret });
  assert.equal(await second.restore({ getAllAccounts: async () => [account] }), account);
  const files = readdirSync(join(f.directory, "secure-signin"));
  for (const file of files) {
    const bytes = readFileSync(join(f.directory, "secure-signin", file));
    assert.ok(!bytes.includes(Buffer.from("test-access-token-never-real")));
    assert.ok(!bytes.includes(Buffer.from("test-refresh-token-never-real")));
    assert.ok(!bytes.includes(Buffer.from(account.homeAccountId)));
  }
  assert.equal(second.mode, "protected");
});

test("cached sign-in is isolated by profile, client, tenant, cloud, generation, and authentication tag", t => {
  const f = setup(t), scope = cacheScope(f.directory, configuration), key = randomBytes(32), generation = "a".repeat(32);
  const encrypted = encryptCache(scope, generation, account, secret, key);
  for (const variant of [
    { ...configuration, clientId: "22222222-2222-2222-2222-222222222222" },
    { ...configuration, tenant: "organizations" }, { ...configuration, cloud: "usgov" }
  ]) assert.throws(() => decryptCache(cacheScope(f.directory, variant), generation, encrypted, key), /authenticated/);
  const other = setup(t);
  assert.throws(() => decryptCache(cacheScope(other.directory, configuration), generation, encrypted, key), /authenticated/);
  assert.throws(() => decryptCache(scope, "b".repeat(32), encrypted, key), /authenticated/);
  const changed = Buffer.from(encrypted); changed[changed.length - 1] ^= 1;
  assert.throws(() => decryptCache(scope, generation, changed, key), /authenticated/);
  assert.throws(() => encryptCache(scope, generation, account, "x".repeat(MAX_CACHE_BYTES), key), /size limit/);
});

test("resume requires one exact selected tenant profile and never chooses the first cached account", async t => {
  const f = setup(t), first = f.create(); await first.select(account, secret);
  const guest = { ...account, tenantId: "another-tenant", localAccountId: "guest-id" };
  const restored = f.create();
  assert.equal(await restored.restore({ getAllAccounts: async () => [guest, account] }), account);
  assert.equal(selectedAccount([guest, account], { homeAccountId: account.homeAccountId }), null);
  const ambiguous = f.create();
  assert.equal(await ambiguous.restore({ getAllAccounts: async () => [account, { ...account }] }), null);
  assert.equal(ambiguous.mode, "reconnect");
  assert.equal(await f.create().load(), null);
});

test("vault failures latch session-only mode until deliberate retry and never create plaintext fallback", async t => {
  const f = setup(t), cache = f.create(); f.state.unavailable = true;
  assert.equal(await cache.select(account, secret), false);
  assert.equal(cache.mode, "session"); assert.doesNotMatch(cache.hint(), /VAULT ERROR|SECRET/);
  const attempts = f.calls.length;
  for (let i = 0; i < 3; i++) { await cache.load(); await cache.save(secret); }
  assert.equal(f.calls.length, attempts);
  assert.ok(readdirSync(join(f.directory, "secure-signin")).every(name => name.endsWith(".json")));
  f.state.unavailable = false; cache.retry();
  assert.equal(await cache.select(account, secret), true);
  assert.equal((await f.create().load()).cache, secret);
});

test("damaged ciphertext is not retried automatically and explicit reconnect can replace it", async t => {
  const f = setup(t), first = f.create(); await first.select(account, secret);
  const path = join(f.directory, "secure-signin", `${first.scope.id}.bin`), bytes = readFileSync(path);
  bytes[bytes.length - 1] ^= 1; writeFileSync(path, bytes);
  const cache = f.create(); assert.equal(await cache.load(), null);
  const attempts = f.calls.length;
  assert.equal(await cache.load(), null); assert.equal(f.calls.length, attempts);
  cache.retry(); assert.equal(await cache.select(account, secret), true);
  assert.equal((await f.create().load()).cache, secret);
});

test("disconnect tombstone prevents account resurrection even when vault and ciphertext cleanup fail", async t => {
  const f = setup(t), cache = f.create(); await cache.select(account, secret);
  const real = cache.files;
  cache.files = { ...real, deletePayload() { throw new Error("read-only payload"); } };
  f.state.unavailable = true;
  assert.equal(await cache.forget(), false);
  assert.equal(cache.mode, "cleanup");
  f.state.unavailable = false;
  const calls = f.calls.length;
  assert.equal(await f.create().load(), null);
  assert.equal(f.calls.length, calls, "disabled selection must not even probe the vault");
});

test("a failed durable tombstone cannot report successful persistent disconnect", async t => {
  const f = setup(t), cache = f.create(); await cache.select(account, secret);
  cache.files = { ...cache.files, writeMarker() { throw new Error("cannot persist selection"); } };
  await assert.rejects(cache.forget(), /cannot persist selection/);
});

test("a partial enabled-marker write is revoked before declaring session-only fallback", async t => {
  const f = setup(t), cache = f.create(), files = cache.files;
  cache.files = { ...files, writeMarker(marker) {
    files.writeMarker(marker);
    if (marker.enabled) throw new Error("directory sync failed after rename");
  } };
  assert.equal(await cache.select(account, secret), false);
  assert.equal(cache.mode, "session");
  assert.equal(await f.create().load(), null);
});

test("unresolved marker durability is reported as a storage error instead of session-only success", async t => {
  const f = setup(t), cache = f.create(), files = cache.files;
  let enabled = false;
  cache.files = { ...files, writeMarker(marker) {
    if (enabled) throw new Error("directory became read-only");
    files.writeMarker(marker);
    if (marker.enabled) { enabled = true; throw new Error("directory sync failed after rename"); }
  } };
  assert.equal(await cache.select(account, secret), false);
  assert.equal(cache.mode, "storage-error");
  assert.doesNotMatch(cache.hint(), /lasts only until/);
  assert.ok(await f.create().load(), "persisted selection is explicitly reported as unresolved");
});

test("an in-flight vault key write cannot recreate a saved account or key after disconnect completes", async t => {
  const f = setup(t), cache = f.create(), entered = deferred(), release = deferred();
  f.state.beforeWrite = async () => { entered.resolve(); await release.promise; };
  const selecting = cache.select(account, secret); await entered.promise;
  const forgetting = cache.forget();
  release.resolve();
  assert.equal(await selecting, false); assert.equal(await forgetting, true);
  assert.equal(f.values.size, 0); assert.equal(cache.keyBytes, undefined);
  assert.equal(await f.create().load(), null);
});

test("late cache-plugin writes after revocation are ignored and optional-consent suspension preserves the prior selection", async t => {
  const f = setup(t), cache = f.create(); await cache.select(account, secret);
  const context = { cacheHasChanged: true, tokenCache: { deserialize() {}, serialize: () => "changed-cache" } };
  await cache.plugin.beforeCacheAccess(context);
  const resume = cache.pauseWrites(); await cache.plugin.afterCacheAccess(context); resume();
  assert.equal((await f.create().load()).cache, secret);
  cache.revoke(); await cache.plugin.afterCacheAccess(context);
  assert.equal(await f.create().load(), null);
});

test("ciphertext alone never implies account selection after interrupted enable", async t => {
  const f = setup(t), cache = f.create(); await cache.select(account, secret);
  cache.files.writeMarker({ enabled: false });
  const fresh = f.create();
  assert.equal(await fresh.load(), null);
  assert.equal(await fresh.restore({ getAllAccounts: async () => [account] }), null);
});

test("a late MSAL account enumeration cannot restore a revoked cache", async t => {
  const f = setup(t), cache = f.create(); await cache.select(account, secret);
  const entered = deferred(), release = deferred();
  const restoring = cache.restore({ getAllAccounts: async () => { entered.resolve(); await release.promise; return [account]; } });
  await entered.promise; cache.revoke(); release.resolve();
  assert.equal(await restoring, null); assert.equal(await f.create().load(), null);
});

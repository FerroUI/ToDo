import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const repository = fileURLToPath(new URL("../../", import.meta.url));
const original = name => pathToFileURL(join(repository, "crates/todo-desktop/native", name)).href;

// Exercise the production auth/cache code and the real vendored MSAL cache.
// Only browser, network, and operating-system vault boundaries are replaced.
export async function nativeAuth(t) {
  const root = mkdtempSync(join(tmpdir(), "todo-native-auth-")), data = join(root, "data");
  mkdirSync(data); mkdirSync(join(root, "web/cloud"), { recursive: true });
  cpSync(join(repository, "crates/todo-desktop/native"), join(root, "native"), { recursive: true });
  cpSync(join(repository, "crates/todo-browser/wwwroot/cloud/environment.js"), join(root, "web/cloud/environment.js"));
  writeFileSync(join(root, "package.json"), '{"type":"module"}');
  writeFileSync(join(root, "native/control.mjs"), `
export const state = { applications: [], authorizations: [], exchanges: [], vault: new Map(), plan: {},
 account: { homeAccountId: "test.home", tenantId: "22222222-2222-2222-2222-222222222222",
 environment: "login.microsoftonline.com", localAccountId: "test-local", username: "test@example.invalid", name: "Test Account" } };
export function snapshot(account, secret = "synthetic-session") {
 return JSON.stringify({ Account: { [account.homeAccountId + "-" + account.environment + "-" + account.tenantId]: {
 home_account_id: account.homeAccountId, environment: account.environment, realm: account.tenantId,
 local_account_id: account.localAccountId, username: account.username, name: account.name, authority_type: "MSSTS" } },
 AccessToken: {}, IdToken: {}, AppMetadata: {}, RefreshToken: { synthetic: {
 home_account_id: account.homeAccountId, environment: account.environment, client_id: "11111111-1111-1111-1111-111111111111",
 credential_type: "RefreshToken", secret } } });
}
`);
  writeFileSync(join(root, "native/vendor/msal-node-7.2.3.mjs"), `
import { PublicClientApplication as Actual, CryptoProvider, LogLevel } from ${JSON.stringify(original("vendor/msal-node-7.2.3.mjs"))};
import { state, snapshot } from "../control.mjs";
export { CryptoProvider, LogLevel };
export class PublicClientApplication extends Actual {
 constructor(configuration) { super(configuration); state.applications.push(this); }
 async getAuthCodeUrl(options) { this.nonce = options.nonce; state.authorizations.push({ application: this, options }); return "https://identity.invalid/"; }
 async acquireTokenByCode(options) {
  const plan = state.plan, nonce = this.nonce;
  state.exchanges.push({ application: this, options });
  await this.getTokenCache().getAllAccounts();
  const account = plan.account || state.account;
  this.getTokenCache().deserialize(snapshot(account, plan.secret || "synthetic-new-token"));
  await plan.afterExchange?.();
  return { account, idTokenClaims: { nonce: plan.badNonce ? "invalid-nonce" : nonce }, accessToken: "synthetic-token" };
 }
}
`);
  writeFileSync(join(root, "native/loopback.mjs"), `
import { state } from "./control.mjs";
export { nativeRedirect } from ${JSON.stringify(original("loopback.mjs"))};
export async function receiveAuthorization({ authorize, redirectUri }) {
 const plan = state.plan;
 await authorize({ redirectUri, state: "synthetic-state" });
 await plan.beforeAuthorization?.();
 if (plan.authorizationError) throw new Error("Microsoft sign-in was cancelled.");
 return { code: "synthetic-code", redirectUri };
}
`);
  writeFileSync(join(root, "native/bridge.mjs"), 'export function ui() { throw new Error("Auth tests must not open the system browser."); }');
  writeFileSync(join(root, "native/credentials.mjs"), `
import { state } from "./control.mjs";
export async function credential(operation, scope, value) {
 if (operation === "read") { await state.beforeRead?.(); return state.vault.get(scope) ?? null; }
 if (operation === "write") { state.vault.set(scope, value); return true; }
 if (operation === "delete") { state.vault.delete(scope); return true; }
 throw new Error("Unexpected credential operation");
}
`);
  const oldEnvironment = Object.fromEntries(Object.entries(process.env).filter(([key]) => key.startsWith("TODO_MICROSOFT_")));
  for (const key of Object.keys(oldEnvironment)) delete process.env[key];
  const previousStorage = globalThis.localStorage;
  const load = name => import(pathToFileURL(join(root, "native", name)).href);
  const { installStorage } = await load("storage.mjs"); installStorage(data);
  const auth = await load("auth.mjs"), { state } = await load("control.mjs"), { SecureTokenCache } = await load("token-cache.mjs");
  t.after(() => {
    auth.disposeSession(); globalThis.localStorage = previousStorage;
    for (const key of Object.keys(process.env)) if (key.startsWith("TODO_MICROSOFT_")) delete process.env[key];
    Object.assign(process.env, oldEnvironment); rmSync(root, { recursive: true, force: true });
  });
  return { auth, state, data, cache: () => state.applications.at(-1).getTokenCache(),
    saved: async () => new SecureTokenCache(data, await auth.configuration()).load() };
}

export function deferred() {
  let resolve; const promise = new Promise(complete => { resolve = complete; });
  return { promise, resolve };
}

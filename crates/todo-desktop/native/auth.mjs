import { randomUUID } from "node:crypto";
import { PublicClientApplication, CryptoProvider, LogLevel } from "./vendor/msal-node-7.2.3.mjs";
import { BASE_SCOPES, cloudEnvironment, graphScopes, requireCloudService } from "../web/cloud/environment.js";
import { receiveAuthorization, nativeRedirect } from "./loopback.mjs";
import { ui } from "./bridge.mjs";
import { dataDirectory } from "./storage.mjs";
import { SecureTokenCache } from "./token-cache.mjs";
import { accountIdentity, selectedAccount } from "./cache-format.mjs";

export const SCOPES = BASE_SCOPES;
const KEY = "ferroui.todo.native.public-configuration";
const PERSONAL = "9188040d-6c67-4c5b-b112-36a304b66dad";
const GUID = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i;
let settings, application, selected, persistence, authEpoch = 0, needsInteraction = false;
let restoreFailure = false;
const cryptoProvider = new CryptoProvider();

function readConfiguration(clientId) {
  let saved = {};
  try { saved = JSON.parse(localStorage.getItem(KEY) || "{}"); } catch { /* Invalid saved public configuration is replaced by explicit settings. */ }
  const cloud = process.env.TODO_MICROSOFT_CLOUD || saved.cloud || "global";
  const environment = cloudEnvironment(cloud);
  const tenant = process.env.TODO_MICROSOFT_TENANT || saved.tenant || (cloud === "global" ? "common" : "organizations");
  if (!["common", "organizations", "consumers"].includes(tenant) && !GUID.test(tenant)) throw new Error("Microsoft tenant must be common, organizations, consumers, or a directory GUID.");
  if (cloud !== "global" && tenant === "consumers") throw new Error("National Microsoft clouds require an organizational account.");
  const explicit = clientId?.trim();
  const clientIdExplicit = !!explicit || (clientId === undefined && saved.clientIdExplicit === true);
  const id = explicit || (clientIdExplicit ? saved.clientId : "") || process.env.TODO_MICROSOFT_CLIENT_ID || (!saved.clientIdExplicit ? saved.clientId : "") || "";
  if (id && !GUID.test(id)) throw new Error("Microsoft application client ID must be a GUID.");
  const redirectUri = nativeRedirect(process.env.TODO_MICROSOFT_NATIVE_REDIRECT_URI || "http://localhost/").href;
  return { clientId: id, tenant, cloud: environment.id, redirectUri, configured: !!id, clientIdExplicit };
}

export async function configuration() { return settings || readConfiguration(); }
export function serviceEnvironment() { return cloudEnvironment((settings || readConfiguration()).cloud); }
export function account() { return selected || null; }
export function accountKey() { return selected ? (serviceEnvironment().id === "global" ? selected.homeAccountId : `${serviceEnvironment().id}:${selected.homeAccountId}`) : null; }
export function isWorkAccount() { return !!selected && GUID.test(selected.tenantId || "") && selected.tenantId.toLowerCase() !== PERSONAL; }
export function sessionStatus() { return { connected: !!selected, needsInteraction, username: selected?.username || "", displayName: selected?.name || selected?.username || "", accountType: selected ? isWorkAccount() ? "workOrSchool" : "personal" : null }; }
export async function restore() {
  if (selected) return selected;
  if (!readConfiguration().configured) return null;
  let app, saved, epoch;
  try {
    await initialize();
    app = application; saved = persistence; epoch = authEpoch;
    const restored = await saved.restore(app.getTokenCache());
    if (epoch !== authEpoch || app !== application || saved !== persistence) return null;
    selected = restored;
    return account();
  } catch {
    if (epoch === undefined || (epoch === authEpoch && app === application && saved === persistence)) restoreFailure = true;
    return null;
  }
}

export function persistenceHint() {
  if (needsInteraction) return "Microsoft needs you to reconnect this account before synchronization can continue.";
  if (restoreFailure) return "Saved Microsoft sign-in could not be restored. Connect Microsoft to continue; local tasks remain available.";
  return persistence?.hint() || "Microsoft sign-in opens your system browser. It is remembered on this device when the operating system credential store is available.";
}

function currentPersistence() {
  const configured = settings || readConfiguration();
  if (!configured.configured) return null;
  return persistence ||= new SecureTokenCache(dataDirectory(), configured);
}

export function revokeResume() { currentPersistence()?.revoke(); }

async function initialize(clientId) {
  const next = readConfiguration(clientId);
  if (!next.clientId) throw new Error("Enter a registered Microsoft application's public client ID, or set TODO_MICROSOFT_CLIENT_ID.");
  requireCloudService(cloudEnvironment(next.cloud), "todo");
  if (application && JSON.stringify(next) === JSON.stringify(settings)) return;
  persistence?.dispose(); authEpoch++;
  const environment = cloudEnvironment(next.cloud);
  persistence = new SecureTokenCache(dataDirectory(), next);
  application = new PublicClientApplication({ auth: { clientId: next.clientId, authority: `${environment.authority}/${next.tenant}` }, cache: { cachePlugin: persistence.plugin }, system: {
    loggerOptions: { piiLoggingEnabled: false, logLevel: LogLevel.Error, loggerCallback() {} },
  } });
  selected = undefined; needsInteraction = false; settings = next;
  localStorage.setItem(KEY, JSON.stringify(next));
}

async function interactive(scopes, { choose = false } = {}) {
  const previous = selected, epoch = authEpoch, app = application, config = settings, saved = persistence;
  const resumeWrites = saved.pauseWrites(), snapshot = app.getTokenCache().serialize();
  const current = () => epoch === authEpoch && app === application && saved === persistence;
  const requireCurrent = () => { if (!current()) throw new Error("Microsoft sign-in was cancelled."); };
  try {
    const codes = await cryptoProvider.generatePkceCodes();
    requireCurrent();
    const qualified = graphScopes(scopes, config.cloud), nonce = randomUUID();
    const response = await receiveAuthorization({ redirectUri: config.redirectUri,
      authorize: ({ redirectUri, state }) => app.getAuthCodeUrl({ scopes: qualified, redirectUri, state, nonce,
        codeChallenge: codes.challenge, codeChallengeMethod: "S256", ...(choose ? { prompt: "select_account" } : previous ? { loginHint: previous.username } : {}) }),
      open: url => ui("openUrl", { url }),
    });
    requireCurrent();
    const token = await app.acquireTokenByCode({ scopes: qualified, code: response.code, redirectUri: response.redirectUri, codeVerifier: codes.verifier });
    requireCurrent();
    if (!token?.account) throw new Error("Microsoft did not return an account for this application.");
    if (token.idTokenClaims?.nonce !== nonce) throw new Error("Microsoft returned an identity token for a different sign-in request.");
    if (!choose && previous && !selectedAccount([token.account], accountIdentity(previous))) {
      throw new Error("The selected account differs from the connected Microsoft account. Disconnect before switching accounts.");
    }
    await saved.select(token.account, app.getTokenCache().serialize());
    requireCurrent();
    selected = token.account; needsInteraction = false; restoreFailure = false;
    return selected;
  } catch (error) {
    // An optional-consent flow may have changed the SDK cache before nonce or
    // account validation. Restore the prior session only while it is current.
    // MSAL deserialize merges its input with in-memory entities. Clear through
    // the public API first so a rejected tenant/account leaves no new tokens.
    if (current()) { app.clearCache(); app.getTokenCache().deserialize(snapshot); selected = previous; }
    throw error;
  } finally { resumeWrites(); }
}

export async function connect(clientId = "") { await initialize(clientId); persistence.retry(); return interactive(BASE_SCOPES, { choose: true }); }
export async function reconnect() { await initialize(); persistence.retry(); return interactive(BASE_SCOPES); }
export async function requestScopes(scopes) {
  if (!application || !selected) throw new Error("Connect Microsoft before enabling this integration.");
  return interactive([...new Set([...BASE_SCOPES, ...scopes])]);
}

export async function accessToken(forceRefresh = false, scopes = BASE_SCOPES) {
  if (!application || !selected) throw new Error("Connect your Microsoft account before synchronizing.");
  try {
    const response = await application.acquireTokenSilent({ account: selected, scopes: graphScopes(scopes, settings.cloud), forceRefresh });
    if (!response?.accessToken) throw new Error("Microsoft did not return an access token.");
    if (scopes.every(scope => BASE_SCOPES.includes(scope))) needsInteraction = false;
    return response.accessToken;
  } catch (error) {
    const interaction = error?.name === "InteractionRequiredAuthError" || /interaction_required|consent_required|login_required|invalid_grant/i.test(error?.errorCode || error?.message || "");
    if (interaction && scopes.every(scope => BASE_SCOPES.includes(scope))) needsInteraction = true;
    const failure = new Error(interaction ? "Microsoft authorization needs interaction. Reconnect the account or enable the requested integration again." : "Microsoft token renewal failed. Reconnect your account and retry.");
    failure.requiresInteraction = interaction; failure.code = error?.errorCode; throw failure;
  }
}

export async function disconnect() {
  // The durable tombstone is written before cache cleanup. Failure propagates:
  // callers must not report a successful persistent disconnect without it.
  const saved = currentPersistence();
  saved?.revoke(); authEpoch++;
  const app = application, previous = selected;
  selected = undefined; application = undefined; needsInteraction = false;
  if (app && previous) await app.getTokenCache().removeAccount(previous);
  if (saved) await saved.forget();
  restoreFailure = false;
}

export function disposeSession() {
  authEpoch++; persistence?.dispose();
  selected = undefined; application = undefined; needsInteraction = false;
}

export async function logout() {
  const environment = serviceEnvironment(), { tenant } = await configuration();
  await disconnect();
  ui("openUrl", { url: `${environment.authority}/${tenant}/oauth2/v2.0/logout` });
}

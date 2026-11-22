import { randomUUID } from "node:crypto";
import { PublicClientApplication, CryptoProvider, LogLevel } from "./vendor/msal-node-7.2.3.mjs";
import { BASE_SCOPES, cloudEnvironment, graphScopes, requireCloudService } from "../web/cloud/environment.js";
import { receiveAuthorization, nativeRedirect } from "./loopback.mjs";
import { ui } from "./bridge.mjs";

export const SCOPES = BASE_SCOPES;
const KEY = "ferroui.todo.native.public-configuration";
const PERSONAL = "9188040d-6c67-4c5b-b112-36a304b66dad";
const GUID = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i;
let settings, application, selected, needsInteraction = false;
const cryptoProvider = new CryptoProvider();

function readConfiguration(clientId = "") {
  let saved = {};
  try { saved = JSON.parse(localStorage.getItem(KEY) || "{}"); } catch { /* Invalid saved public configuration is replaced by explicit settings. */ }
  const cloud = process.env.TODO_MICROSOFT_CLOUD || saved.cloud || "global";
  const environment = cloudEnvironment(cloud);
  const tenant = process.env.TODO_MICROSOFT_TENANT || saved.tenant || (cloud === "global" ? "common" : "organizations");
  if (!["common", "organizations", "consumers"].includes(tenant) && !GUID.test(tenant)) throw new Error("Microsoft tenant must be common, organizations, consumers, or a directory GUID.");
  if (cloud !== "global" && tenant === "consumers") throw new Error("National Microsoft clouds require an organizational account.");
  const id = clientId.trim() || process.env.TODO_MICROSOFT_CLIENT_ID || saved.clientId || "";
  if (id && !GUID.test(id)) throw new Error("Microsoft application client ID must be a GUID.");
  const redirectUri = nativeRedirect(process.env.TODO_MICROSOFT_NATIVE_REDIRECT_URI || "http://localhost/").href;
  return { clientId: id, tenant, cloud: environment.id, redirectUri, configured: !!id };
}

export async function configuration() { return settings || readConfiguration(); }
export function serviceEnvironment() { return cloudEnvironment((settings || readConfiguration()).cloud); }
export function account() { return selected || null; }
export function accountKey() { return selected ? (serviceEnvironment().id === "global" ? selected.homeAccountId : `${serviceEnvironment().id}:${selected.homeAccountId}`) : null; }
export function isWorkAccount() { return !!selected && GUID.test(selected.tenantId || "") && selected.tenantId.toLowerCase() !== PERSONAL; }
export function sessionStatus() { return { connected: !!selected, needsInteraction, username: selected?.username || "", displayName: selected?.name || selected?.username || "", accountType: selected ? isWorkAccount() ? "workOrSchool" : "personal" : null }; }
export async function restore() { return account(); }

async function initialize(clientId) {
  const next = readConfiguration(clientId);
  if (!next.clientId) throw new Error("Enter a registered Microsoft application's public client ID, or set TODO_MICROSOFT_CLIENT_ID.");
  requireCloudService(cloudEnvironment(next.cloud), "todo");
  if (application && JSON.stringify(next) === JSON.stringify(settings)) return;
  const environment = cloudEnvironment(next.cloud);
  application = new PublicClientApplication({ auth: { clientId: next.clientId, authority: `${environment.authority}/${next.tenant}` }, system: {
    loggerOptions: { piiLoggingEnabled: false, logLevel: LogLevel.Error, loggerCallback() {} },
  } });
  selected = undefined; needsInteraction = false; settings = next;
  localStorage.setItem(KEY, JSON.stringify(next));
}

async function interactive(scopes, { choose = false } = {}) {
  const previous = selected;
  const codes = await cryptoProvider.generatePkceCodes();
  const qualified = graphScopes(scopes, settings.cloud);
  const nonce = randomUUID();
  const response = await receiveAuthorization({ redirectUri: settings.redirectUri,
    authorize: ({ redirectUri, state }) => application.getAuthCodeUrl({ scopes: qualified, redirectUri, state, nonce,
      codeChallenge: codes.challenge, codeChallengeMethod: "S256", ...(choose ? { prompt: "select_account" } : previous ? { loginHint: previous.username } : {}) }),
    open: url => ui("openUrl", { url }),
  });
  const token = await application.acquireTokenByCode({ scopes: qualified, code: response.code, redirectUri: response.redirectUri, codeVerifier: codes.verifier });
  if (!token?.account) throw new Error("Microsoft did not return an account for this application.");
  if (token.idTokenClaims?.nonce !== nonce) {
    await application.getTokenCache().removeAccount(token.account);
    throw new Error("Microsoft returned an identity token for a different sign-in request.");
  }
  if (!choose && previous && previous.homeAccountId !== token.account.homeAccountId) {
    await application.getTokenCache().removeAccount(token.account);
    throw new Error("The selected account differs from the connected Microsoft account. Disconnect before switching accounts.");
  }
  selected = token.account; needsInteraction = false;
  return selected;
}

export async function connect(clientId = "") { await initialize(clientId); return interactive(BASE_SCOPES, { choose: true }); }
export async function reconnect() { await initialize(); return interactive(BASE_SCOPES); }
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
  const app = application, previous = selected;
  selected = undefined; application = undefined; needsInteraction = false;
  if (app && previous) await app.getTokenCache().removeAccount(previous);
}

export async function logout() {
  const environment = serviceEnvironment(), { tenant } = await configuration();
  await disconnect();
  ui("openUrl", { url: `${environment.authority}/${tenant}/oauth2/v2.0/logout` });
}

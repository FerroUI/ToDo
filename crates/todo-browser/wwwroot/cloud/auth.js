import { CLIENT_KEY, CONFIG_KEY, configuration, savedConfiguration, rememberConfiguration, resolveConfiguration } from "./auth-config.js";
import { ACCOUNT_KEY, hasAuthResponse, rememberReturn, finishReturn, forgetReturn } from "./auth-return.js";
import { authenticationError } from "./auth-errors.js";

// MSAL owns PKCE, state, nonce, token renewal, and credential storage. Public app
// configuration is separate from tokens and never contains a client secret.
export { configuration };
export const SCOPES = Object.freeze(["Tasks.ReadWrite", "User.Read"]);
let application;
let applicationConfiguration;
let initialization;
let initializationKey;
let library;
let generation = 0;
let needsInteraction = false;
const PERSONAL_TENANT = "9188040d-6c67-4c5b-b112-36a304b66dad";

function loadLibrary() {
  library ||= new Promise((resolve, reject) => {
    const script = document.createElement("script");
    const failed = () => { script.remove(); reject(Object.assign(new Error(), { code: "library_load_failed" })); };
    const timer = setTimeout(failed, 15_000);
    script.src = new URL("../vendor/msal-browser-5.30.0.min.js", import.meta.url).href;
    script.onload = () => {
      clearTimeout(timer);
      if (!globalThis.msal?.createStandardPublicClientApplication) return failed();
      resolve(globalThis.msal);
    };
    script.onerror = () => { clearTimeout(timer); failed(); };
    document.head.append(script);
  });
  return library.catch(error => { library = undefined; throw error; });
}

async function initialize(settings) {
  if (!settings?.clientId) throw new Error("Microsoft sign-in is not configured. Enter your Entra application's public client ID, or configure this deployment.");
  const key = JSON.stringify(settings);
  if (initialization) {
    if (initializationKey === key) return initialization;
    await initialization.catch(() => {});
  }
  if (application && JSON.stringify(applicationConfiguration) === key) return application;
  initializationKey = key;
  const expected = generation;
  initialization = (async () => {
    const msal = await loadLibrary();
    const app = await msal.createStandardPublicClientApplication({
      auth: {
        clientId: settings.clientId, authority: `https://login.microsoftonline.com/${settings.tenant}`,
        redirectUri: settings.redirectUri, postLogoutRedirectUri: settings.redirectUri,
      },
      cache: { cacheLocation: "sessionStorage" },
      system: { loggerOptions: { piiLoggingEnabled: false, logLevel: msal.LogLevel.Error, loggerCallback: () => {} } },
    });
    const callback = hasAuthResponse();
    let response;
    try { response = await app.handleRedirectPromise({ navigateToLoginRequestUrl: false }); }
    finally { if (callback || response) finishReturn(); }
    // Select only the account explicitly chosen by this app. Never silently pick
    // another cached account, including after switching to a different client ID.
    const id = sessionStorage.getItem(ACCOUNT_KEY);
    const selected = response?.account || app.getAllAccounts().find(item => item.homeAccountId === id) || null;
    if (generation !== expected) {
      // Disconnect can run while a redirect result or library initialization is
      // pending. Do not let its late completion restore the account or tokens.
      await app.clearCache(selected ? { account: selected } : undefined);
      throw Object.assign(new Error(), { code: "account_changed" });
    }
    app.setActiveAccount(selected);
    if (selected) sessionStorage.setItem(ACCOUNT_KEY, selected.homeAccountId);
    else sessionStorage.removeItem(ACCOUNT_KEY);
    application = app;
    applicationConfiguration = settings;
    needsInteraction = false;
    generation++;
    return app;
  })();
  try { return await initialization; }
  catch (error) { throw authenticationError(error); }
  finally { initialization = undefined; initializationKey = undefined; }
}

export async function restore() {
  const callback = hasAuthResponse();
  try {
    const settings = savedConfiguration();
    if (settings) { await initialize(settings); return account(); }
    // A first-time local launch does not load MSAL or start authentication. Load
    // public configuration for the UI and process a callback only when one exists.
    const deployed = await configuration();
    if (callback) {
      if (!deployed.configured) throw new Error("Microsoft sign-in returned without an application configuration. Connect again from Settings.");
      await initialize({ clientId: deployed.clientId, tenant: deployed.tenant, redirectUri: deployed.redirectUri });
      return account();
    }
    return null;
  } finally {
    // Configuration or script-loading errors occur before MSAL can clean a
    // callback. Do not leave response codes or error details in the address bar.
    if (callback && hasAuthResponse()) finishReturn();
  }
}

export async function connect(clientId = "") {
  const settings = await resolveConfiguration(clientId);
  const app = await initialize(settings);
  rememberConfiguration(settings);
  rememberReturn();
  try { await app.loginRedirect({ scopes: [...SCOPES], prompt: "select_account", redirectUri: settings.redirectUri }); }
  catch (error) { forgetReturn(); throw authenticationError(error); }
}

export async function reconnect() {
  const settings = applicationConfiguration || await resolveConfiguration();
  const app = await initialize(settings);
  const selected = app.getActiveAccount();
  if (!selected) return connect(settings.clientId);
  rememberConfiguration(settings);
  rememberReturn();
  try {
    await app.acquireTokenRedirect({ scopes: [...SCOPES], account: selected, loginHint: selected.username, redirectUri: settings.redirectUri });
  } catch (error) { forgetReturn(); throw authenticationError(error); }
}

export function account() { return application?.getActiveAccount() || null; }

function tenantId(selected) { return (selected?.tenantId || selected?.idTokenClaims?.tid || "").toLowerCase(); }

export function isWorkAccount() {
  const tenant = tenantId(account());
  return /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/.test(tenant) && tenant !== PERSONAL_TENANT;
}

export function sessionStatus() {
  const selected = account();
  return {
    connected: !!selected, needsInteraction,
    accountType: selected ? (tenantId(selected) === PERSONAL_TENANT ? "personal" : isWorkAccount() ? "workOrSchool" : "unknown") : null,
    username: selected?.username || "", displayName: selected?.name || selected?.username || "",
  };
}

export async function accessToken(forceRefresh = false) {
  const app = application, selected = account(), expected = generation;
  if (!app || !selected) throw new Error("Connect your Microsoft account before synchronizing.");
  try {
    const response = await app.acquireTokenSilent({
      scopes: [...SCOPES], account: selected, forceRefresh,
      // A 401 retry must bypass the cached access token; neither policy opens an
      // invisible sign-in iframe when the refresh token needs interaction.
      cacheLookupPolicy: forceRefresh ? globalThis.msal.CacheLookupPolicy.RefreshToken
        : globalThis.msal.CacheLookupPolicy.AccessTokenAndRefreshToken,
    });
    if (generation !== expected || application !== app || account()?.homeAccountId !== selected.homeAccountId) {
      throw Object.assign(new Error(), { code: "account_changed" });
    }
    needsInteraction = false;
    return response.accessToken;
  } catch (error) {
    const failure = authenticationError(error, "token renewal");
    if (generation === expected) needsInteraction = failure.requiresInteraction;
    throw failure;
  }
}

function forgetSession() {
  generation++;
  application = undefined;
  applicationConfiguration = undefined;
  needsInteraction = false;
  sessionStorage.removeItem(ACCOUNT_KEY);
  sessionStorage.removeItem(CLIENT_KEY);
  sessionStorage.removeItem(CONFIG_KEY);
  forgetReturn();
}

export async function disconnect() {
  const app = application, selected = account();
  forgetSession();
  try { if (app) await app.clearCache(selected ? { account: selected } : undefined); }
  catch (error) { throw authenticationError(error, "disconnect"); }
}

// Explicit Microsoft sign-out is separate from Disconnect, which leaves other
// Microsoft applications signed in. Call only from the user's sign-out action.
export async function logout() {
  const app = application, selected = account(), settings = applicationConfiguration;
  forgetSession();
  if (!app) return;
  try { await app.logoutRedirect({ account: selected || undefined, postLogoutRedirectUri: settings.redirectUri }); }
  catch (error) { throw authenticationError(error, "sign-out"); }
}

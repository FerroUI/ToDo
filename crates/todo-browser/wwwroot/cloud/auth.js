import { CLIENT_KEY, CONFIG_KEY, configuration as readConfiguration, savedConfiguration, rememberConfiguration, resolveConfiguration } from "./auth-config.js";
import { ACCOUNT_KEY, hasAuthResponse, rememberReturn, finishReturn, forgetReturn } from "./auth-return.js";
import { authenticationError } from "./auth-errors.js";
import { BASE_SCOPES, basePermissions, cloudEnvironment, graphScopes, requireCloudService } from "./environment.js";
import { declinedOptionalConsent, optionalScopes, rememberOptionalConsent, takeOptionalConsent, forgetOptionalConsent } from "./auth-optional.js";

// MSAL owns PKCE, state, nonce, token renewal, and credential storage. Public app
// configuration is separate from tokens and never contains a client secret.
export const SCOPES = BASE_SCOPES;
let application;
let applicationConfiguration;
let loadedConfiguration;
let initialization;
let initializationKey;
let library;
let generation = 0;
let needsInteraction = false;
const PERSONAL_TENANT = "9188040d-6c67-4c5b-b112-36a304b66dad";
const OPTIONAL_FAILURE = Symbol("optional Microsoft consent");

export async function configuration() {
  loadedConfiguration = await readConfiguration();
  return loadedConfiguration;
}

export function serviceEnvironment() {
  return cloudEnvironment(applicationConfiguration?.cloud || loadedConfiguration?.cloud || "global");
}

// Preserve existing global bindings while keeping identical subject IDs in
// independent sovereign directories from sharing sidecars or cached file data.
export function accountKey() {
  const id = account()?.homeAccountId;
  if (!id) return null;
  return serviceEnvironment().id === "global" ? id : `${serviceEnvironment().id}:${id}`;
}

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
  const environment = cloudEnvironment(settings.cloud || "global");
  requireCloudService(environment, "todo");
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
        clientId: settings.clientId, authority: `${environment.authority}/${settings.tenant}`,
        redirectUri: settings.redirectUri, postLogoutRedirectUri: settings.redirectUri,
      },
      cache: { cacheLocation: "sessionStorage" },
      system: { loggerOptions: { piiLoggingEnabled: false, logLevel: msal.LogLevel.Error, loggerCallback: () => {} } },
    });
    const callback = hasAuthResponse();
    const id = sessionStorage.getItem(ACCOUNT_KEY);
    const optional = callback ? takeOptionalConsent(settings, id) : null;
    let response, optionalFailure;
    try { response = await app.handleRedirectPromise({ navigateToLoginRequestUrl: false }); }
    catch (error) {
      if (!optional || !declinedOptionalConsent(error) || !app.getAllAccounts().some(item => item.homeAccountId === optional.account)) throw error;
      // Declining an extra permission must not disconnect tasks that already
      // have consent. Ignore callback identity data and retain only the previous
      // explicit account; the generation guard below still checks disconnect.
      optionalFailure = authenticationError(error, "connected service consent");
      optionalFailure.message = "The optional Microsoft service was not enabled. Your existing Microsoft task connection is still available; enable the service again from Settings to retry.";
      optionalFailure.optionalConsent = true;
      optionalFailure.optionalScopes = optional.scopes;
      optionalFailure[OPTIONAL_FAILURE] = true;
    }
    finally { if (callback || response) { forgetOptionalConsent(); finishReturn(); } }
    // Select only the account explicitly chosen by this app. Never silently pick
    // another cached account, including after switching to a different client ID.
    const selected = response?.account || app.getAllAccounts().find(item => item.homeAccountId === id) || null;
    if (selected && environment.id !== "global" && tenantId(selected) === PERSONAL_TENANT) {
      await app.clearCache({ account: selected });
      throw Object.assign(new Error(), { code: "national_cloud_personal_account" });
    }
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
    if (optionalFailure) throw optionalFailure;
    return app;
  })();
  try { return await initialization; }
  catch (error) { throw error?.[OPTIONAL_FAILURE] ? error : authenticationError(error); }
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
      await initialize({ clientId: deployed.clientId, tenant: deployed.tenant, redirectUri: deployed.redirectUri, cloud: deployed.cloud });
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
  forgetOptionalConsent();
  const settings = await resolveConfiguration(clientId);
  const app = await initialize(settings);
  rememberConfiguration(settings);
  rememberReturn();
  try { await app.loginRedirect({ scopes: graphScopes(SCOPES, settings.cloud), prompt: "select_account", redirectUri: settings.redirectUri }); }
  catch (error) { forgetReturn(); throw authenticationError(error); }
}

export async function reconnect() {
  forgetOptionalConsent();
  const settings = applicationConfiguration || await resolveConfiguration();
  const app = await initialize(settings);
  const selected = app.getActiveAccount();
  if (!selected) return connect(settings.clientId);
  rememberConfiguration(settings);
  rememberReturn();
  try {
    await app.acquireTokenRedirect({ scopes: graphScopes(SCOPES, settings.cloud), authority: `${serviceEnvironment().authority}/${settings.tenant}`, account: selected, loginHint: selected.username, redirectUri: settings.redirectUri });
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

// Optional features ask for consent only from an explicit user action. Their
// silent token failures must not suspend the already authorized To Do service.
export async function requestScopes(scopes) {
  const selected = account(), app = application, settings = applicationConfiguration;
  if (!app || !selected || !settings) throw new Error("Connect your Microsoft account before enabling this service.");
  const requested = optionalScopes(scopes, settings.cloud);
  rememberConfiguration(settings);
  rememberReturn();
  rememberOptionalConsent(settings, selected.homeAccountId, requested);
  try {
    await app.acquireTokenRedirect({ scopes: requested, authority: `${serviceEnvironment().authority}/${settings.tenant}`, account: selected, loginHint: selected.username, redirectUri: settings.redirectUri });
  } catch (error) { forgetOptionalConsent(); forgetReturn(); throw authenticationError(error, "service consent"); }
}

export async function accessToken(forceRefresh = false, scopes = SCOPES) {
  const app = application, selected = account(), expected = generation;
  if (!app || !selected) throw new Error("Connect your Microsoft account before synchronizing.");
  const cloud = serviceEnvironment().id;
  const requested = graphScopes(scopes, cloud);
  const base = basePermissions(scopes, cloud);
  try {
    const response = await app.acquireTokenSilent({
      scopes: requested, authority: `${serviceEnvironment().authority}/${applicationConfiguration.tenant}`, account: selected, forceRefresh,
      // A 401 retry must bypass the cached access token; neither policy opens an
      // invisible sign-in iframe when the refresh token needs interaction.
      cacheLookupPolicy: forceRefresh ? globalThis.msal.CacheLookupPolicy.RefreshToken
        : globalThis.msal.CacheLookupPolicy.AccessTokenAndRefreshToken,
    });
    if (generation !== expected || application !== app || account()?.homeAccountId !== selected.homeAccountId) {
      throw Object.assign(new Error(), { code: "account_changed" });
    }
    if (base) needsInteraction = false;
    return response.accessToken;
  } catch (error) {
    const failure = authenticationError(error, "token renewal");
    if (base && generation === expected) needsInteraction = failure.requiresInteraction;
    if (!base && failure.requiresInteraction) failure.message = "This optional Microsoft service needs consent. Enable it again from Settings; task synchronization can continue.";
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
  forgetOptionalConsent();
  forgetReturn();
}

// A local restore revokes automatic session selection before its durable write.
// It retains the in-memory session until the serialized restore has completed.
export function revokeResume() { sessionStorage.removeItem(ACCOUNT_KEY); }

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

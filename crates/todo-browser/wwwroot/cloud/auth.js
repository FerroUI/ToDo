// Microsoft owns the OAuth implementation: PKCE, state, nonce, renewal and token caching.
// Only sessionStorage contains credentials; documents and backups never contain tokens.
const CLIENT_KEY = "ferroui.todo.microsoft.client";
const ACCOUNT_KEY = "ferroui.todo.microsoft.account";
const RETURN_QUERY_KEY = "ferroui.todo.microsoft.return-query";
const SCOPES = ["Tasks.ReadWrite", "User.Read"];
let application;
let applicationId;
let library;

function loadLibrary() {
  library ||= new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = new URL("../vendor/msal-browser-5.30.0.min.js", import.meta.url).href;
    script.onload = () => resolve(globalThis.msal);
    script.onerror = () => { library = undefined; reject(new Error("Could not load Microsoft sign-in.")); };
    document.head.append(script);
  });
  return library;
}

async function initialize(clientId) {
  if (!/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(clientId)) {
    throw new Error("Enter the Application (client) ID of your Microsoft Entra SPA registration.");
  }
  if (application && applicationId === clientId) return application;
  const msal = await loadLibrary();
  application = await msal.createStandardPublicClientApplication({
    auth: { clientId, authority: "https://login.microsoftonline.com/common", redirectUri: `${location.origin}${location.pathname}` },
    cache: { cacheLocation: "sessionStorage" },
    system: { loggerOptions: { piiLoggingEnabled: false, logLevel: msal.LogLevel.Error, loggerCallback: () => {} } },
  });
  applicationId = clientId;
  const response = await application.handleRedirectPromise({ navigateToLoginRequestUrl: false });
  if (response?.account) {
    application.setActiveAccount(response.account);
    sessionStorage.setItem(ACCOUNT_KEY, response.account.homeAccountId);
    const query = sessionStorage.getItem(RETURN_QUERY_KEY);
    if (query !== null) {
      history.replaceState(null, "", `${location.pathname}${query}`);
      sessionStorage.removeItem(RETURN_QUERY_KEY);
    }
  } else {
    const id = sessionStorage.getItem(ACCOUNT_KEY);
    const account = application.getAllAccounts().find(item => item.homeAccountId === id);
    if (account) application.setActiveAccount(account);
  }
  return application;
}

export async function restore() {
  const clientId = sessionStorage.getItem(CLIENT_KEY);
  if (!clientId) return null;
  await initialize(clientId);
  return account();
}

export async function connect(clientId) {
  clientId = clientId.trim();
  const app = await initialize(clientId);
  sessionStorage.setItem(CLIENT_KEY, clientId);
  const query = new URLSearchParams(location.search);
  for (const key of ["code", "state", "session_state", "error", "error_description"]) query.delete(key);
  sessionStorage.setItem(RETURN_QUERY_KEY, query.size ? `?${query}` : "");
  await app.loginRedirect({ scopes: SCOPES, prompt: "select_account" });
}

export function account() { return application?.getActiveAccount() || null; }

export async function accessToken(forceRefresh = false) {
  if (!account()) throw new Error("Connect your Microsoft account before synchronizing.");
  try {
    const response = await application.acquireTokenSilent({
      scopes: SCOPES, account: account(), forceRefresh,
      // An expired refresh token requires an explicit reconnect, never an invisible login iframe.
      cacheLookupPolicy: globalThis.msal.CacheLookupPolicy.AccessTokenAndRefreshToken,
    });
    return response.accessToken;
  } catch {
    throw new Error("Your Microsoft session needs sign-in again. Choose Connect Microsoft account to continue.");
  }
}

export async function disconnect() {
  if (application) await application.clearCache({ account: account() });
  application = undefined;
  applicationId = undefined;
  sessionStorage.removeItem(ACCOUNT_KEY);
  sessionStorage.removeItem(CLIENT_KEY);
  sessionStorage.removeItem(RETURN_QUERY_KEY);
}

import { graphScopes, cloudEnvironment } from "./environment.js";

export const OPTIONAL_CONSENT_KEY = "ferroui.todo.microsoft.optional-consent";
const OPTIONAL = new Set(["MailboxSettings.Read", "MailboxSettings.ReadWrite", "Files.ReadWrite.AppFolder"]);
const configurationKey = settings => JSON.stringify([
  settings.clientId.toLowerCase(), settings.tenant.toLowerCase(), settings.cloud || "global", settings.redirectUri,
]);

export function optionalScopes(scopes, cloud = "global") {
  const normalized = graphScopes(scopes, cloud), environment = cloudEnvironment(cloud);
  if (normalized.some(scope => !OPTIONAL.has(scope.startsWith(`${environment.graph}/`) ? scope.slice(environment.graph.length + 1) : scope))) {
    throw new Error("Use Reconnect to renew task permissions; optional consent accepts only connected-service permissions.");
  }
  return normalized;
}

export function rememberOptionalConsent(settings, account, scopes) {
  if (!account || typeof account !== "string") throw new Error("Select a Microsoft account before enabling a connected service.");
  sessionStorage.setItem(OPTIONAL_CONSENT_KEY, JSON.stringify({ version: 1, configuration: configurationKey(settings), account,
    scopes: optionalScopes(scopes, settings.cloud), createdAt: Date.now() }));
}

export function forgetOptionalConsent() { sessionStorage.removeItem(OPTIONAL_CONSENT_KEY); }

// Consume only when processing a redirect. A marker never selects an account;
// it can only authorize retaining the account already explicitly selected.
export function takeOptionalConsent(settings, selectedAccount, now = Date.now()) {
  const json = sessionStorage.getItem(OPTIONAL_CONSENT_KEY);
  forgetOptionalConsent();
  if (!json || json.length > 4096) return null;
  try {
    const value = JSON.parse(json);
    if (value.version !== 1 || value.configuration !== configurationKey(settings) || value.account !== selectedAccount || !selectedAccount
      || !Number.isFinite(value.createdAt) || value.createdAt > now + 60_000 || now - value.createdAt > 30 * 60_000) return null;
    return { account: value.account, scopes: optionalScopes(value.scopes, settings.cloud) };
  } catch { return null; }
}

export const declinedOptionalConsent = error => ["access_denied", "user_cancelled", "user_canceled"].includes(error?.errorCode || error?.code);

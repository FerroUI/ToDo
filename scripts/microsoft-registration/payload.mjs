import { cloudEnvironment, requireCloudService } from "../../crates/todo-browser/wwwroot/cloud/environment.js";

export const GRAPH_APPLICATION_ID = "00000003-0000-0000-c000-000000000000";
export const REGISTRATION_SCOPES = Object.freeze(["Tasks.ReadWrite", "User.Read", "MailboxSettings.Read", "Files.ReadWrite.AppFolder"]);
const GUID = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i;
const AUDIENCES = Object.freeze({ both: "AzureADandPersonalMicrosoftAccount", organizations: "AzureADMultipleOrgs", tenant: "AzureADMyOrg", personal: "PersonalMicrosoftAccount" });

export function requireGuid(value, label) {
  if (typeof value !== "string" || !GUID.test(value)) throw new Error(`${label} must be a GUID.`);
  return value.toLowerCase();
}

export function registrationSettings(options) {
  const environment = cloudEnvironment(options.cloud || "global");
  requireCloudService(environment, "todo");
  const tenant = requireGuid(options.tenant, "The explicitly selected --tenant");
  const displayName = options.displayName || "FerroUI To Do";
  if (typeof displayName !== "string" || !displayName.trim() || displayName.length > 256 || /[\u0000-\u001f\u007f]/.test(displayName)) throw new Error("Application display name must contain 1–256 printable characters.");
  const accounts = options.accounts || (environment.id === "global" ? "both" : "organizations");
  if (!Object.hasOwn(AUDIENCES, accounts)) throw new Error("--accounts must be both, organizations, tenant, or personal.");
  if (environment.id !== "global" && ["both", "personal"].includes(accounts)) throw new Error("National cloud registrations support organizational accounts only.");
  const normalizeUri = (value, native) => {
    let uri;
    try { uri = new URL(value); } catch { throw new Error("Redirects must be absolute URIs."); }
    const loopback = ["localhost", "127.0.0.1", "[::1]"].includes(uri.hostname);
    if (uri.username || uri.password || uri.search || uri.hash || uri.href.length > 2048
      || (native ? uri.protocol !== "http:" || !loopback : uri.protocol !== "https:" && !(uri.protocol === "http:" && loopback))) {
      throw new Error(native ? "Native redirects must use HTTP loopback without credentials, query, or fragment." : "SPA redirects must use HTTPS (or local loopback HTTP) without credentials, query, or fragment.");
    }
    if (native && uri.hostname !== "localhost" && !uri.port) throw new Error("Native IP-loopback redirects require an explicit fixed port; use http://localhost/ for an OS-assigned port.");
    if (!native && uri.pathname.endsWith("/index.html")) throw new Error("Register the canonical SPA directory URI instead of index.html.");
    return uri.href;
  };
  const redirects = (values, native) => {
    if (!Array.isArray(values) || !values.length || values.length > 32 || values.some(value => typeof value !== "string")) throw new Error("Provide between 1 and 32 redirect URIs per platform.");
    return [...new Set(values.map(value => normalizeUri(value, native)))];
  };
  return { environment, tenant, displayName: displayName.trim(), accounts, audience: AUDIENCES[accounts],
    spaRedirects: redirects(options.spaRedirects || ["https://ferroui.com/ToDo/"], false),
    nativeRedirects: redirects(options.nativeRedirects || ["http://localhost/"], true),
    applicationId: options.applicationId ? requireGuid(options.applicationId, "--application-id") : null };
}

export function validateAzureContext(settings, azureCloud, account) {
  const expected = settings.environment.id === "global" ? "AzureCloud" : "AzureUSGovernment";
  let authority;
  try { authority = new URL(azureCloud.endpoints.activeDirectory).origin; } catch { throw new Error("Azure CLI returned an invalid identity endpoint."); }
  if (azureCloud.name !== expected || authority !== settings.environment.authority) throw new Error(`Azure CLI must already use ${expected} and its official identity endpoint.`);
  if (requireGuid(account.tenantId, "Azure CLI account tenant") !== settings.tenant) throw new Error("Azure CLI is signed into a different tenant. Select the requested tenant before continuing.");
}

export function registrationPayload(settings, principal, existing) {
  if (principal?.appId !== GRAPH_APPLICATION_ID || !Array.isArray(principal.oauth2PermissionScopes)) throw new Error("Microsoft Graph's service principal could not be verified.");
  const permissions = REGISTRATION_SCOPES.map(name => {
    const matches = principal.oauth2PermissionScopes.filter(scope => scope.value === name && scope.isEnabled === true);
    if (matches.length !== 1) throw new Error(`Microsoft Graph does not expose exactly one enabled delegated ${name} scope in this tenant.`);
    return { id: requireGuid(matches[0].id, `${name} scope ID`), type: "Scope" };
  });
  // An explicitly selected existing registration keeps its unrelated platforms
  // and permissions. No consent grants, credentials, or secrets are created.
  const resourceAccess = structuredClone(existing?.requiredResourceAccess || []);
  let graph = resourceAccess.find(resource => resource.resourceAppId === GRAPH_APPLICATION_ID);
  if (!graph) resourceAccess.push(graph = { resourceAppId: GRAPH_APPLICATION_ID, resourceAccess: [] });
  graph.resourceAccess ||= [];
  for (const permission of permissions) if (!graph.resourceAccess.some(item => item.id === permission.id && item.type === "Scope")) graph.resourceAccess.push(permission);
  return {
    displayName: settings.displayName,
    signInAudience: settings.audience,
    ...(["both", "personal"].includes(settings.accounts) ? { api: { ...existing?.api, requestedAccessTokenVersion: 2 } } : {}),
    spa: { ...existing?.spa, redirectUris: [...new Set([...(existing?.spa?.redirectUris || []), ...settings.spaRedirects])] },
    publicClient: { ...existing?.publicClient, redirectUris: [...new Set([...(existing?.publicClient?.redirectUris || []), ...settings.nativeRedirects])] },
    requiredResourceAccess: resourceAccess,
  };
}

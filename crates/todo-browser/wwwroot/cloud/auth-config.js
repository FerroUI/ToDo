const GUID = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i;
const TENANTS = new Set(["common", "organizations", "consumers"]);
export const CLIENT_KEY = "ferroui.todo.microsoft.client";
export const CONFIG_KEY = "ferroui.todo.microsoft.configuration";
let deployment;

export function normalizeConfiguration(value = {}, address = location) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Microsoft sign-in configuration must be a JSON object.");
  if (Object.keys(value).some(key => !["clientId", "tenant", "redirectUri"].includes(key))) {
    throw new Error("Microsoft sign-in configuration accepts only the public clientId, tenant, and redirectUri. Never put a client secret in this app.");
  }
  const string = (name, fallback) => {
    if (value[name] == null) return fallback;
    if (typeof value[name] !== "string") throw new Error(`Microsoft ${name} must be text.`);
    return value[name].trim();
  };
  const clientId = string("clientId", "");
  if (clientId && !GUID.test(clientId)) throw new Error("Enter the Application (client) ID of your Microsoft Entra SPA registration.");
  const tenant = string("tenant", "common").toLowerCase() || "common";
  if (!TENANTS.has(tenant) && !GUID.test(tenant)) throw new Error("Microsoft tenant must be common, organizations, consumers, or a tenant ID.");
  const current = new URL(address.pathname, address.origin);
  current.pathname = current.pathname.replace(/\/index\.html$/, "/");
  const redirectUri = string("redirectUri", "") || current.href;
  let redirect;
  try { redirect = new URL(redirectUri); } catch { throw new Error("Microsoft redirect URI must be an absolute URL."); }
  const development = ["localhost", "127.0.0.1", "[::1]"].includes(redirect.hostname);
  if ((redirect.protocol !== "https:" && !(development && redirect.protocol === "http:")) || redirect.username || redirect.password
    || redirect.origin !== current.origin || redirect.pathname !== current.pathname || redirect.search || redirect.hash) {
    throw new Error(`Register this app's exact SPA redirect URI: ${current.href}`);
  }
  return { clientId, tenant, redirectUri: redirect.href };
}

export function deploymentConfiguration() {
  deployment ||= (async () => {
    const response = await fetch(new URL("../microsoft-config.json", import.meta.url), {
      credentials: "omit", redirect: "error", cache: "no-cache", signal: AbortSignal.timeout(5_000),
    });
    if (response.status === 404) return normalizeConfiguration();
    if (!response.ok) throw new Error("Could not load this deployment's Microsoft sign-in configuration. Try again when online.");
    let value;
    try { value = await response.json(); } catch { throw new Error("Microsoft sign-in configuration is not valid JSON."); }
    return normalizeConfiguration(value);
  })();
  return deployment.catch(error => { deployment = undefined; throw error; });
}

export function savedConfiguration() {
  const saved = sessionStorage.getItem(CONFIG_KEY);
  if (saved) {
    let value;
    try { value = JSON.parse(saved); } catch { throw new Error("The saved Microsoft sign-in configuration is invalid. Connect your account again."); }
    return normalizeConfiguration(value);
  }
  const clientId = sessionStorage.getItem(CLIENT_KEY);
  return clientId ? normalizeConfiguration({ clientId }) : null;
}

export function rememberConfiguration(configuration) {
  sessionStorage.setItem(CONFIG_KEY, JSON.stringify(configuration));
  sessionStorage.setItem(CLIENT_KEY, configuration.clientId);
}

export async function configuration() {
  const saved = savedConfiguration();
  const value = saved || await deploymentConfiguration();
  return { ...value, configured: !!value.clientId, source: saved ? "session" : "deployment" };
}

export async function resolveConfiguration(clientId) {
  const override = typeof clientId === "string" ? clientId.trim() : "";
  let saved;
  try { saved = savedConfiguration(); }
  catch (error) { if (!override) throw error; }
  if (saved && (!override || saved.clientId.toLowerCase() === override.toLowerCase())) return saved;
  let configured;
  try { configured = await deploymentConfiguration(); }
  catch (error) { if (!override) throw error; }
  if (!override || configured?.clientId.toLowerCase() === override.toLowerCase()) return configured;
  // A user's separate registration must not inherit another application's tenant restriction.
  return normalizeConfiguration({ clientId: override });
}

// National-cloud identity and resource origins are a fixed pair. Never accept
// deployment-supplied hosts or follow another cloud's Graph continuation URL.
const ENVIRONMENTS = Object.freeze({
  global: Object.freeze({ id: "global", label: "Microsoft global", authority: "https://login.microsoftonline.com", graph: "https://graph.microsoft.com", todo: true, planner: true, categories: true, presentation: true,
    todoUrl: "https://to-do.office.com/tasks/", todoLabel: "Open Microsoft To Do", plannerUrl: "https://planner.cloud.microsoft/", plannerLabel: "Open in Planner" }),
  usgov: Object.freeze({ id: "usgov", label: "US Government GCC High", authority: "https://login.microsoftonline.us", graph: "https://graph.microsoft.us", todo: true, planner: true, categories: true, presentation: true,
    todoUrl: "https://outlook.office365.us/", todoLabel: "Open To Do in Outlook", plannerUrl: "https://tasks.office365.us/", plannerLabel: "Open in Planner" }),
  usgovdod: Object.freeze({ id: "usgovdod", label: "US Government DoD", authority: "https://login.microsoftonline.us", graph: "https://dod-graph.microsoft.us", todo: true, planner: true, categories: true, presentation: true,
    todoUrl: "https://outlook-dod.office365.us/", todoLabel: "Open To Do in Outlook", plannerUrl: "https://portal.apps.mil/", plannerLabel: "Open Microsoft 365" }),
  china: Object.freeze({ id: "china", label: "China operated by 21Vianet", authority: "https://login.chinacloudapi.cn", graph: "https://microsoftgraph.chinacloudapi.cn", todo: false, planner: false, categories: false, presentation: false,
    todoUrl: null, todoLabel: "Microsoft To Do unavailable", plannerUrl: null, plannerLabel: "Planner unavailable" }),
});
export const BASE_SCOPES = Object.freeze(["Tasks.ReadWrite", "User.Read"]);
const PERMISSIONS = new Set([...BASE_SCOPES, "MailboxSettings.Read", "Files.ReadWrite.AppFolder"]);

export function cloudEnvironment(value = "global") {
  if (typeof value !== "string" || !Object.hasOwn(ENVIRONMENTS, value.trim().toLowerCase())) {
    throw new Error("Microsoft cloud must be global, usgov, usgovdod, or china.");
  }
  return ENVIRONMENTS[value.trim().toLowerCase()];
}

export function graphScopes(scopes = BASE_SCOPES, cloud = "global") {
  const environment = cloudEnvironment(cloud);
  if (!Array.isArray(scopes) || !scopes.length || scopes.length > PERMISSIONS.size) throw new Error("Select supported Microsoft Graph permissions.");
  const unique = new Set();
  for (const value of scopes) {
    if (typeof value !== "string") throw new Error("Microsoft Graph permissions must be text.");
    const name = value.startsWith(`${environment.graph}/`) ? value.slice(environment.graph.length + 1) : value;
    if (!PERMISSIONS.has(name)) throw new Error("This Microsoft Graph permission is not supported by the application.");
    unique.add(name);
  }
  // Short permission names are valid in every Microsoft cloud. Keep existing
  // global consent unchanged; explicitly name the resource in national clouds.
  return [...unique].map(scope => environment.id === "global" ? scope : `${environment.graph}/${scope}`);
}

export function basePermissions(scopes, cloud = "global") {
  const normalized = graphScopes(scopes, cloud);
  const base = graphScopes(BASE_SCOPES, cloud);
  return normalized.length === base.length && base.every(scope => normalized.includes(scope));
}

export function requireCloudService(environment, service) {
  if (!environment[service]) {
    const error = new Error(`Microsoft does not expose this ${service === "todo" ? "To Do" : service === "categories" ? "Outlook category" : service} service in ${environment.label}. Choose a supported deployment; local tasks remain available.`);
    error.code = "cloud_service_unavailable";
    throw error;
  }
}

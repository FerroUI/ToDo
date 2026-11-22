import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { GRAPH_APPLICATION_ID, registrationPayload, validateAzureContext, requireGuid } from "./payload.mjs";

const execute = promisify(execFile);
export async function azureJson(args) {
  // argv is passed directly, never through a shell. Azure CLI owns its token
  // cache and injects Authorization; the helper never reads or prints tokens.
  const { stdout } = await execute("az", [...args, "--only-show-errors", "--output", "json"], { maxBuffer: 8 * 1024 * 1024 });
  return stdout.trim() ? JSON.parse(stdout) : null;
}

export async function planRegistration(settings, run = azureJson) {
  const [cloud, account] = await Promise.all([run(["cloud", "show"]), run(["account", "show"])]);
  validateAzureContext(settings, cloud, account);
  const root = `${settings.environment.graph}/v1.0/`;
  const request = async (path, method = "get", body) => {
    const url = new URL(path, root);
    if (url.origin !== settings.environment.graph || !url.pathname.startsWith("/v1.0/") || url.username || url.password || url.hash) throw new Error("Unexpected registration API URL.");
    const args = ["rest", "--method", method, "--url", url.href, "--resource", settings.environment.graph];
    if (body !== undefined) args.push("--headers", "Content-Type=application/json", "--body", JSON.stringify(body));
    return run(args);
  };
  const serviceQuery = new URLSearchParams({ "$filter": `appId eq '${GRAPH_APPLICATION_ID}'`, "$select": "appId,oauth2PermissionScopes" });
  const response = await request(`servicePrincipals?${serviceQuery}`);
  if (response?.value?.length !== 1 || response["@odata.nextLink"]) throw new Error("The selected tenant did not return one Microsoft Graph service principal.");
  let existing;
  if (settings.applicationId) {
    existing = await request(`applications(appId='${settings.applicationId}')?$select=id,appId,displayName,signInAudience,spa,publicClient,requiredResourceAccess,api`);
    requireGuid(existing?.id, "Existing application object ID");
    if (existing.appId?.toLowerCase() !== settings.applicationId) throw new Error("The application lookup returned another client ID.");
  } else {
    const query = new URLSearchParams({ "$filter": `displayName eq '${settings.displayName.replaceAll("'", "''")}'`, "$select": "appId,displayName" });
    const matches = await request(`applications?${query}`);
    if (!Array.isArray(matches?.value) || matches.value.length || matches["@odata.nextLink"]) throw new Error("An application with this name already exists, or the lookup was incomplete. Review it and supply its explicit --application-id to update it.");
  }
  const body = registrationPayload(settings, response.value[0], existing);
  return {
    review: { operation: existing ? "update" : "create", tenant: settings.tenant, cloud: settings.environment.id, applicationId: existing?.appId || null, registration: body,
      note: "Review only unless --apply is supplied. Configured scopes do not grant user or administrator consent. No client secret is created." },
    async apply() {
      // No automatic POST retry: an uncertain create must be reconciled using
      // the explicit application ID instead of producing a duplicate app.
      const created = existing ? (await request(`applications/${existing.id}`, "patch", body), existing) : await request("applications", "post", body);
      const clientId = requireGuid(created?.appId, "Created application client ID");
      const config = { clientId, tenant: settings.accounts === "tenant" ? settings.tenant : settings.accounts === "personal" ? "consumers" : settings.accounts === "both" ? "common" : "organizations", redirectUri: settings.spaRedirects[0], cloud: settings.environment.id };
      return { applicationId: clientId, objectId: requireGuid(created.id, "Application object ID"), configuration: config,
        repositoryVariables: { MICROSOFT_CLIENT_ID: clientId, MICROSOFT_TENANT: config.tenant, MICROSOFT_REDIRECT_URI: config.redirectUri, MICROSOFT_CLOUD: config.cloud },
        nativeRedirectUri: settings.nativeRedirects[0] };
    },
  };
}

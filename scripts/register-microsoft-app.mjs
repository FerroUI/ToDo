import { parseArgs } from "node:util";
import { writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import path from "node:path";
import { registrationSettings } from "./microsoft-registration/payload.mjs";
import { planRegistration } from "./microsoft-registration/azure.mjs";

export async function registerMicrosoftApp(args = process.argv.slice(2)) {
  const { values } = parseArgs({ args, options: {
    tenant: { type: "string" }, cloud: { type: "string", default: "global" },
    "display-name": { type: "string" }, accounts: { type: "string" },
    "spa-uri": { type: "string", multiple: true }, "native-uri": { type: "string", multiple: true },
    "application-id": { type: "string" }, output: { type: "string" },
    apply: { type: "boolean", default: false }, help: { type: "boolean", default: false },
  } });
  if (values.help) {
    console.log(`Prepare an owner-controlled Entra application for FerroUI To Do.

Prerequisite: Azure CLI is already signed into the intended tenant/cloud.
This helper does not run az login, switch clouds, grant consent, or create secrets.

Review:
  node scripts/register-microsoft-app.mjs --tenant TENANT_GUID
Apply the reviewed registration:
  node scripts/register-microsoft-app.mjs --tenant TENANT_GUID --apply
Update an explicitly selected existing application:
  node scripts/register-microsoft-app.mjs --tenant TENANT_GUID --application-id CLIENT_GUID --apply

Options:
  --cloud global|usgov|usgovdod   Defaults to global; China To Do is unsupported.
  --display-name NAME           Defaults to FerroUI To Do.
  --accounts both|organizations|tenant|personal
  --spa-uri URL                 Repeat to add SPA redirects; default https://ferroui.com/ToDo/
  --native-uri URL              Repeat to add public-client redirects; default http://localhost/
  --output PATH                 Write public configuration after --apply.

Existing unrelated redirects/permissions are retained. Rerun with --application-id
after creation; duplicate display names are never automatically adopted.`);
    return;
  }
  if (values.output && !values.apply) throw new Error("--output requires --apply; review does not modify files or registrations.");
  const settings = registrationSettings({ tenant: values.tenant, cloud: values.cloud, displayName: values["display-name"], accounts: values.accounts,
    spaRedirects: values["spa-uri"], nativeRedirects: values["native-uri"], applicationId: values["application-id"] });
  const plan = await planRegistration(settings);
  if (!values.apply) { console.log(JSON.stringify(plan.review, null, 2)); return; }
  console.error(JSON.stringify(plan.review, null, 2));
  const result = await plan.apply();
  console.log(JSON.stringify(result, null, 2));
  if (values.output) await writeFile(path.resolve(values.output), `${JSON.stringify(result.configuration, null, 2)}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  registerMicrosoftApp().catch(error => { console.error(error.message); process.exitCode = 1; });
}

import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { normalizeConfiguration } from "../crates/todo-browser/wwwroot/cloud/auth-config.js";

export async function configureMicrosoft(root, env = process.env) {
  const file = path.join(root, "microsoft-config.json");
  const settings = JSON.parse(await readFile(file, "utf8"));
  for (const [field, name] of [["clientId", "TODO_MICROSOFT_CLIENT_ID"], ["tenant", "TODO_MICROSOFT_TENANT"], ["redirectUri", "TODO_MICROSOFT_REDIRECT_URI"], ["cloud", "TODO_MICROSOFT_CLOUD"]]) {
    if (env[name]?.trim()) settings[field] = env[name].trim();
  }
  // Use the same allowlist and cloud pairing as the runtime. An unspecified
  // redirect stays empty so installations can derive their actual origin.
  const address = settings.redirectUri ? new URL(settings.redirectUri) : new URL("https://configuration.invalid/");
  const normalized = normalizeConfiguration(settings, address);
  normalized.redirectUri = settings.redirectUri?.trim() || "";
  await writeFile(file, `${JSON.stringify(normalized, null, 2)}\n`);
  return !!normalized.clientId;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const configured = await configureMicrosoft(path.resolve(process.argv[2] || "target/browser/todo-browser"));
  console.log(configured ? "Microsoft sign-in public configuration included." : "Microsoft sign-in supports a client ID supplied in Settings.");
}

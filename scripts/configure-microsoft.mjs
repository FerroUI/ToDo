import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

export async function configureMicrosoft(root, env = process.env) {
  const file = path.join(root, "microsoft-config.json");
  const settings = JSON.parse(await readFile(file, "utf8"));
  for (const [field, name] of [["clientId", "TODO_MICROSOFT_CLIENT_ID"], ["tenant", "TODO_MICROSOFT_TENANT"], ["redirectUri", "TODO_MICROSOFT_REDIRECT_URI"]]) {
    if (env[name]?.trim()) settings[field] = env[name].trim();
  }
  const guid = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i;
  if (settings.clientId && !guid.test(settings.clientId)) throw new Error("MICROSOFT_CLIENT_ID must be the public Entra application GUID.");
  if (!["common", "organizations", "consumers"].includes(settings.tenant) && !guid.test(settings.tenant)) throw new Error("MICROSOFT_TENANT must be common, organizations, consumers, or a tenant GUID.");
  if (settings.redirectUri) {
    const url = new URL(settings.redirectUri);
    if ((url.protocol !== "https:" && !(url.protocol === "http:" && ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname)))
      || url.username || url.password || url.search || url.hash) throw new Error("MICROSOFT_REDIRECT_URI must be the canonical HTTPS app address without query or fragment.");
  }
  await writeFile(file, `${JSON.stringify(settings, null, 2)}\n`);
  return !!settings.clientId;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const configured = await configureMicrosoft(path.resolve(process.argv[2] || "target/browser/todo-browser"));
  console.log(configured ? "Microsoft sign-in public configuration included." : "Microsoft sign-in supports a client ID supplied in Settings.");
}

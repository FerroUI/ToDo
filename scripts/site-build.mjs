import { createHash } from "node:crypto";
import { copyFile, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";

const WORKER = new URL("../crates/todo-browser/wwwroot/app-sw.js", import.meta.url);
const excluded = new Set(["_app", "qa", "test-results", ".nojekyll", "app-sw.js"]);
const sha256 = contents => createHash("sha256").update(contents).digest("base64");
const urlPath = name => name.split(path.sep).map(encodeURIComponent).join("/");

async function files(root, directory = root) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async entry => {
    if (excluded.has(entry.name) || entry.name.endsWith(".map")) return [];
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) return files(root, file);
    if (!entry.isFile()) throw new Error(`Unsupported site entry: ${file}`);
    return [path.relative(root, file)];
  }));
  return nested.flat();
}

export async function prepareSite(root, workerTemplate) {
  const names = (await files(root)).sort();
  const contents = new Map(await Promise.all(names.map(async name => [name, await readFile(path.join(root, name))])));
  for (const required of ["bootstrap.js", "offline.js", "main.js", "todo-browser.js", "todo_browser.wasm", "ferroui.js", "ferroui-loader.js", "app.css"]) {
    if (!contents.has(required)) throw new Error(`The built site is missing ${required}.`);
  }
  let index = contents.get("index.html")?.toString();
  if (!index) throw new Error("The built site has no index.html.");
  const previous = /data-todo-build="([a-f0-9]{16}|__BUILD_VERSION__)"/.exec(index)?.[1];
  if (!previous) throw new Error("The site index must declare data-todo-build.");
  index = index.replaceAll(previous, "__BUILD_VERSION__");
  contents.set("index.html", Buffer.from(index));
  const worker = workerTemplate ?? await readFile(WORKER, "utf8");
  // Include worker policy in the revision, and length-separate every input.
  const hash = createHash("sha256").update(worker);
  for (const [name, bytes] of contents) hash.update(`\0${name}\0${bytes.length}\0`).update(bytes);
  const version = hash.digest("hex").slice(0, 16);
  const assetRoot = path.join(root, "_app", version);
  await rm(path.join(root, "_app"), { recursive: true, force: true });
  const assets = [];
  for (const [name, bytes] of contents) {
    if (name === "index.html") continue;
    const file = path.join(assetRoot, name);
    await mkdir(path.dirname(file), { recursive: true });
    await copyFile(path.join(root, name), file);
    assets.push([`./_app/${version}/${urlPath(name)}`, `sha256-${sha256(bytes)}`]);
  }
  index = index.replaceAll("__BUILD_VERSION__", version);
  await writeFile(path.join(root, "index.html"), index);
  assets.push(["./index.html", `sha256-${sha256(index)}`]);
  for (const name of ["icon.svg", "manifest.webmanifest"]) {
    if (contents.has(name)) assets.push([`./${name}`, `sha256-${sha256(contents.get(name))}`]);
  }
  await writeFile(path.join(root, "app-sw.js"), worker
    .replace('"__BUILD_VERSION__"', JSON.stringify(version))
    .replace('"__PRECACHE_FILES__"', JSON.stringify(assets)));
  await writeFile(path.join(root, ".nojekyll"), "");
  return { version, assets };
}

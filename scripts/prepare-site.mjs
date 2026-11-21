import { createHash } from "node:crypto";
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const root = path.resolve(process.argv[2] ?? "target/browser/todo-browser");
async function files(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async entry => {
    const file = path.join(directory, entry.name);
    return entry.isDirectory() ? files(file) : [file];
  }));
  return nested.flat();
}
const entries = (await files(root))
  .filter(file => !file.endsWith(".map") && !file.endsWith("app-sw.js") && !file.endsWith(".nojekyll"))
  .sort();
const hash = createHash("sha256");
for (const file of entries) hash.update(path.relative(root, file)).update(await readFile(file));
const version = hash.digest("hex").slice(0, 16);
const urls = entries.map(file => `./${path.relative(root, file).split(path.sep).join("/")}`);
const workerPath = path.join(root, "app-sw.js");
let worker = await readFile(workerPath, "utf8");
worker = worker.replace('"__BUILD_VERSION__"', JSON.stringify(version))
  .replace('"__PRECACHE_FILES__"', JSON.stringify(urls));
await writeFile(workerPath, worker);
await writeFile(path.join(root, ".nojekyll"), "");
console.log(`Prepared ${urls.length} offline resources (${version}).`);

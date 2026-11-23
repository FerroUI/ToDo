import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { prepareSite } from "./site-build.mjs";
import { integrity } from "./tests/service-worker-fixture.mjs";

async function site(t) {
  const root = await mkdtemp(path.join(tmpdir(), "todo-site-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const index = await readFile(new URL("../crates/todo-browser/wwwroot/index.html", import.meta.url));
  await mkdir(path.join(root, "fonts"));
  await Promise.all([
    writeFile(path.join(root, "index.html"), index),
    writeFile(path.join(root, "main.js"), 'import "./host.js";'),
    writeFile(path.join(root, "host.js"), "export const state = 1;"),
    ...["attachments.js", "attachment-store.js"].map(async name => writeFile(path.join(root, name),
      await readFile(new URL(`../crates/todo-browser/wwwroot/${name}`, import.meta.url)))),
    writeFile(path.join(root, "todo_browser.wasm"), "wasm bytes"),
    writeFile(path.join(root, "icon.svg"), "icon"),
    writeFile(path.join(root, "manifest.webmanifest"), '{"start_url":"./"}'),
    writeFile(path.join(root, "fonts", "Noto-COLRv1.ttf"), Buffer.from([0, 1, 0, 0, 10, 20, 30, 40])),
    writeFile(path.join(root, "fonts", "OFL.txt"), "SIL OPEN FONT LICENSE Version 1.1"),
    ...["bootstrap.js", "offline.js", "push-storage.js", "push-worker.js", "todo-browser.js", "ferroui.js", "ferroui-loader.js", "app.css", "fonts.js"].map(name => writeFile(path.join(root, name), name)),
  ]);
  return root;
}

test("each index references one immutable runtime tree with integrity for every offline resource", async t => {
  const root = await site(t), result = await prepareSite(root);
  const index = await readFile(path.join(root, "index.html"), "utf8");
  assert(index.includes(`./_app/${result.version}/bootstrap.js`));
  assert(!index.includes('src="./main.js"'));
  assert.equal(await readFile(path.join(root, "_app", result.version, "main.js"), "utf8"), 'import "./host.js";');
  assert.equal(await readFile(path.join(root, "main.js"), "utf8"), 'import "./host.js";');
  for (const [url, digest] of result.assets) assert.equal(digest, integrity(await readFile(path.join(root, decodeURIComponent(url)))));
  assert(result.assets.every(([url]) => url.startsWith(`./_app/${result.version}/`) || ["./index.html", "./icon.svg", "./manifest.webmanifest"].includes(url)));
  assert(!(await readFile(path.join(root, "app-sw.js"), "utf8")).includes("__BUILD_VERSION__"));
  for (const name of ["fonts/Noto-COLRv1.ttf", "fonts/OFL.txt", "attachments.js", "attachment-store.js", "push-storage.js", "push-worker.js"]) {
    assert(result.assets.some(([url]) => url === `./_app/${result.version}/${name}`), `${name} is available offline with the same runtime revision`);
    assert.deepEqual(await readFile(path.join(root, "_app", result.version, name)), await readFile(path.join(root, name)));
  }
});

test("re-preparing is stable and diagnostic captures never become offline assets", async t => {
  const root = await site(t), first = await prepareSite(root);
  await mkdir(path.join(root, "qa"));
  await writeFile(path.join(root, "qa", "failure.png"), "diagnostic");
  await writeFile(path.join(root, "main.js.map"), "map");
  const second = await prepareSite(root);
  assert.deepEqual(second, first);
  assert(!second.assets.some(([url]) => url.includes("qa/") || url.endsWith(".map")));
});

test("an incomplete runtime cannot publish an apparently installable offline shell", async t => {
  const root = await site(t);
  await rm(path.join(root, "todo_browser.wasm"));
  await assert.rejects(prepareSite(root), /missing todo_browser.wasm/);
});

test("an emoji font missing from the staged site prevents publication", async t => {
  const root = await site(t);
  await rm(path.join(root, "fonts", "Noto-COLRv1.ttf"));
  await assert.rejects(prepareSite(root), /missing fonts\/Noto-COLRv1.ttf/);
});

test("missing immutable push companions prevent an apparently installable service worker", async t => {
  for (const name of ["push-storage.js", "push-worker.js"]) {
    const root = await site(t); await rm(path.join(root, name));
    await assert.rejects(prepareSite(root), new RegExp(`missing ${name.replaceAll(".", "\\.")}`));
  }
});

test("both runtime bytes and service-worker policy change the cache revision", async t => {
  const root = await site(t), first = await prepareSite(root);
  await writeFile(path.join(root, "todo_browser.wasm"), "new wasm bytes");
  const second = await prepareSite(root);
  assert.notEqual(second.version, first.version);
  const template = await readFile(new URL("../crates/todo-browser/wwwroot/app-sw.js", import.meta.url), "utf8");
  const third = await prepareSite(root, `${template}\n// new policy\n`);
  assert.notEqual(third.version, second.version);
});

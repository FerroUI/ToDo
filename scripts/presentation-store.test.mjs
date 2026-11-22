import test from "node:test";
import assert from "node:assert/strict";
import { cloudEnvironment } from "../crates/todo-browser/wwwroot/cloud/environment.js";
import { PRESENTATION_KIND, MAX_PRESENTATION_BYTES, MAX_REPLICAS } from "../crates/todo-browser/wwwroot/cloud/presentation-format.js";
import { clearPresentationCache, createPresentationStore, presentationDownloadUrl, readPresentationResponse, replicaName } from "../crates/todo-browser/wwwroot/cloud/presentation-store.js";

const DEVICE = "11111111-1111-1111-1111-111111111111";
const another = index => `${String(index).padStart(8, "0")}-2222-2222-2222-222222222222`;
const replica = (device = DEVICE) => ({ kind: PRESENTATION_KIND, version: 1, device, records: {} });
const sizeOf = value => new TextEncoder().encode(value).length;

function fixture(devices = [DEVICE]) {
  clearPresentationCache();
  const events = [], files = devices.map((device, index) => {
    const body = JSON.stringify(replica(device));
    return { body, item: { id: `item/${index}`, name: replicaName(device), eTag: `revision-${index}`, size: sizeOf(body), file: { mimeType: "application/json" } } };
  });
  let cloud = "global", listing, detail, latest, downloadHook;
  const origin = () => cloud === "global" ? "https://owner.sharepoint.com" : cloud === "usgov" ? "https://owner.sharepoint.us" : "https://owner.sharepoint-mil.us";
  const store = createPresentationStore({
    environment: () => cloudEnvironment(cloud),
    async list(path, options) { events.push({ kind: "list", path, options }); return listing || structuredClone(files.map(file => file.item)); },
    async request(path, method = "GET", body, etag, options) {
      events.push({ kind: "request", path, method, body, etag, options });
      if (method === "PUT") {
        const name = path.match(/approot:\/(.*):\/content$/)[1];
        let file = files.find(file => file.item.name === name);
        if (!file) { file = { item: { id: `created/${files.length}`, name, file: {} } }; files.push(file); }
        file.body = JSON.stringify(body); file.item.eTag = "written"; file.item.size = sizeOf(file.body);
        return structuredClone(file.item);
      }
      const id = decodeURIComponent(path.split("items/")[1].split("?")[0]);
      const file = files.find(file => file.item.id === id);
      assert(file, "request stays within listed app-folder files");
      if (path.includes("?$select=")) return latest ? latest(file) : { id, eTag: file.item.eTag };
      const metadata = { ...structuredClone(file.item), "@microsoft.graph.downloadUrl": `${origin()}/download/${encodeURIComponent(id)}?preauthenticated=fixture` };
      return detail ? detail(file, metadata) : metadata;
    },
    async download(url, options) {
      events.push({ kind: "download", url: String(url), options });
      const id = decodeURIComponent(new URL(url).pathname.split("/download/")[1]);
      const file = files.find(file => file.item.id === id); assert(file);
      if (downloadHook) { const response = await downloadHook(file); if (response) return response; }
      return new Response(file.body, { headers: { "Content-Type": "application/json", "Content-Length": String(sizeOf(file.body)) } });
    },
  });
  return { files, events, store,
    set cloud(value) { cloud = value; }, set listing(value) { listing = value; },
    set detail(value) { detail = value; }, set latest(value) { latest = value; }, set downloadHook(value) { downloadHook = value; },
  };
}

test("preauthenticated OneDrive downloads are limited to the selected cloud's documented hosts", () => {
  for (const url of ["https://public.files.1drv.com/data?authkey=fixture", "https://api.onedrive.com/file", "https://tenant.sharepoint.com/file"]) {
    assert.equal(presentationDownloadUrl(url, cloudEnvironment()).href, url);
  }
  assert.equal(presentationDownloadUrl("https://tenant.sharepoint.us/file", cloudEnvironment("usgov")).hostname, "tenant.sharepoint.us");
  assert.equal(presentationDownloadUrl("https://tenant.dps.mil/file", cloudEnvironment("usgovdod")).hostname, "tenant.dps.mil");
  for (const url of ["https://attacker.test/file", "https://tenant.sharepoint.com.attacker.test/file", "https://evilsharepoint.com/file", "http://tenant.sharepoint.com/file",
    "https://user:password@tenant.sharepoint.com/file", "https://tenant.sharepoint.com:8443/file", "https://tenant.sharepoint.com/file#token", "https://tenant.sharepoint.us/file"]) {
    assert.throws(() => presentationDownloadUrl(url, cloudEnvironment()), /unexpected/);
  }
  assert.throws(() => presentationDownloadUrl("https://tenant.sharepoint-mil.us/file", cloudEnvironment("china")), error => error.code === "cloud_service_unavailable");
});

test("app-folder reads scope Graph requests and never forward credentials to the download URL", async () => {
  const f = fixture(); const result = await f.store.read("account");
  assert.deepEqual(result, [replica()]);
  for (const event of f.events.filter(event => event.kind !== "download")) assert.deepEqual(event.options, { scopes: ["Files.ReadWrite.AppFolder"] });
  const options = f.events.find(event => event.kind === "download").options;
  assert.equal(options.headers, undefined); assert.equal(options.credentials, "omit");
  assert.equal(options.redirect, "error"); assert.equal(options.referrerPolicy, "no-referrer");
  assert.equal(options.cache, "no-store"); assert(options.signal instanceof AbortSignal);
  assert(f.events.some(event => event.path?.includes("item%2F0")));
});

test("replica cache skips unchanged downloads, cannot be mutated by a caller, and isolates clouds/accounts", async () => {
  const f = fixture(); const first = await f.store.read("account");
  first[0].device = another(9);
  assert.deepEqual(await f.store.read("account"), [replica()]);
  assert.equal(f.events.filter(event => event.kind === "download").length, 1);
  await f.store.read("other-account");
  f.cloud = "usgov"; await f.store.read("account");
  assert.equal(f.events.filter(event => event.kind === "download").length, 3);
  assert.match(f.events.filter(event => event.kind === "download").at(-1).url, /sharepoint\.us/);
});

test("missing revisions, changed identities, invalid sizes, and duplicate files are rejected before publication", async () => {
  for (const mutate of [
    metadata => ({ ...metadata, id: "another" }), metadata => ({ ...metadata, name: replicaName(another(1)) }),
    metadata => ({ ...metadata, size: -1 }), metadata => ({ ...metadata, size: "12" }), metadata => ({ ...metadata, eTag: "" }),
  ]) {
    const f = fixture(); f.detail = (_file, metadata) => mutate(metadata);
    await assert.rejects(f.store.read("account"), /changed identity/);
    assert.equal(f.events.some(event => event.kind === "download"), false);
  }
  const duplicate = fixture(); duplicate.listing = [duplicate.files[0].item, duplicate.files[0].item];
  await assert.rejects(duplicate.store.read("account"), /invalid device settings file/);
  const latest = fixture(); latest.latest = file => ({ id: "different", eTag: file.item.eTag });
  await assert.rejects(latest.store.read("account"), /changed while downloading/);
});

test("partial responses and byte counts inconsistent with Graph metadata never become settings", async () => {
  const body = JSON.stringify(replica());
  await assert.rejects(readPresentationResponse(new Response(body, { status: 206, headers: { "Content-Range": `bytes 0-${body.length - 1}/${body.length + 2}` } })), /partial/);
  await assert.rejects(readPresentationResponse(new Response(body, { headers: { "Content-Range": "bytes 0-3/4" } })), /partial/);
  await assert.rejects(readPresentationResponse(new Response(body), { expectedSize: body.length + 1 }), /size changed/);
  await assert.rejects(readPresentationResponse(new Response(new Uint8Array([0xff, 0xfe]))), /encoded data/);
});

test("oversized streaming bodies are cancelled before JSON decoding", async () => {
  let cancelled = false;
  const stream = new ReadableStream({ start(controller) { controller.enqueue(new Uint8Array(MAX_PRESENTATION_BYTES + 1)); }, cancel() { cancelled = true; } });
  await assert.rejects(readPresentationResponse(new Response(stream, { headers: { "Content-Length": "1" } })), /4 MiB/);
  assert.equal(cancelled, true);
});

test("file count and aggregate size limits stop excessive folders before downloading", async () => {
  const many = fixture(); many.listing = Array.from({ length: MAX_REPLICAS + 1 }, (_, index) => ({ id: `id-${index}`, name: replicaName(another(index)), file: {}, size: 1, eTag: "1" }));
  await assert.rejects(many.store.read("account"), /too many/);
  assert.equal(many.events.some(event => event.kind === "download"), false);
  const large = fixture(); large.listing = Array.from({ length: 5 }, (_, index) => ({ id: `id-${index}`, name: replicaName(another(index)), file: {}, size: MAX_PRESENTATION_BYTES, eTag: "1" }));
  await assert.rejects(large.store.read("account"), /too many/);
  assert.equal(large.events.some(event => event.kind === "download"), false);
});

test("growth after listing still respects the combined download limit", async () => {
  const f = fixture(Array.from({ length: 5 }, (_, index) => another(index)));
  f.listing = f.files.map(file => ({ ...file.item, size: 0 }));
  for (const file of f.files) { file.body = file.body.padEnd(MAX_PRESENTATION_BYTES, " "); file.item.size = MAX_PRESENTATION_BYTES; }
  await assert.rejects(f.store.read("account"), /16 MiB combined/);
  assert.equal(f.events.filter(event => event.kind === "download").length, 4);
});

test("a failed later replica does not publish an earlier partial cache", async () => {
  const f = fixture([DEVICE, another(1)]), valid = f.files[1].body;
  f.files[1].body = "invalid"; f.files[1].item.size = 7;
  await assert.rejects(f.store.read("account"));
  f.files[1].body = valid; f.files[1].item.size = sizeOf(valid);
  await f.store.read("account");
  assert.equal(f.events.filter(event => event.kind === "download").length, 4);
});

test("clearing a session cancels late cache publication and permits a clean next read", async () => {
  const f = fixture(), gate = Promise.withResolvers(), entered = Promise.withResolvers();
  f.downloadHook = async () => { entered.resolve(); await gate.promise; };
  const read = f.store.read("account"), rejected = assert.rejects(read, error => error.code === "account_changed");
  await entered.promise; clearPresentationCache(); gate.resolve(); await rejected;
  f.downloadHook = null; await f.store.read("account");
  assert.equal(f.events.filter(event => event.kind === "download").length, 2);
});

test("writes use one stable device file, optional scope, and a validated acknowledgment", async () => {
  const f = fixture([]), value = replica();
  await f.store.write("account", value);
  const call = f.events.find(event => event.method === "PUT");
  assert.equal(call.path, `me/drive/special/approot:/${replicaName(DEVICE)}:/content`);
  assert.deepEqual(call.options, { scopes: ["Files.ReadWrite.AppFolder"] }); assert.equal(call.etag, undefined);
  value.device = another(2);
  assert.deepEqual(await f.store.read("account"), [replica()]);
  assert.equal(f.events.some(event => event.kind === "download"), false);
});

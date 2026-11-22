import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";

registerHooks({
  resolve(specifier, context, next) {
    const result = next(specifier, context);
    if (result.url.endsWith("/cloud/client.js")) return { url: "fixture:upload", shortCircuit: true };
    if (result.url.endsWith("/wwwroot/attachments.js")) return { url: "fixture:files", shortCircuit: true };
    return result;
  },
  load(url, context, next) {
    if (url === "fixture:files") return { format: "module", shortCircuit: true, source: `
      export const readAttachment = async key => globalThis.uploadFixture.files.get(key) || null;
      export const storeAttachment = async (key,file) => globalThis.uploadFixture.files.set(key,file);
      export const downloadBlob = () => {};
    ` };
    if (url !== "fixture:upload") return next(url, context);
    return { format: "module", shortCircuit: true, source: `
      export const segment=encodeURIComponent;
      export const taskPath=(list,id)=>'me/todo/lists/'+segment(list)+'/tasks/'+segment(id);
      export const graph=(...args)=>globalThis.uploadFixture.graph(...args);
      export const graphBytes=(...args)=>globalThis.uploadFixture.bytes(...args);
      export const graphUpload=(...args)=>globalThis.uploadFixture.upload(...args);
      export const graphUrl=path=>{const url=new URL(path,'https://graph.microsoft.com/v1.0/');if(url.origin!=='https://graph.microsoft.com'||!url.pathname.startsWith('/v1.0/'))throw new Error('Unexpected URL');return url;};
    ` };
  },
});
const { uploadAttachment, materializeCloudAttachment, cloudAttachmentKey } = await import("../crates/todo-browser/wwwroot/cloud/attachment-content.js");
const uploadUrl = "https://graph.microsoft.com/v1.0/me/todo/lists/list/tasks/task/attachmentSessions/session";
const path = "me/todo/lists/list/tasks/task/attachments";
const MiB = 1024 * 1024;
const context = () => ({ guard() {}, save() {} });

test("large attachment uploads use ordered bounded chunks and the final attachment location", async () => {
  const file = new File([new Uint8Array(5 * MiB)], "large.bin");
  const calls = [];
  globalThis.uploadFixture = {
    files: new Map(),
    async graph(url, method) {
      calls.push([method || "GET", url]);
      if (method === "POST") return { uploadUrl, expirationDateTime: new Date(Date.now() + 600_000).toISOString(), nextExpectedRanges: ["0-"] };
      return { id: "file", size: file.size };
    },
    async upload(url, chunk, start, total) {
      calls.push(["PUT", start, chunk.size]); assert.equal(url, uploadUrl); assert.equal(total, file.size);
      return start + chunk.size === total ? { status: 201, location: `${path}/file` }
        : { status: 200, data: { NextExpectedRanges: [String(start + chunk.size)] } };
    },
  };
  const result = await uploadAttachment(path, { name: file.name, size: file.size, mime_type: file.type }, file, {}, context());
  assert.equal(result.id, "file");
  assert.deepEqual(calls.filter(call => call[0] === "PUT"), [["PUT", 0, 2 * MiB], ["PUT", 2 * MiB, 2 * MiB], ["PUT", 4 * MiB, MiB]]);
});

test("an interrupted range resumes from Microsoft's accepted offset", async () => {
  const file = new File([new Uint8Array(5 * MiB)], "resume.bin"), pending = {};
  const positions = []; let interrupted = false;
  globalThis.uploadFixture = {
    files: new Map(),
    async graph(url, method) {
      if (method === "POST") return { uploadUrl, expirationDateTime: new Date(Date.now() + 600_000).toISOString(), nextExpectedRanges: ["0-"] };
      if (url === `${uploadUrl}?$select=expirationDateTime,nextExpectedRanges`) return { nextExpectedRanges: [String(4 * MiB)], content: "must-not-be-checkpointed" };
      return { id: "file" };
    },
    async upload(url, chunk, start, total) {
      positions.push(start);
      if (start === 2 * MiB && !interrupted) { interrupted = true; throw new Error("Lost acknowledgement"); }
      return start + chunk.size === total ? { status: 201, location: `${path}/file` }
        : { status: 200, data: { nextExpectedRanges: [String(start + chunk.size)] } };
    },
  };
  const local = { name: file.name, size: file.size, mime_type: file.type };
  await assert.rejects(uploadAttachment(path, local, file, pending, context()), /Lost acknowledgement/);
  assert.equal((await uploadAttachment(path, local, file, pending, context())).id, "file");
  assert.deepEqual(positions, [0, 2 * MiB, 4 * MiB]);
  assert.equal(pending.session.content, undefined);
});

test("cloud file downloads require their original account and cache verified bytes", async () => {
  const file = { id: "file", name: "data.txt", size: 3, contentType: "text/plain" };
  const key = cloudAttachmentKey("account", "list", "task", file); let reads = 0;
  globalThis.uploadFixture = { files: new Map(), graph: async () => file, bytes: async () => { reads++; return new Blob(["abc"]); } };
  await assert.rejects(materializeCloudAttachment(key, "another"), /account that owns/);
  assert.equal(reads, 0);
  assert.equal(await (await materializeCloudAttachment(key, "account")).text(), "abc");
  await materializeCloudAttachment(key, "account"); assert.equal(reads, 1);
});

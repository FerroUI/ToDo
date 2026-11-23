import test from "node:test";
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { Agent } from "node:https";
import { sendEncrypted } from "../src/transport.mjs";
import { config, subscription } from "./fixture.mjs";

test("the pinned library encrypts and signs payloads while HTTP redirects and response bodies are never followed", async () => {
  const settings = config(), agent = new Agent(), options = { vapidDetails: settings.vapid, agent, TTL: 60, timeout: 1000, contentEncoding: "aes128gcm" };
  let encrypted, destroyed = false, captured;
  const request = (_url, parameters, respond) => {
    captured = parameters;
    const outgoing = new EventEmitter();
    outgoing.end = body => { encrypted = body; queueMicrotask(() => respond({ statusCode: 302, headers: { location: "http://127.0.0.1/private", "retry-after": "15" }, destroy() { destroyed = true; } })); };
    return outgoing;
  };
  await assert.rejects(sendEncrypted(subscription(), "Private task title", options, request), error => error.statusCode === 302 && error.headers["retry-after"] === "15");
  assert(destroyed); assert(Buffer.isBuffer(encrypted)); assert(!encrypted.includes("Private task title"));
  assert.equal(captured.agent, agent); assert.equal(captured.maxHeaderSize, 8192);
  const headers = Object.fromEntries(Object.entries(captured.headers).map(([key, value]) => [key.toLowerCase(), value]));
  assert.equal(headers["content-encoding"], "aes128gcm"); assert.match(headers.authorization, /^vapid /); agent.destroy();
});

test("an absolute delivery deadline aborts even when DNS or a provider never produces a response", async () => {
  const settings = config(), agent = new Agent(); let aborted = false;
  const request = (_url, { signal }) => {
    const outgoing = new EventEmitter(); outgoing.end = () => {};
    signal.addEventListener("abort", () => { aborted = true; outgoing.emit("error", new Error("Delivery deadline")); }, { once: true });
    return outgoing;
  };
  await assert.rejects(sendEncrypted(subscription(), "Task", { vapidDetails: settings.vapid, agent, TTL: 60, timeout: 10 }, request), /deadline/);
  assert(aborted); agent.destroy();
});

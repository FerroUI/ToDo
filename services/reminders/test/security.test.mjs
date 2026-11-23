import test from "node:test";
import assert from "node:assert/strict";
import { isPublicAddress, safeAgent, subscription as validateSubscription, RateLimiter } from "../src/security.mjs";
import { readConfig } from "../src/config.mjs";
import { config, subscription } from "./fixture.mjs";

test("endpoint validation rejects credentials, redirects to arbitrary hosts, extra fields, bad keys and non-HTTPS", () => {
  const settings = config(), valid = subscription();
  assert.equal(validateSubscription(valid, settings).endpoint, valid.endpoint);
  for (const endpoint of ["http://fcm.googleapis.com/push", "https://fcm.googleapis.com.evil.test/push", "https://127.0.0.1/push",
    "https://user:pass@fcm.googleapis.com/push", "https://fcm.googleapis.com:444/push", "https://fcm.googleapis.com/push#fragment"]) {
    assert.throws(() => validateSubscription({ ...valid, endpoint }, settings), /allowed HTTPS/);
  }
  assert.throws(() => validateSubscription({ ...valid, access_token: "never accepted" }, settings), /Unexpected/);
  assert.throws(() => validateSubscription({ ...valid, keys: { ...valid.keys, auth: "invalid" } }, settings), /keys/);
  assert.throws(() => readConfig({ TODO_PUSH_ORIGINS: "*" }), /Invalid URL|exact/);
});

test("DNS connection guard rejects all local, mapped, documentation and transitional networks", async () => {
  for (const address of ["0.0.0.0", "127.0.0.1", "10.2.3.4", "169.254.169.254", "172.16.0.1", "192.168.1.1", "100.64.0.1",
    "198.18.0.1", "192.0.2.1", "198.51.100.1", "203.0.113.1", "224.0.0.1", "255.255.255.255", "::1", "::", "fe80::1", "fc00::1",
    "::ffff:127.0.0.1", "2001:db8::1", "2001:0::1", "2001::1", "2001:2::1", "2002:7f00:1::1", "3fff::1"]) assert.equal(isPublicAddress(address), false, address);
  for (const address of ["8.8.8.8", "142.250.1.1", "2607:f8b0:4005:80a::200e", "2001:4860:4860::8888"]) assert.equal(isPublicAddress(address), true, address);
  const agent = safeAgent((_name, _options, done) => done(null, [{ address: "8.8.8.8", family: 4 }, { address: "127.0.0.1", family: 4 }]));
  await assert.rejects(new Promise((resolve, reject) => agent.options.lookup("push.example", {}, error => error ? reject(error) : resolve())), /non-public/);
  agent.destroy();
});

test("rate buckets are bounded and expire without allowing unlimited attacker keys", () => {
  let now = 0; const limiter = new RateLimiter({ maximum: 2, now: () => now });
  limiter.take("a", 1); assert.throws(() => limiter.take("a", 1), /Too many/);
  limiter.take("b", 1); assert.throws(() => limiter.take("c", 1), /rate limit/);
  now = 60_001; limiter.take("c", 1); assert.equal(limiter.entries.size, 1);
});

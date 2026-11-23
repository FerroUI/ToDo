import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";
import { workerFixture } from "./tests/service-worker-fixture.mjs";

async function fixture({ href = "https://example.test/ToDo/", permission = "granted", reply = "granted", registration } = {}) {
  const calls = [], notifications = [];
  class Notification {
    static permission = permission;
    static async requestPermission() { calls.push("permission"); return reply; }
    constructor(title, options) { this.title = title; this.options = options; notifications.push(this); }
    close() { this.closed = true; }
  }
  const context = { Notification, URL, location: { href },
    window: { Notification, focus() { calls.push("focus"); } },
    navigator: { serviceWorker: { async getRegistration(scope) { calls.push(scope); return registration; } } },
  };
  const source = await readFile(new URL("../crates/todo-browser/wwwroot/notifications.js", import.meta.url), "utf8");
  vm.runInNewContext(source.replaceAll("export ", ""), context);
  return { ...context, calls, notifications };
}

test("reminder data preserves only app document mode and never OAuth callback data", async () => {
  const shown = [], registration = { scope: "https://example.test/ToDo/", active: {},
    async showNotification(title, options) { shown.push({ title, options }); } };
  const api = await fixture({ href: "https://example.test/ToDo/index.html?demo=1&code=secret&state=one-use#access_token=secret", registration });
  await api.notify("Remember", "Task reminder");
  assert.equal(api.calls[0], registration.scope);
  assert.equal(shown[0].options.data.url, "https://example.test/ToDo/?demo=1");
  assert.equal(shown[0].options.icon, "https://example.test/ToDo/icon.svg");
  assert.equal(shown[0].options.tag, "todo-demo-Remember");
  assert(!JSON.stringify(shown).includes("secret"));
  assert.equal(api.notifications.length, 0);
});

test("another app's registration and an installing worker cannot receive a reminder", async () => {
  for (const registration of [
    { scope: "https://example.test/", active: {}, showNotification() { throw new Error("Wrong app"); } },
    { scope: "https://example.test/ToDo/", active: null, showNotification() { throw new Error("Not active"); } },
  ]) {
    const api = await fixture({ registration });
    await api.notify("Remember", "Task reminder");
    const [notification] = api.notifications;
    assert.equal(notification.options.data.url, "https://example.test/ToDo/");
    notification.onclick();
    assert.equal(notification.closed, true);
    assert.equal(api.calls.at(-1), "focus");
  }
});

test("requesting notification permission distinguishes dismissal from a permanent block", async () => {
  const allowed = await fixture();
  await allowed.requestNotifications();
  assert.deepEqual(allowed.calls, [], "a granted permission does not reopen the prompt");
  const dismissed = await fixture({ permission: "default", reply: "default" });
  await assert.rejects(dismissed.requestNotifications(), /not granted.*try again/);
  const denied = await fixture({ permission: "denied", reply: "denied" });
  await assert.rejects(denied.requestNotifications(), /blocked.*browser settings/);
  await denied.notify("Remember", "Task reminder");
  assert.equal(denied.notifications.length, 0);
});

test("clicking a personal reminder focuses a personal task document without navigating its editor", async () => {
  const api = await workerFixture(), focused = [];
  const client = (name, url, visible = false) => ({ url, visibilityState: visible ? "visible" : "hidden",
    async focus() { focused.push(name); return this; }, navigate() { throw new Error("Must preserve edits"); } });
  api.clients.set("demo", client("demo", `${api.scope}?demo=1`, true));
  api.clients.set("foreign", client("foreign", `${api.scope}other.html`, true));
  api.clients.set("personal", client("personal", `${api.index}?code=callback&state=one-use`));
  await api.notificationClick({ url: `${api.scope}?code=obsolete` });
  assert.deepEqual(focused, ["personal"]);
  assert.deepEqual(api.openedWindows, []);
});

test("demo reminders open the demo document when only a personal tab is present", async () => {
  const api = await workerFixture();
  api.clients.set("personal", { url: api.scope, async focus() { throw new Error("Wrong mode"); } });
  await api.notificationClick({ url: `${api.index}?demo=1&code=obsolete#fragment` });
  assert.deepEqual(api.openedWindows, [`${api.scope}?demo=1`]);
});

test("notification targets cannot navigate outside the application or replay auth fragments", async () => {
  for (const url of [undefined, "invalid", "https://other.test/ToDo/?demo=1", "https://example.test/ToDo/other.html?demo=1",
    "https://user:secret@example.test/ToDo/?demo=1", "https://example.test/ToDo/?code=obsolete#access_token=secret"]) {
    const api = await workerFixture();
    await api.notificationClick({ url });
    assert.deepEqual(api.openedWindows, [api.scope]);
  }
});

test("notification focus tolerates closed clients and prefers a visible matching window", async () => {
  const api = await workerFixture(), focused = [];
  api.clients.set("hidden", { url: api.scope, visibilityState: "hidden", async focus() { focused.push("hidden"); return this; } });
  api.clients.set("gone", { url: api.scope, focused: true, async focus() { focused.push("gone"); throw new Error("Closed"); } });
  api.clients.set("visible", { url: api.index, visibilityState: "visible", async focus() { focused.push("visible"); return this; } });
  await api.notificationClick({ url: api.scope });
  assert.deepEqual(focused, ["gone", "visible"]);
  assert.deepEqual(api.openedWindows, []);
  api.clients.clear();
  api.clients.set("gone", { url: api.scope, async focus() { throw new Error("Closed"); } });
  await api.notificationClick({ url: api.scope });
  assert.deepEqual(api.openedWindows, [api.scope]);
});

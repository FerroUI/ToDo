import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { open, sleep, colours } from "../../FerroUI/scripts/browser/harness.mjs";
import { typeText } from "./text-input.mjs";
import { boundPage, withDeadline } from "./deadline.mjs";

// Shared driver over the actual compiled controls. No application commands bypass input handling.
export async function createHarness(site, output) {
  await mkdir(output, { recursive: true });
  const page = boundPage(await open(site, { query: "?nooffline=1", width: 1280, height: 800, network: true, navigate: false }));
  const failures = [], requests = new Map();
  page.on("Network.requestWillBeSent", event => requests.set(event.requestId, event.request.url));
  page.on("Network.responseReceived", event => {
    if (event.response.status >= 400) failures.push(`HTTP ${event.response.status}: ${event.response.url}`);
  });
  page.on("Network.loadingFailed", event => {
    if (!event.canceled) failures.push(`${event.errorText}: ${requests.get(event.requestId) || event.requestId}`);
  });

  const snapshot = async () => JSON.parse(await page.evaluate("globalThis.todoApp.inspectView()"));
  // exportData() flushes editors and would mask a broken blur/pagehide commit handler.
  const saved = async () => JSON.parse(await page.evaluate("localStorage.getItem('ferroui.todo.v1') || '{}'"));
  function healthy() {
    const errors = [...failures, ...page.errors];
    if (errors.length) throw new Error(errors.join("\n"));
  }
  async function until(check, label, timeout = 10000) {
    const end = Date.now() + timeout;
    let lastError;
    while (Date.now() < end) {
      healthy();
      try { const value = await withDeadline(check, `condition: ${label}`, Math.max(1, end - Date.now())); if (value) return value; }
      catch (error) { lastError = error; }
      await sleep(75);
    }
    throw new Error(`Timed out: ${label}${lastError ? ` (${lastError.message})` : ""}`);
  }
  async function ready() {
    await until(() => page.evaluate(`(() => {
      const splash = document.querySelector('.ferroui-splash');
      return !!document.querySelector('canvas') && !!globalThis.todoApp?.inspectView
        && (!splash || (splash.classList.contains('splash-close') && getComputedStyle(splash).display === 'none'));
    })()`), "the application draws and hides its splash", 60000);
    await control(element => element.name === "NewTask", "task composer");
  }
  async function control(match, label) {
    return until(async () => {
      const matches = (await snapshot())?.elements.filter(element => element.hit && match(element)) || [];
      assert(matches.length <= 1, `Ambiguous rendered control: ${label}`);
      return matches[0];
    }, `visible, hit-testable ${label}`);
  }
  async function clickText(text) {
    // WinUI string content uses AccessText for access-key labels.
    await clickControl(element => /(?:TextBlock|AccessText)$/.test(element.type) && element.text === text, text);
  }
  async function clickControl(match, label) {
    const element = await control(match, label);
    await page.click(element.x + element.width / 2, element.y + element.height / 2);
  }
  async function focusControl(match, label) {
    for (let tabs = 0; tabs <= 24; tabs++) {
      const element = await control(match, label);
      if (element.focused) return;
      if (tabs < 24) await page.press("Tab", ["shift"]);
    }
    throw new Error(`Cannot reach ${label} with keyboard navigation`);
  }
  async function edit(name, text, enter = false) {
    const field = await control(element => element.name === name, name);
    await page.click(field.x + field.width / 2, field.y + field.height / 2);
    await page.press("a", ["ctrl"]);
    await typeText(page, text);
    // Empty typing emits no key, so explicitly remove the selected contents.
    if (!text) await page.press("Backspace");
    if (enter) await page.press("Enter");
  }
  async function navigate(command, parameters) {
    const previous = page.navigations.length;
    const result = await page.send(command, parameters);
    assert(!result.error, result.error?.message);
    await until(() => page.navigations.length > previous, "a fresh document after navigation");
    await ready();
  }
  async function resize(width, height = 800) {
    await page.resize(width, height);
    await until(async () => {
      const view = await snapshot();
      return Math.abs(view.width - width) < 1 && Math.abs(view.height - height) < 1;
    }, `the ${width} × ${height} viewport`);
    // Let ResizeObserver and the retained-control layout pass complete before reading geometry.
    await page.evaluate("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))");
  }
  async function capture(name) {
    await sleep(200);
    healthy();
    const image = await page.screenshot(path.join(output, `${name}.png`));
    assert(colours(image) > 40, `${name} does not contain a rendered interface`);
    await writeFile(path.join(output, `${name}.json`), JSON.stringify(await snapshot()));
  }
  return { page, snapshot, saved, healthy, until, ready, control, clickControl, clickText,
    focusControl, edit, navigate, capture, resize };
}

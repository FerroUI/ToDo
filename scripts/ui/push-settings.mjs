import assert from "node:assert/strict";

const button = name => element => element.type.endsWith("Button") && element.automationName === name;
const named = name => element => element.name === name;

/** Native Settings state/layout review. Never enrolls a real subscription. */
export async function verifyPushSettings(ui) {
  const { page, saved, resize, clickControl, control, snapshot, edit, until, capture, navigate } = ui;
  const originalUrl = await page.evaluate("location.href"), personal = await saved();
  const demoUrl = new URL(originalUrl); demoUrl.search = "?demo=1&nooffline=1";
  const demoDocument = () => page.evaluate("localStorage.getItem('ferroui.todo.demo.v1')");
  const previousDemo = await demoDocument();
  let originalDemo;
  const state = { available: true, supported: true, enabled: false, service_url: "", public_key: "", status: "A user-configured reminder service is required." };
  const setState = value => page.evaluate(`todoApp.pushState(${JSON.stringify(JSON.stringify(value))})`);
  const visible = async (match, label, width) => {
    for (let step = 0; step < 35; step++) {
      if ((await snapshot()).elements.some(element => element.hit && match(element))) return control(match, label);
      await page.wheel(width - 60, 630, 0, 150);
    }
    return control(match, label);
  };
  const bounded = (element, width) => assert(element.x >= -1 && element.x + element.width <= width + 1, `Push Settings control overflows: ${JSON.stringify(element)}`);
  try {
    await navigate("Page.navigate", { url: demoUrl.href });
    originalDemo = await demoDocument();
    assert.equal(await page.evaluate("new URLSearchParams(location.search).get('demo')"), "1", "push Settings review runs in the isolated demo");
    for (const width of [1280, 320]) {
      await resize(width, 844);
      await setState({ ...state, available: false, supported: false });
      await clickControl(button("Settings"), "open native push Settings review");
      await setState({ ...state, supported: false, status: "Background push is unavailable in this host." });
      await visible(named("PushStatus"), "unsupported background push explanation", width);
      assert(!(await snapshot()).elements.some(named("PushEnrollmentKey")));
      await setState(state);
      await visible(named("PushServiceUrl"), "reminder service address", width);
      await edit("PushServiceUrl", "https://reminders.example.test/");
      await visible(named("PushPublicKey"), "public VAPID key", width);
      await edit("PushPublicKey", "public-key-draft");
      await setState({ ...state, status: "Status refresh keeps unfinished setup fields." });
      for (const [name, value] of [["PushServiceUrl", "https://reminders.example.test/"], ["PushPublicKey", "public-key-draft"]]) {
        const input = await control(named(name), name); bounded(input, width); assert.equal(input.text, value);
      }
      await visible(named("PushEnrollmentKey"), "masked service enrollment key", width);
      await edit("PushEnrollmentKey", "not-a-real-enrollment-key");
      await visible(button("Enable background reminders"), "explicit background reminder opt-in", width);
      await capture(`push-settings-${width}`);
      // The real host remains an unsupported demo. This only verifies that the
      // native action clears the secret draft; the host cannot contact a service.
      await clickControl(button("Enable background reminders"), "clear one-use enrollment draft");
      await until(async () => (await control(named("PushEnrollmentKey"), "cleared enrollment field")).text === "", "enrollment draft is cleared");
      await setState({ ...state, enabled: true, service_url: "https://reminders.example.test/", public_key: "public-key-draft", status: "2 reminders scheduled from the active workspace." });
      await visible(button("Refresh background reminders"), "refresh configured subscription", width);
      bounded(await control(button("Disable background reminders"), "disable configured subscription"), width);
      assert(!(await snapshot()).elements.some(named("PushEnrollmentKey")));
      await capture(`push-settings-enabled-${width}`);
      await setState({ ...state, service_url: "https://reminders.example.test/", public_key: "public-key-draft" });
      await visible(button("Enable background reminders"), "return to disabled setup", width);
      await clickControl(button("Close settings"), "close background push Settings review");
    }
    assert.equal(await demoDocument(), originalDemo, "host push state and drafts never enter the demo task backup");
    assert.deepEqual(await saved(), personal, "push review leaves the personal task document unchanged");
  } finally {
    // Leave the demo before restoring its earlier storage, so lifecycle saves
    // cannot put the review's sample document back over the preserved bytes.
    try { await navigate("Page.navigate", { url: originalUrl }); }
    finally {
      await page.evaluate(previousDemo === null ? "localStorage.removeItem('ferroui.todo.demo.v1')"
        : `localStorage.setItem('ferroui.todo.demo.v1', ${JSON.stringify(previousDemo)})`);
      await resize(1280, 800);
    }
  }
}

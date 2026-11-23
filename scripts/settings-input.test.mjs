import assert from "node:assert/strict";
import test from "node:test";
import { revealSettingsControl, settingsSwitchTarget, setSettingsPreference } from "./ui/settings-input.mjs";

const label = "Auto-hide empty smart lists", key = "hide_empty_smart_lists";
// Browser ff96da88 failure: row.hit was true, but the attempted y=807 click
// was below both the 800px canvas and the SettingsScroll bottom at y=738.
const captured = {
  width: 1280, height: 800,
  elements: [
    { name: "SettingsScroll", type: "ScrollViewer", x: 941, y: 88, width: 339, height: 650, hit: true },
    { type: "ToggleSwitch", automationName: label, x: 961, y: 766, width: 299, height: 61, hit: true },
    { name: "SwitchKnobBounds", type: "Rectangle", x: 961, y: 797, width: 40, height: 20, hit: false },
  ],
};

function at(y) {
  return { ...captured, elements: captured.elements.map(element => element.name === "SettingsScroll" ? { ...element }
    : { ...element, y: element.type === "ToggleSwitch" ? y : y + 31, hit: true }) };
}

function harness(initial = captured, { persists = true } = {}) {
  let view = initial, enabled = true;
  const clicks = [], wheels = [];
  return {
    clicks, wheels,
    saved: async () => ({ settings: { [key]: enabled } }),
    snapshot: async () => view,
    until: async (condition, description) => assert(await condition(), description),
    page: {
      click: async (x, y) => {
        const { point } = settingsSwitchTarget(view, label);
        assert.deepEqual({ x, y }, point, "click uses the newly observed, fully visible native track");
        assert(y < 738 && y > 88);
        clicks.push({ x, y });
        if (persists) enabled = false;
      },
      wheel: async (x, y, dx, dy) => {
        assert(x > 941 && x < 1280 && y > 88 && y < 738, "wheel enters the real Settings viewport");
        wheels.push(dy);
        view = dy < -1000 ? captured : at(400);
      },
      evaluate: async source => {
        assert(source.includes("requestAnimationFrame"));
        assert(!source.includes("localStorage"), "waiting for layout never mutates preferences");
      },
    },
  };
}

test("clipped switch row cannot supply an offscreen input point despite hit=true", () => {
  assert.equal(settingsSwitchTarget(captured, label).point, null);
  assert.equal(settingsSwitchTarget(at(50), label).point, null, "track clipped by the fixed Settings header");
  assert.deepEqual(settingsSwitchTarget(at(400), label).point, { x: 981, y: 441 });
  const shortCanvas = { ...at(400), height: 440 };
  assert.equal(settingsSwitchTarget(shortCanvas, label).point, null, "viewport also clips a taller scroll area");
});

test("bulk preference interaction scrolls and reacquires the actual track before clicking", async () => {
  const ui = harness();
  await setSettingsPreference(ui, label, key, false);
  assert(ui.wheels[0] > 0);
  assert.deepEqual(ui.clicks, [{ x: 981, y: 441 }]);
  assert.equal((await ui.saved()).settings[key], false);
});

test("an omitted offscreen row starts a bounded scan from the top", async () => {
  const ui = harness({ ...captured, elements: [captured.elements[0]] });
  await setSettingsPreference(ui, label, key, false);
  assert(ui.wheels[0] < 0 && ui.wheels[1] > 0);
  assert.equal(ui.clicks.length, 1);
});

test("saved preference assertions remain mandatory after native activation", async () => {
  const ui = harness(at(400), { persists: false });
  await assert.rejects(setSettingsPreference(ui, label, key, false), /Auto-hide empty smart lists preference saved/);
  assert.equal(ui.clicks.length, 1);
});

test("a preference already at its requested value does not toggle", async () => {
  const ui = harness();
  await setSettingsPreference(ui, label, key, true);
  assert.deepEqual(ui.clicks, []);
  assert.deepEqual(ui.wheels, []);
});

test("generic Settings reveal reacquires a complete editor after scrolling", async () => {
  const scroll = captured.elements[0];
  const editor = { name: "CategoryName", type: "TextBox", x: 961, y: 725, width: 299, height: 38, hit: true };
  let view = { width: 1280, height: 800, elements: [scroll, editor] };
  let wheelCount = 0;
  const ui = {
    snapshot: async () => view,
    page: {
      wheel: async (_x, _y, _dx, dy) => {
        assert(dy > 0, "an editor crossing the scroll bottom is brought upward");
        wheelCount++;
        view = { ...view, elements: [scroll, { ...editor, y: 400 }] };
      },
      evaluate: async () => {},
    },
  };
  const visible = await revealSettingsControl(ui, element => element.name === "CategoryName", "category editor");
  assert.equal(wheelCount, 1);
  assert.equal(visible.y, 400, "caller receives the post-scroll geometry rather than the clipped original");
});

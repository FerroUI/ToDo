import assert from "node:assert/strict";

function intersection(a, b) {
  const x = Math.max(a.x, b.x), y = Math.max(a.y, b.y);
  const right = Math.min(a.x + a.width, b.x + b.width);
  const bottom = Math.min(a.y + a.height, b.y + b.height);
  return right > x && bottom > y ? { x, y, width: right - x, height: bottom - y } : null;
}

function contains(outer, inner) {
  return inner.x >= outer.x && inner.y >= outer.y
    && inner.x + inner.width <= outer.x + outer.width
    && inner.y + inner.height <= outer.y + outer.height;
}

function settingsClip(view) {
  const scroll = view.elements.find(element => element.name === "SettingsScroll");
  return scroll && intersection(scroll, { x: 0, y: 0, width: view.width, height: view.height });
}

/** Resolve the actual native switch track inside the Settings scroll viewport. */
export function settingsSwitchTarget(view, label) {
  const rows = view.elements.filter(element => element.type === "ToggleSwitch" && element.automationName === label);
  assert(rows.length <= 1, `Ambiguous Settings switch: ${label}`);
  const row = rows[0];
  const clip = settingsClip(view);
  const track = row && view.elements.find(element => element.name === "SwitchKnobBounds" && contains(row, element));
  // A row can intersect the canvas while its switch is clipped or completely
  // below it. Its broad hit flag cannot establish that this input point is safe.
  const point = track?.hit && clip && contains(clip, track)
    ? { x: track.x + track.width / 2, y: track.y + track.height / 2 } : null;
  return { row, track, clip, point };
}

/** Scroll and reacquire a complete, hit-testable control inside Settings. */
export async function revealSettingsControl(ui, match, label) {
  return reveal(ui, label, view => {
    const controls = view.elements.filter(match);
    assert(controls.length <= 1, `Ambiguous Settings control: ${label}`);
    const row = controls[0], clip = settingsClip(view);
    return { row, target: row, clip, ready: row?.hit && clip && contains(clip, row) ? row : null };
  });
}

async function reveal(ui, label, locate) {
  let searchedFromTop = false;
  for (let step = 0; step < 24; step++) {
    const { row, target, clip, ready } = locate(await ui.snapshot());
    assert(clip, `Settings scroll viewport is visible while locating ${label}`);
    if (ready) return ready;
    let delta;
    if (row) {
      const bounds = target || row;
      delta = Math.max(-300, Math.min(300, bounds.y + bounds.height / 2 - (clip.y + clip.height / 2)));
    } else if (!searchedFromTop) {
      // The inspector omits controls entirely outside the canvas. Start a
      // bounded forward scan instead of guessing which side contains the row.
      searchedFromTop = true;
      delta = -10000;
    } else {
      delta = 300;
    }
    await ui.page.wheel(clip.x + clip.width / 2, clip.y + clip.height / 2, 0, delta);
    await ui.page.evaluate("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))");
  }
  throw new Error(`Cannot reveal the native Settings input: ${label}`);
}

/** Change a persisted preference solely through real native pointer input. */
export async function setSettingsPreference(ui, label, key, value) {
  if ((await ui.saved()).settings[key] === value) return;
  const point = await reveal(ui, label, view => {
    const { row, track, clip, point } = settingsSwitchTarget(view, label);
    return { row, target: track, clip, ready: point };
  });
  await ui.page.click(point.x, point.y);
  await ui.until(async () => (await ui.saved()).settings[key] === value, `${label} preference saved`);
}

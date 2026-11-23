import assert from "node:assert/strict";

function contains(outer, inner) {
  return inner.x >= outer.x && inner.y >= outer.y
    && inner.x + inner.width <= outer.x + outer.width
    && inner.y + inner.height <= outer.y + outer.height;
}

export function sidebarViewport(view) {
  const scroll = view.elements.find(element => element.name === "SidebarScroll");
  if (!scroll) return null;
  const x = Math.max(0, scroll.x), y = Math.max(0, scroll.y);
  const right = Math.min(view.width, scroll.x + scroll.width);
  const bottom = Math.min(view.height, scroll.y + scroll.height);
  return right > x && bottom > y ? { x, y, width: right - x, height: bottom - y } : null;
}

// Prepare input targets before a press/hold. Never scroll during a drag or as
// cleanup after a rejected gesture: that would obscure capture/release failures.
export async function revealSidebarControls(ui, matches, label, { touch, edgeMargin = 0, fractions = matches.map(() => .5) } = {}) {
  assert(fractions.length === matches.length && fractions.every(value => value >= 0 && value <= 1));
  let scanningDown = false;
  for (let attempt = 0; attempt < 40; attempt++) {
    const view = await ui.snapshot(), clip = sidebarViewport(view);
    assert(clip, `The sidebar scroll viewport is visible while locating ${label}`);
    const inside = (row, index) => row && contains(clip, row)
      && row.y + row.height * fractions[index] >= clip.y + edgeMargin
      && row.y + row.height * fractions[index] <= clip.y + clip.height - edgeMargin;
    const rows = matches.map(match => {
      const found = view.elements.filter(match);
      assert(found.length <= 1, `Ambiguous sidebar target: ${label}`);
      return found[0];
    });
    if (rows.every((row, index) => row?.hit && inside(row, index))) return rows;

    let delta;
    if (rows.every(Boolean)) {
      const top = Math.min(...rows.map(row => row.y));
      const bottom = Math.max(...rows.map(row => row.y + row.height));
      assert(bottom - top <= clip.height, `The sidebar targets for ${label} must fit together before input`);
      const contacts = rows.map((row, index) => row.y + row.height * fractions[index]);
      assert(Math.max(...contacts) - Math.min(...contacts) <= clip.height - edgeMargin * 2,
        `The sidebar contacts for ${label} must fit outside the edge-scroll zones`);
      delta = (top + bottom) / 2 - (clip.y + clip.height / 2);
      // A fully contained target can be briefly obscured during native layout.
      // Waiting does not dismiss an obstruction or manufacture a successful hit.
      if (rows.every(inside)) delta = 0;
    } else {
      // inspectView omits controls outside the canvas. Use the rendered content
      // origin to scan from the top without reading or changing application state.
      const content = view.elements.find(element => element.name === "ListNavigation");
      assert(content, `The native navigation content exists while locating ${label}`);
      if (content.y >= clip.y - 1) scanningDown = true;
      delta = scanningDown ? 300 : -300;
    }
    if (delta) {
      if (touch) {
        const distance = Math.min(Math.max(Math.abs(delta), 32), 300, clip.height * .6);
        const x = clip.x + clip.width / 2, middle = clip.y + clip.height / 2;
        const half = Math.sign(delta) * distance / 2;
        await touch.swipe({ x, y: middle + half }, { x, y: middle - half });
      } else {
        await ui.page.wheel(clip.x + clip.width / 2, clip.y + clip.height / 2,
          0, Math.max(-300, Math.min(300, delta)));
      }
    }
    await ui.page.evaluate("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))");
  }
  throw new Error(`Cannot reveal the native sidebar targets: ${label}`);
}

export async function revealSidebarControl(ui, match, label, options) {
  return (await revealSidebarControls(ui, [match], label, options))[0];
}

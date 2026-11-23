import assert from "node:assert/strict";
import { sidebarViewport } from "./sidebar-viewport.mjs";

// A native tooltip can cover a row's pointer hit point while the row correctly
// owns keyboard focus. Assert focus and visible geometry without moving the
// mouse, dismissing the tooltip, or scrolling after the key being verified.
export async function expectSidebarFocus(ui, match, label) {
  return ui.until(async () => {
    const view = await ui.snapshot(), clip = sidebarViewport(view);
    const rows = view.elements.filter(match);
    assert(rows.length <= 1, `Ambiguous sidebar focus target: ${label}`);
    const row = rows[0];
    return row?.focused && clip && row.width > 0 && row.height > 0
      && row.x >= clip.x && row.y >= clip.y
      && row.x + row.width <= clip.x + clip.width
      && row.y + row.height <= clip.y + clip.height ? row : null;
  }, label);
}

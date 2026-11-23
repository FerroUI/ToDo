import { dragControls } from "./mouse-drag.mjs";
import { revealSidebarControl, revealSidebarControls } from "./sidebar-viewport.mjs";

export async function dragSidebar(ui, from, to, { fraction = 0.5, cancel = false } = {}) {
  await revealSidebarControls(ui, [from, to], "navigation drag source and drop target");
  await dragControls(ui, from, to, { fraction, cancel });
}

export async function rightClick(ui, match) {
  const item = await revealSidebarControl(ui, match, "navigation context target");
  const x = item.x + item.width / 2, y = item.y + item.height / 2;
  await ui.page.mouseMove(x, y);
  for (const [type, buttons] of [["mousePressed", 2], ["mouseReleased", 0]]) {
    await ui.page.send("Input.dispatchMouseEvent", { type, x, y, button: "right", buttons, clickCount: 1, pointerType: "mouse" });
  }
}

export async function pressFunction(page, number) {
  for (const type of ["rawKeyDown", "keyUp"]) {
    await page.send("Input.dispatchKeyEvent", { type, key: `F${number}`, code: `F${number}`, windowsVirtualKeyCode: 111 + number });
  }
}

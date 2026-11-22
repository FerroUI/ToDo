import { sleep } from "../../FerroUI/scripts/browser/harness.mjs";

export async function dragSidebar(ui, from, to, { fraction = 0.5, cancel = false } = {}) {
  const source = await ui.control(from, "navigation drag source");
  const target = await ui.control(to, "navigation drop target");
  const x = source.x + source.width / 2, y = source.y + source.height / 2;
  const tx = target.x + target.width / 2, ty = target.y + target.height * fraction;
  const move = (x, y) => ui.page.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, buttons: 1, pointerType: "mouse" });
  const escape = type => ui.page.send("Input.dispatchKeyEvent", { type, key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
  await ui.page.mouseMove(x, y);
  await ui.page.mouseDown(x, y);
  await move(x + 12, y);
  await move(x + 14, y);
  await move(tx, ty);
  if (cancel) { await escape("rawKeyDown"); await move(tx + 1, ty); }
  await ui.page.mouseUp(tx, ty);
  if (cancel) await escape("keyUp");
  await sleep(100);
}

export async function rightClick(ui, match) {
  const item = await ui.control(match, "navigation context target");
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

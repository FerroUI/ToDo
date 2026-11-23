import assert from "node:assert/strict";
import { button, named, touchInput } from "./touch-input.mjs";
import { reviewTouch } from "./touch-review.mjs";
import { revealSidebarControl } from "./sidebar-viewport.mjs";

export async function verifyTouch(ui, output) {
  const { page, saved, snapshot, control, clickControl, edit, until, resize } = ui;
  await resize(1280, 800);
  async function createList(name) {
    await clickControl(button("New list"), `create ${name}`);
    await edit("ListName", name, true);
    return until(async () => (await saved()).lists.find(list => list.name === name), `${name} is saved`);
  }
  const destination = await createList("Touch destination");
  const source = await createList("Touch review");
  for (let index = 1; index <= 16; index++) await edit("NewTask", `Touch ${String(index).padStart(2, "0")}`, true);
  await until(async () => (await saved()).tasks.filter(task => task.list_id === source.id).length === 16, "the touch fixture is saved");
  const touch = touchInput(ui);
  const ctx = {
    ...ui, touch, source, destination,
    row: title => control(button(title), `task row ${title}`),
    task: async title => (await saved()).tasks.find(task => task.title === title),
    dismissStatus: async () => {
      if ((await snapshot()).elements.some(element => element.hit && button("Dismiss message")(element))) {
        await touch.tapControl(button("Dismiss message"), "dismiss fixture status");
      }
    },
  };
  await resize(390, 844);
  // Prime a real mouse tooltip before switching input devices. Touch must close
  // the open popup and prevent its native timer from resurfacing over task rows.
  const title = await control(named("RenameList"), "tooltip owner before touch");
  await page.mouseMove(title.x + title.width / 2, title.y + title.height / 2);
  await until(async () => (await snapshot()).elements.some(element =>
    element.type === "TextBlock" && element.text === "Rename list"), "native mouse tooltip opens");
  const enabled = await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 2 });
  assert(!enabled.error, enabled.error?.message);
  try {
    await ctx.dismissStatus();
    await reviewTouch(ctx, output);
    await resize(1280, 800);
    // Remove only this scenario's lists so the following sidebar layout fixture stays unchanged.
    for (const list of [source, destination]) {
      await revealSidebarControl(ui, named(`NavigateList(${list.id})`), `disposable ${list.name}`, { touch });
      await touch.tapControl(named(`NavigateList(${list.id})`), `open disposable ${list.name}`);
      await touch.tapControl(button("List options"), "fixture list options");
      await touch.tapControl(element => element.type.endsWith("MenuItem") && element.automationName === "Delete list", "delete the disposable fixture list");
      await touch.tapControl(button("Delete"), "confirm fixture cleanup");
      await until(async () => !(await saved()).lists.some(item => item.id === list.id), "fixture list is removed");
    }
    await ctx.dismissStatus();
    console.log("ok: touch fixture cleaned up and desktop restored");
  } finally {
    await touch.cancel();
    const disabled = await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
    assert(!disabled.error, disabled.error?.message);
  }
}

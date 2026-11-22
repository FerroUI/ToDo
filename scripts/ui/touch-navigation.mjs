import assert from "node:assert/strict";
import { button, center, named, noDetails, noRail } from "./touch-input.mjs";

async function browserBack(page) {
  const history = await page.send("Page.getNavigationHistory");
  assert(!history.error, history.error?.message);
  assert(history.currentIndex > 0, "the app surface exposes a browser Back destination");
  const back = await page.send("Page.navigateToHistoryEntry", { entryId: history.entries[history.currentIndex - 1].id });
  assert(!back.error, back.error?.message);
}

export async function verifyCompactNavigation(ctx) {
  const { page, touch, control, saved, snapshot, edit, until } = ctx;
  await touch.tapControl(button("Touch 01"), "ordinary finger tap on a task");
  assert.equal((await control(named("TaskTitle"), "touch-opened details")).text, "Touch 01");
  assert(!(await snapshot()).elements.some(element => element.type === "TextBlock" && element.text === "Rename list"),
    "real touch dismisses the preceding mouse tooltip");
  const beforeNote = await saved();
  await edit("TaskNotes", "Touch browser Back draft");
  const url = await page.evaluate("location.href");
  await browserBack(page);
  await until(async () => !(await snapshot()).elements.some(named("TaskTitle")), "browser Back closes details");
  await until(async () => (await ctx.task("Touch 01")).notes === "Touch browser Back draft", "browser Back flushes the focused note");
  assert.deepEqual(await saved(), {
    ...beforeNote,
    tasks: beforeNote.tasks.map(task => task.title === "Touch 01" ? { ...task, notes: "Touch browser Back draft" } : task),
  }, "browser Back preserves all document data except the intended note edit");
  assert.equal(await page.evaluate("location.href"), url, "browser Back preserves the document URL");
  await control(named("NewTask"), "the list survives browser Back");

  await touch.tapControl(button("Touch 01"), "reopen details for edge gestures");
  const before = await saved();
  await touch.swipe({ x: 12, y: 300 }, { x: 48, y: 300 });
  await control(named("TaskTitle"), "short edge swipe preserves details");
  await touch.swipe({ x: 12, y: 300 }, { x: 132, y: 300 });
  await control(named("NewTask"), "committed edge Back returns to the list");
  await noDetails(ctx, "edge Back closes compact details");
  assert.deepEqual(await saved(), before, "edge Back preserves the complete document");
  const beforeRename = await saved();
  await touch.tapControl(named("RenameList"), "touch the inline list title");
  await edit("ListName", "Uncommitted touch name");
  await browserBack(page);
  await until(async () => !(await snapshot()).elements.some(named("ListName")), "browser Back cancels the active rename");
  await control(named("RenameList"), "the original list remains open after rename cancellation");
  await control(element => element.type === "TextBlock" && element.text === ctx.source.name, "the original list title is retained");
  assert.deepEqual(await saved(), beforeRename, "browser Back must not commit or navigate past a transient rename");
  assert.equal(await page.evaluate("location.href"), url, "rename cancellation retains the URL");
  await touch.tapControl(button("Show navigation"), "open compact navigation by touch");
  await control(named("SearchTasks"), "open navigation drawer");
  await touch.swipe({ x: 240, y: 400 }, { x: 110, y: 400 });
  await until(async () => !(await snapshot()).elements.some(named("SearchTasks")), "left swipe dismisses the drawer");
  await control(named("NewTask"), "the underlying composer remains available");
  console.log("ok: finger tap, browser Back note flush/rename cancellation, thresholded edge Back, and drawer dismissal");
}

export async function verifyScrollAndRefresh(ctx) {
  const { touch, saved, snapshot, control, until, edit, capture, page } = ctx;
  const tasks = (await saved()).tasks.filter(task => task.list_id === ctx.source.id).sort((a, b) => a.position - b.position);
  const first = await ctx.row(tasks[0].title);
  const titles = new Set(tasks.map(task => task.title));
  const rows = (await snapshot()).elements.filter(element => element.hit && element.type === "Button" && titles.has(element.automationName)).sort((a, b) => a.y - b.y);
  const marker = rows.at(-2);
  assert(marker, "the fixture supplies enough visible rows to scroll");
  const beforeScroll = await saved();
  const from = center(marker);
  await touch.swipe(from, { x: from.x + 2, y: from.y - 180 });
  await until(async () => {
    const moved = (await snapshot()).elements.find(element => element.hit && button(marker.automationName)(element));
    return !moved || moved.y < marker.y - 70;
  }, "vertical finger movement scrolls the task list");
  await noDetails(ctx, "scrolling cannot open the touched row");
  await noRail(ctx);
  assert.deepEqual(await saved(), beforeScroll, "scrolling cannot change the document");
  // Return to the top using fingers as well. A contact that started below the top remains scrolling.
  for (let attempt = 0; attempt < 8; attempt++) {
    const top = (await snapshot()).elements.find(element => element.hit && button(tasks[0].title)(element));
    if (top && Math.abs(top.y - first.y) <= 1) break;
    await touch.swipe({ x: 195, y: 320 }, { x: 195, y: 620 });
  }
  await until(async () => Math.abs((await ctx.row(tasks[0].title)).y - first.y) <= 1, "touch scrolling returns to the list top");
  await edit("NewTask", "Touch draft stays");
  const baseline = await saved();
  const timeOrigin = await page.evaluate("performance.timeOrigin");
  const feedback = text => until(async () => (await snapshot()).elements.some(element => element.name === "TouchGestureMessage" && element.text?.includes(text)), text);
  const noFeedback = () => until(async () => !(await snapshot()).elements.some(named("TouchGestureMessage")), "gesture feedback clears");
  for (const [distance, cancel] of [[35, false], [110, true], [110, false]]) {
    const start = center(await ctx.row(tasks[0].title));
    await touch.start(start);
    await touch.move({ x: start.x, y: start.y + distance });
    await feedback(distance < 80 ? "Pull to refresh" : "Release to refresh");
    assert.deepEqual(await saved(), baseline, "pulling cannot modify data before release");
    if (distance === 110 && !cancel) await capture("touch-pull-refresh");
    if (cancel) await touch.cancel(); else await touch.end();
    await noFeedback();
    assert.deepEqual(await saved(), baseline, "partial, cancelled and completed local refresh preserve the document");
    assert.equal((await control(named("NewTask"), "composer after pull gesture")).text, "Touch draft stays", "refresh preserves the unsubmitted draft");
    assert.equal(await page.evaluate("performance.timeOrigin"), timeOrigin, "refresh keeps the mounted application rather than reloading");
  }
  await edit("NewTask", "");
  await noDetails(ctx, "refresh does not activate the touched task");
  console.log("ok: native vertical scrolling, partial/cancelled/full pull refresh, and preserved document/draft");
}

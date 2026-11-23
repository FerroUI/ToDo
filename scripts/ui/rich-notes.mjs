import assert from "node:assert/strict";
import { button, named, touchInput } from "./touch-input.mjs";
import { typeText } from "./text-input.mjs";
import { revealSidebarControl } from "./sidebar-viewport.mjs";

// Native RichEditBox commands, selection, history, IME and read-view links.
// All document mutations below use ordinary input; saved() never flushes drafts.
export async function verifyRichNotes(ui) {
  const { page, saved, snapshot, control, clickControl, edit, until, resize, capture, focusControl } = ui;
  await resize(1280, 1000);
  await clickControl(button("New list"), "create rich-note review list");
  await edit("ListName", "Rich note review", true);
  const list = (await saved()).lists.find(item => item.name === "Rich note review");
  await edit("NewTask", "Rich note document", true);
  const task = (await saved()).tasks.find(item => item.title === "Rich note document");
  const current = async () => (await saved()).tasks.find(item => item.id === task.id);
  await clickControl(button(task.title), "rich-note task");
  const visible = async name => {
    for (let attempt = 0; attempt < 12; attempt++) {
      const view = await snapshot(), item = view.elements.find(element => element.name === name && element.hit);
      if (item && item.y > 70 && item.y + item.height < view.height - 35) return item;
      await page.wheel(view.width - 80, view.height - 160, 0, item?.y < 70 ? -180 : 180);
    }
    return control(named(name), name);
  };
  const click = async name => { await visible(name); await clickControl(named(name), name); };
  const save = () => page.press("Enter", ["ctrl"]);
  const text = "日本語 中文 한국어 한글 👩🏽‍💻 #RichNotes";
  await visible("TaskNotes"); await edit("TaskNotes", text);
  await page.press("a", ["ctrl"]);
  for (const name of ["NoteBold", "NoteItalic", "NoteUnderline", "NoteStrike"]) await click(name);
  await save();
  await until(async () => {
    const value = await current();
    return value.notes === text && ["strong", "em", "u", "s"].every(tag => value.notes_html?.includes(`<${tag}>`));
  }, "native character formatting persists without changing selected Unicode text");
  await page.press("z", ["ctrl"]); await save();
  await until(async () => !(await current()).notes_html?.includes("<s>"), "native undo removes only the last formatting command");
  await click("NoteRedo"); await save();
  await until(async () => (await current()).notes_html?.includes("<s>"), "native redo retains the note selection and earlier emphasis");
  await page.press("a", ["ctrl"]); await click("NoteBullets");
  await page.press("End", ["ctrl"]); await page.press("Enter"); await typeText(page, "Second item"); await save();
  await until(async () => {
    const value = await current(); return value.notes === `• ${text}\n• Second item` && value.notes_html?.includes("<ul>");
  }, "native Enter continues a bulleted paragraph");
  await page.press("a", ["ctrl"]); await click("NoteNumbered"); await save();
  await until(async () => (await current()).notes === `1. ${text}\n2. Second item` && (await current()).notes_html?.includes("<ol>"), "numbered lists retain inline formatting");
  await page.press("End", ["ctrl"]); await page.press("Home", ["shift"]);
  await click("NoteLink"); await edit("NoteLinkAddress", "https://example.com/rich-notes"); await click("NoteApplyLink"); await save();
  await until(async () => (await current()).notes_html?.includes('href="https://example.com/rich-notes"'), "link dialog applies to the retained native selection");

  // Browser IME uses Chromium's real composition route into the native editor.
  await page.press("End", ["ctrl"]);
  const beforeIme = (await current()).notes;
  let result = await page.send("Input.imeSetComposition", { text: "候補", selectionStart: 2, selectionEnd: 2 });
  assert(!result.error, result.error?.message);
  await until(async () => (await control(named("TaskNotes"), "composing note")).text.endsWith("候補"), "native preedit appears");
  result = await page.send("Input.imeSetComposition", { text: "", selectionStart: 0, selectionEnd: 0 });
  assert(!result.error, result.error?.message);
  await until(async () => (await control(named("TaskNotes"), "cancelled composition")).text === beforeIme, "IME cancellation restores rich text");
  result = await page.send("Input.imeSetComposition", { text: "日本", selectionStart: 2, selectionEnd: 2 });
  assert(!result.error, result.error?.message);
  await typeText(page, "日本語"); await save();
  await until(async () => (await current()).notes === `${beforeIme}日本語`, "IME commits once without flattening formatting");

  await click("NotePreviewToggle");
  const linkPoint = label => until(() => page.evaluate(`(() => {
    const boxes = [...document.querySelectorAll('[role="link"]')].filter(node => node.getAttribute('aria-label') === ${JSON.stringify(label)})
      .map(node => node.getBoundingClientRect()).filter(box => box.width > 0 && box.height > 0);
    return boxes.length === 1 ? { x: boxes[0].x + boxes[0].width / 2, y: boxes[0].y + boxes[0].height / 2 } : null;
  })()`), label);
  await visible("NotePreview"); await linkPoint("Open https://example.com/rich-notes");
  let point = await linkPoint("Search for #RichNotes");
  await page.click(point.x, point.y);
  await control(button("Close search"), "note hashtag search");
  assert.equal((await control(named("SearchTasks"), "note hashtag query")).text, "#RichNotes");
  await clickControl(button("Close search"), "return from note search");
  await clickControl(button(task.title), "reopen rich-note task");
  await visible("NewStep"); await edit("NewStep", "Review #StepNotes https://example.com/steps", true);
  const step = (await current()).steps[0];
  await click(`EditStep${step.id}`); await edit(`Step${step.id}`, "Review #StepNotes https://example.com/steps updated", true);
  await until(async () => (await current()).steps[0].title.endsWith("updated"), "readable step links retain an explicit native editor");
  await click(`EditStep${step.id}`); await edit(`Step${step.id}`, "Ordinary literal step", true);
  await control(element => element.name === `Step${step.id}` && element.focused, "removing the last step link retains native editor focus");
  await edit(`Step${step.id}`, "Review #StepNotes https://example.com/steps updated", true);
  await control(element => element.name === `EditStep${step.id}` && element.focused, "adding a step link returns to its explicit edit action");
  await visible(`StepRead${step.id}`);
  await focusControl(named(`StepRead${step.id}`), "linked step text"); await page.press("Tab"); await page.press("Enter");
  await control(button("Close search"), "keyboard step hashtag search");
  assert.equal((await control(named("SearchTasks"), "step hashtag query")).text, "#StepNotes");
  await clickControl(button("Close search"), "return from step search");
  await clickControl(button(task.title), "open task for compact note preview");
  await resize(390, 1000); await click("NotePreviewToggle"); await visible("NotePreview");
  point = await linkPoint("Search for #RichNotes");
  const touch = touchInput(ui);
  result = await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 2 }); assert(!result.error, result.error?.message);
  try { await touch.tap(point); await control(button("Close search"), "touch note hashtag search"); }
  finally { await touch.cancel(); await page.send("Emulation.setTouchEmulationEnabled", { enabled: false }); }
  await capture("rich-note-search-narrow");
  await clickControl(button("Close search"), "return from compact note search");
  await resize(1280, 1000);
  await clickControl(button(task.title), "reopen note after navigation");
  await visible("TaskNotes");
  assert.equal((await control(named("TaskNotes"), "restored rich editor")).text, (await current()).notes, "switching read/edit and navigating retains the complete note");
  const beforeReload = await current();
  await ui.navigate("Page.reload", { ignoreCache: true });
  await revealSidebarControl(ui, named(`NavigateList(${list.id})`), "rich-note list after reload");
  await clickControl(named(`NavigateList(${list.id})`), "open the saved rich-note list");
  await clickControl(button(task.title), "reopen the rich-note document after reload");
  await visible("TaskNotes");
  assert.deepEqual(await current(), beforeReload, "reload preserves every rich-note task field, inline format, list, and link");
  assert.equal((await control(named("TaskNotes"), "reloaded CJK rich editor")).text, beforeReload.notes,
    "reload retains Japanese, Chinese, Korean syllables, emoji, and the committed composition in the native editor");
  await capture("rich-note-editor");
  await clickControl(button("Close details"), "close rich-note task");
  await clickControl(button("List options"), "rich-note fixture options");
  await clickControl(element => element.type.endsWith("MenuItem") && element.automationName === "Delete list", "delete rich-note fixture");
  await clickControl(button("Delete"), "confirm rich-note fixture cleanup");
  await until(async () => !(await saved()).lists.some(item => item.id === list.id), "rich-note fixture removed");
  if ((await snapshot()).elements.some(element => element.hit && button("Dismiss message")(element))) await clickControl(button("Dismiss message"), "dismiss rich-note status");
  await resize(1280, 800);
  console.log("ok: native rich-note Japanese/Chinese/Korean/emoji selection, formatting, lists, undo/redo, links, IME, reload, and pointer/keyboard/touch note/step hashtag navigation");
}

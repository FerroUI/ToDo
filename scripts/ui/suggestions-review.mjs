import assert from "node:assert/strict";
import { right } from "./layout-geometry.mjs";

const button = label => element => element.type.endsWith("Button") && element.automationName === label;
const named = name => element => element.name === name;

export async function verifySuggestions(ui, chooseTheme) {
  const { page, snapshot, control, clickControl, until, resize, capture } = ui;
  const viewport = await snapshot();
  const saved = async () => JSON.parse(await page.evaluate("localStorage.getItem('ferroui.todo.demo.v1') || '{}'"));
  const before = await saved();
  await resize(1280, 800);
  await clickControl(button("Suggestions"), "open populated My Day suggestions");
  const suggestion = await until(async () => (await snapshot()).elements.find(element => element.hit
    && element.name?.startsWith("SuggestionTask")), "a real suggested task");
  const id = Number(suggestion.name.slice("SuggestionTask".length));
  const task = before.tasks.find(task => task.id === id);
  assert(task, "the suggestion refers to a saved demo task");
  for (const [width, label] of [[1280, "wide"], [390, "narrow"]]) {
    await resize(width, 844);
    for (const [match, label] of [[button("Close suggestions"), "suggestions close"],
      [named(`SuggestionTask${id}`), "suggested task"], [named(`SuggestionAdd${id}`), "suggestion add action"]]) {
      const item = await control(match, label);
      assert(item.x >= -1 && right(item) <= width + 1, `${label} fits the viewport`);
    }
    await capture(`suggestions-${label}`);
  }
  await clickControl(named(`SuggestionAdd${id}`), "add the displayed suggestion to My Day");
  await until(async () => (await saved()).tasks.find(item => item.id === id).my_day, "the suggestion is added to My Day");
  await until(async () => !(await snapshot()).elements.some(named(`SuggestionTask${id}`)), "the added task leaves suggestions");
  assert.deepEqual((await saved()).tasks.filter(item => item.id !== id), before.tasks.filter(item => item.id !== id),
    "adding one suggestion preserves all other task data");
  await clickControl(button("Close suggestions"), "return from suggestions");
  await clickControl(button(task.title), "open the added My Day task");
  await clickControl(button("Remove from My Day"), "restore the demo task's original day");
  await until(async () => (await saved()).tasks.find(item => item.id === id).my_day === task.my_day,
    "the temporary suggestion addition is reversed");
  await page.press("Escape");
  await control(named("NewTask"), "My Day composer after suggestions");
  await chooseTheme("Dark");
  await clickControl(button("Suggestions"), "dark compact suggestions");
  await control(named(`SuggestionTask${id}`), "restored suggestion in the dark theme");
  await capture("suggestions-dark-narrow");
  await clickControl(button("Close suggestions"), "close dark suggestions");
  await chooseTheme(before.settings.theme);
  await resize(viewport.width, viewport.height);
  console.log("ok: populated desktop/compact suggestions fit, add the selected task, and preserve other task data");
}

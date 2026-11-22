import { verifySuggestions } from "./suggestions-review.mjs";

export async function verifyDemo(ui, output) {
  const { page, snapshot, healthy, until, control, clickControl, clickText, navigate, capture } = ui;
  const demo = new URL(page.url);
  demo.search = "?demo=1&nooffline=1";
  await navigate("Page.navigate", { url: demo.href });
  await control(element => element.type.endsWith("TextBlock") && element.text === "Plan the week ahead", "demo task rows");
  await capture("my-day");
  await clickText("Review the new designs");
  await control(element => element.name === "TaskTitle", "demo task details");
  await capture("details");
  await page.press("Escape");
  await until(async () => !(await snapshot()).elements.some(element => element.name === "TaskTitle"), "Escape closes task details");
  const chooseTheme = async theme => {
    await clickControl(element => element.automationName === "Settings", "Settings button");
    await clickText(`${theme} theme`);
    await until(() => page.evaluate(`JSON.parse(localStorage.getItem('ferroui.todo.demo.v1') || '{}').settings?.theme === ${JSON.stringify(theme)}`), `${theme} theme is saved`);
    await clickControl(element => element.automationName === "Close settings", "Close settings button");
    await until(async () => !(await snapshot()).elements.some(element => element.automationName === "Close settings"), "Settings closes");
  };
  await chooseTheme("Dark");
  await capture("my-day-dark");
  await chooseTheme("Light");
  await page.resize(390, 844);
  await until(async () => (await snapshot()).width < 400, "the narrow viewport");
  await control(element => element.name === "NewTask", "narrow task composer");
  await capture("my-day-narrow");
  await verifySuggestions(ui, chooseTheme);
  healthy();
  console.log(`ok: desktop, task-detail, dark-theme and narrow renders; ${page.requests.length} requests; previews in ${output}`);
}

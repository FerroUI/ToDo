import assert from "node:assert/strict";
import { sleep } from "../../FerroUI/scripts/browser/harness.mjs";

export async function verifyDrafts(ui, taskId) {
  const { page, resize, saved, snapshot, control, clickControl, edit, until, navigate } = ui;
  await resize(1280, 800);
  const draft = "Step draft retained across settings";
  await edit("NewStep", draft);
  await clickControl(element => element.automationName === "Settings", "Settings");
  await control(element => element.automationName === "Close settings" && element.focused, "opening Settings transfers focus into its pane");
  await control(element => element.name === "AccountName", "account settings");
  await clickControl(element => element.automationName === "Close settings", "Close settings");
  assert.equal((await control(element => element.name === "NewStep", "restored step composer")).text, draft);
  assert(!(await saved()).tasks.find(task => task.id === taskId).steps.some(step => step.title === draft),
    "an unsubmitted draft must not become a saved step");
  await control(element => element.automationName === "Settings" && element.focused, "Settings trigger regains focus");
  await resize(390, 844);
  assert.equal((await control(element => element.name === "NewStep", "narrow step draft")).text, draft);
  await resize(1280, 800);
  assert.equal((await control(element => element.name === "NewStep", "wide step draft")).text, draft);
  await edit("NewStep", "");
  await page.press("Escape");
  await control(element => element.name === "NewTask", "list composer after closing details");

  const query = "Unsubmitted narrow search";
  await edit("SearchTasks", query);
  // Do not wait for another control: resize while the desktop debounce is pending.
  await resize(390, 844);
  await sleep(250);
  const title = (await saved()).tasks.find(task => task.id === taskId).title;
  await control(element => element.type.endsWith("TextBlock") && element.text === title,
    "the original list stays active after the desktop search timer fires at a compact width");
  await clickControl(element => element.automationName === "Show navigation", "show compact navigation");
  assert.equal((await control(element => element.name === "SearchTasks", "preserved search draft")).text, query);
  // Clearing in compact mode remains a draft operation, too.
  await edit("SearchTasks", "");
  await page.press("Escape");
  await until(async () => !(await snapshot()).elements.some(element => element.name === "SearchTasks"), "compact navigation closes");

  await resize(1280, 800);
  await clickControl(element => element.automationName === "Settings", "Settings before reload");
  const account = "Adaptive account";
  await edit("AccountName", account);
  // No blur or export: the host must flush this focused field on document departure.
  await navigate("Page.reload", { ignoreCache: true });
  assert.equal((await saved()).settings.account_name, account, "focused account-name edits survive reload");
  console.log("ok: step drafts survive settings and resize; compact search cancels desktop navigation; focused account edits survive reload");
}

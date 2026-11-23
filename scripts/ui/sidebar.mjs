import assert from "node:assert/strict";
import { centerY, close, one, sameBounds, textPresenter } from "./layout-geometry.mjs";
import { dragSidebar, pressFunction, rightClick } from "./sidebar-input.mjs";
import { revealSidebarControl, revealSidebarControls } from "./sidebar-viewport.mjs";
import { expectSidebarFocus } from "./sidebar-keyboard.mjs";

const button = label => element => element.type.endsWith("Button") && element.automationName === label;
const groupButton = id => element => element.name === `Group${id}`;
const listButton = id => element => element.name === `NavigateList(${id})`;
const menu = label => element => element.type.endsWith("MenuItem") && element.automationName === label;

export async function verifySidebar(ui) {
  const { page, saved, control, clickControl, edit, until, resize, snapshot, capture, navigate } = ui;
  await resize(1280, 1000);
  const createList = async name => {
    await clickControl(button("New list"), "New list");
    await edit("ListName", name);
    await until(async () => (await control(element => element.name === "ListName", "list-name editor")).text === name,
      `the complete Unicode name ${name} reaches the editor`);
    await page.press("Enter");
    return until(async () => (await saved()).lists.find(list => list.name === name), `saved ${name}`);
  };
  const createGroup = async name => {
    await clickControl(button("Create a new group"), "Create group");
    await edit("GroupName", name, true);
    return until(async () => (await saved()).groups.find(group => group.name === name), `saved ${name}`);
  };
  const first = await createList("📚 Sidebar A");
  await edit("NewTask", "Sidebar retained task", true);
  const task = await until(async () => (await saved()).tasks.find(task => task.title === "Sidebar retained task"), "a task for navigation durability");
  const second = await createList("Sidebar B");
  const group = await createGroup("Sidebar group");
  await revealSidebarControl(ui, element => element.name === `EmptyGroup${group.id}`, "empty-group drop guidance");
  const other = await createGroup("Sidebar other");

  await dragSidebar(ui, listButton(first.id), groupButton(group.id));
  await until(async () => (await saved()).lists.find(list => list.id === first.id).group_id === group.id, "drop list into a group");
  await expectSidebarFocus(ui, listButton(first.id), "focus follows the relocated list");
  await dragSidebar(ui, listButton(second.id), groupButton(group.id), { fraction: 0.85 });
  await until(async () => {
    const order = (await saved()).sidebar_order;
    return order.indexOf(group.id) + 1 === order.indexOf(second.id);
  }, "dropping at a group edge changes root order without grouping");
  await dragSidebar(ui, groupButton(other.id), listButton(second.id), { fraction: 0.15 });
  await until(async () => {
    const order = (await saved()).sidebar_order;
    return order.indexOf(other.id) + 1 === order.indexOf(second.id);
  }, "group and list roots interleave");
  assert.equal((await saved()).groups.find(item => item.id === other.id).collapsed, false, "drag release must not toggle the source group");

  const beforeCancel = await saved();
  await dragSidebar(ui, listButton(second.id), groupButton(other.id), { cancel: true });
  assert.deepEqual(await saved(), beforeCancel, "Escape cancels sidebar dragging even after another move and release");
  await dragSidebar(ui, listButton(second.id), listButton(first.id), { fraction: 0.15 });
  await until(async () => {
    const lists = (await saved()).lists;
    const a = lists.find(list => list.id === first.id), b = lists.find(list => list.id === second.id);
    return b.group_id === group.id && b.position < a.position;
  }, "a list can move into a group before an existing member");
  const emojiRow = await control(listButton(first.id), "emoji list row");
  assert.equal(emojiRow.automationName, "📚 Sidebar A", "the accessible name preserves the full list title");
  const rowLabels = (await snapshot()).elements.filter(element => element.type.endsWith("TextBlock")
    && element.x >= emojiRow.x && element.x + element.width <= emojiRow.x + emojiRow.width + 1
    && element.y >= emojiRow.y && element.y + element.height <= emojiRow.y + emojiRow.height + 1);
  assert(rowLabels.some(element => element.text === "Sidebar A"), "the sidebar label omits the leading emoji and separator");
  assert(rowLabels.some(element => element.text === "📚"), "the leading emoji occupies the icon column");
  assert(!rowLabels.some(element => element.text === "📚 Sidebar A"), "the emoji is not duplicated in the visible label");
  await capture("sidebar-groups");

  await revealSidebarControls(ui, [groupButton(group.id), listButton(first.id), listButton(second.id)], "group and members before keyboard navigation");
  await ui.focusControl(groupButton(group.id), "group keyboard focus");
  await page.press("ArrowLeft");
  await until(async () => (await saved()).groups.find(item => item.id === group.id).collapsed, "Left collapses a group");
  assert(!(await snapshot()).elements.some(listButton(first.id)), "collapsed children leave the visible navigation");
  await page.press("ArrowRight");
  await until(async () => !(await saved()).groups.find(item => item.id === group.id).collapsed, "Right expands a group");
  await page.press("ArrowRight");
  await expectSidebarFocus(ui, listButton(second.id), "Right enters the first group member");
  await page.press("ArrowDown");
  await expectSidebarFocus(ui, listButton(first.id), "Down follows visible sibling order");
  await page.press("ArrowLeft");
  await expectSidebarFocus(ui, groupButton(group.id), "Left returns to the parent group");
  const beforeName = await snapshot();
  const optionsBefore = one(beforeName.elements, element => element.name === `GroupOptions${group.id}`, "group options before rename");
  const memberBefore = one(beforeName.elements, listButton(second.id), "first group member before rename");
  const labelBefore = one(beforeName.elements, element => /TextBlock$/.test(element.type) && element.text === group.name,
    "group label before rename");
  await pressFunction(page, 2);
  await expectSidebarFocus(ui, element => element.name === "GroupName", "F2 opens inline group naming");
  await until(async () => {
    const view = await snapshot();
    sameBounds(one(view.elements, element => element.name === `GroupOptions${group.id}`, "group options during rename"),
      optionsBefore, "group options stay in place during rename");
    close(one(view.elements, listButton(second.id), "member during group rename").y, memberBefore.y,
      "group rename preserves the following member's position");
    const text = textPresenter(view, "GroupName");
    close(text.x, labelBefore.x, "group editor preserves the label's text origin");
    close(centerY(text), centerY(labelBefore), "group editor preserves the label's vertical center");
    return true;
  }, "group renaming preserves label and member geometry");
  await capture("sidebar-group-rename");
  await edit("GroupName", "Sidebar renamed", true);
  await control(element => groupButton(group.id)(element) && element.automationName === "Sidebar renamed", "renamed group header");
  close(one((await snapshot()).elements, listButton(second.id), "member after group rename").y, memberBefore.y,
    "saving the group name preserves the following member's position");

  await rightClick(ui, listButton(second.id));
  await clickControl(menu("Remove from group"), "Remove from group");
  await until(async () => (await saved()).lists.find(list => list.id === second.id).group_id === null, "direct member removal keeps its list");
  const beforeUngroup = await saved();
  await clickControl(element => element.name === `GroupOptions${group.id}`, "Group options");
  await clickControl(menu("Ungroup lists"), "Ungroup lists");
  await until(async () => !(await saved()).groups.some(item => item.id === group.id), "Ungroup deletes only the container");
  const after = await saved();
  const expected = beforeUngroup.sidebar_order.flatMap(id => id === group.id ? [first.id] : [id]);
  assert.deepEqual(after.sidebar_order, expected, "ungrouped members occupy their former group position");
  assert.deepEqual(after.tasks.find(item => item.id === task.id), task, "navigation operations preserve every task field");
  await resize(1280, 800);
  await navigate("Page.reload", { ignoreCache: true });
  assert.deepEqual((await saved()).sidebar_order, expected, "mixed root order survives reload");
  console.log("ok: typed sidebar drag/drop, mixed root order, cancellation, emoji icon and label, group keyboard navigation, direct removal, ordered ungroup, and reload persistence");
}

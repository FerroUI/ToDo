import assert from "node:assert/strict";
import { dragSidebar, pressFunction, rightClick } from "./sidebar-input.mjs";

const button = label => element => element.type.endsWith("Button") && element.automationName === label;
const groupButton = id => element => element.name === `Group${id}`;
const listButton = id => element => element.name === `NavigateList(${id})`;
const menu = label => element => element.type.endsWith("MenuItem") && element.automationName === label;

export async function verifySidebar(ui) {
  const { page, saved, control, clickControl, edit, until, resize, snapshot, capture, navigate } = ui;
  await resize(1280, 1000);
  const createList = async name => {
    await clickControl(button("New list"), "New list");
    await edit("ListName", name, true);
    return until(async () => (await saved()).lists.find(list => list.name === name), `saved ${name}`);
  };
  const createGroup = async name => {
    await clickControl(button("Create a new group"), "Create group");
    await edit("GroupName", name, true);
    return until(async () => (await saved()).groups.find(group => group.name === name), `saved ${name}`);
  };
  const first = await createList("Sidebar A");
  await edit("NewTask", "Sidebar retained task", true);
  const task = await until(async () => (await saved()).tasks.find(task => task.title === "Sidebar retained task"), "a task for navigation durability");
  const second = await createList("Sidebar B");
  const group = await createGroup("Sidebar group");
  await control(element => element.name === `EmptyGroup${group.id}`, "empty-group drop guidance");
  const other = await createGroup("Sidebar other");

  await dragSidebar(ui, listButton(first.id), groupButton(group.id));
  await until(async () => (await saved()).lists.find(list => list.id === first.id).group_id === group.id, "drop list into a group");
  await control(element => listButton(first.id)(element) && element.focused, "focus follows the relocated list");
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
  await capture("sidebar-groups");

  await ui.focusControl(groupButton(group.id), "group keyboard focus");
  await page.press("ArrowLeft");
  await until(async () => (await saved()).groups.find(item => item.id === group.id).collapsed, "Left collapses a group");
  assert(!(await snapshot()).elements.some(listButton(first.id)), "collapsed children leave the visible navigation");
  await page.press("ArrowRight");
  await until(async () => !(await saved()).groups.find(item => item.id === group.id).collapsed, "Right expands a group");
  await page.press("ArrowRight");
  await control(element => listButton(second.id)(element) && element.focused, "Right enters the first group member");
  await page.press("ArrowDown");
  await control(element => listButton(first.id)(element) && element.focused, "Down follows visible sibling order");
  await page.press("ArrowLeft");
  await control(element => groupButton(group.id)(element) && element.focused, "Left returns to the parent group");
  await pressFunction(page, 2);
  await control(element => element.name === "GroupName" && element.focused, "F2 opens inline group naming");
  await edit("GroupName", "Sidebar renamed", true);

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
  console.log("ok: typed sidebar drag/drop, mixed root order, cancellation, group keyboard navigation, direct removal, ordered ungroup, and reload persistence");
}

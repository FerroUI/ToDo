import assert from "node:assert/strict";
import { button, center, named } from "./touch-input.mjs";
import { revealSidebarControls } from "./sidebar-viewport.mjs";

async function hold(ctx, match, kind) {
  await ctx.touch.start(center(await ctx.control(match, `touch ${kind} source`)));
  await ctx.until(async () => (await ctx.snapshot()).elements.some(element =>
    element.name === "TouchGestureMessage" && element.text === `Drag to move ${kind}`), `native ${kind} hold feedback`);
}

async function drop(ctx, from, kind, to, fraction = .5, cancel = false) {
  await revealSidebarControls(ctx, [from, to], `touch ${kind} source and drop target`, { touch: ctx.touch });
  await hold(ctx, from, kind);
  const target = await ctx.control(to, `touch ${kind} drop target`);
  await ctx.touch.move({ x: target.x + target.width / 2, y: target.y + target.height * fraction });
  if (cancel) await ctx.touch.cancel(); else await ctx.touch.end();
}

export async function verifyTouchSidebar(ctx) {
  const { touch, saved, snapshot, control, edit, until, resize, capture } = ctx;
  await resize(1280, 1000);
  await touch.tapControl(button("Create a new group"), "create a disposable touch group");
  await edit("GroupName", "Touch group", true);
  const group = await until(async () => (await saved()).groups.find(group => group.name === "Touch group"), "touch group is saved");
  const source = named(`NavigateList(${ctx.source.id})`);
  const destination = named(`NavigateList(${ctx.destination.id})`);
  const groupRow = named(`Group${group.id}`);
  const tasks = (await saved()).tasks;

  await drop(ctx, source, "list", groupRow);
  await until(async () => (await saved()).lists.find(list => list.id === ctx.source.id).group_id === group.id,
    "native list hold and drop joins the target group");
  const beforeCancel = await saved();
  await drop(ctx, destination, "list", groupRow, .5, true);
  assert.deepEqual(await saved(), beforeCancel, "touchCancel preserves list membership and mixed root order");
  await control(element => element.type === "TextBlock" && element.text === ctx.source.name && element.x >= 280,
    "cancelled sidebar drag cannot navigate to its source list");

  await drop(ctx, groupRow, "group", destination, .15);
  await until(async () => {
    const order = (await saved()).sidebar_order;
    return order.indexOf(group.id) + 1 === order.indexOf(ctx.destination.id);
  }, "held group drag changes the mixed root order");
  assert.equal((await saved()).groups.find(item => item.id === group.id).collapsed, false,
    "group drag release cannot also toggle the group");
  await capture("touch-sidebar-group");
  await touch.tapControl(groupRow, "ordinary group collapse tap");
  await until(async () => (await saved()).groups.find(item => item.id === group.id).collapsed, "touch tap collapses the group");
  assert(!(await snapshot()).elements.some(source), "collapsed group members leave the visible navigation");
  await touch.tapControl(groupRow, "ordinary group expansion tap");
  await control(source, "expanded child list");

  await hold(ctx, groupRow, "group");
  await touch.end();
  await touch.tapControl(element => element.type.endsWith("MenuItem") && element.automationName === "Ungroup lists",
    "stationary touch hold opens the native group menu");
  await until(async () => !(await saved()).groups.some(item => item.id === group.id), "touch menu ungroups the disposable group");
  assert.equal((await saved()).lists.find(list => list.id === ctx.source.id).group_id, null);
  assert.deepEqual((await saved()).tasks, tasks, "touch navigation operations preserve every task field");
  console.log("ok: sidebar touch hold/drop, group reorder, cancellation, normal taps, and preserving touch-menu ungroup");
}

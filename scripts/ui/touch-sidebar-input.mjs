import { center } from "./touch-input.mjs";
import { revealSidebarControls } from "./sidebar-viewport.mjs";

export async function holdSidebar(ctx, match, kind) {
  await ctx.touch.start(center(await ctx.control(match, `touch ${kind} source`)));
  await ctx.until(async () => (await ctx.snapshot()).elements.some(element =>
    element.name === "TouchGestureMessage" && element.text === `Drag to move ${kind}`), `native ${kind} hold feedback`);
}

// A native ScrollViewer can move its rows during a drag. Follow the rendered
// target and validate the final finger position against freshly read bounds.
// Reusing a pre-movement coordinate can drop beside a group instead of into it.
export async function followSidebarTarget(ctx, match, label, fraction = .5) {
  const point = target => ({ x: target.x + target.width / 2, y: target.y + target.height * fraction });
  let target = await ctx.control(match, label);
  for (let attempt = 0; attempt < 12; attempt++) {
    const destination = point(target);
    await ctx.touch.move(destination);
    target = await ctx.control(match, label);
    const current = point(target);
    if (Math.abs(current.x - destination.x) <= .5 && Math.abs(current.y - destination.y) <= .5) return;
  }
  throw new Error(`The native sidebar drop target did not settle beneath the finger: ${label}`);
}

export async function dropSidebar(ctx, from, kind, to, fraction = .5, cancel = false) {
  // TouchDrag scrolls within 32px of an edge. Prepare both contact points clear
  // of that band before Holding, so the destination remains under the finger.
  await revealSidebarControls(ctx, [from, to], `touch ${kind} source and drop target`,
    { touch: ctx.touch, edgeMargin: 36, fractions: [.5, fraction] });
  await holdSidebar(ctx, from, kind);
  await followSidebarTarget(ctx, to, `touch ${kind} drop target`, fraction);
  if (cancel) await ctx.touch.cancel(); else await ctx.touch.end();
}

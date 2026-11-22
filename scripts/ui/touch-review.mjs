import assert from "node:assert/strict";
import { button, named } from "./touch-input.mjs";
import { runReviewGroups } from "./review-groups.mjs";
import { verifyTouchActions } from "./touch-actions.mjs";
import { verifyTouchDrag } from "./touch-drag.mjs";
import { verifyCompactNavigation, verifyScrollAndRefresh } from "./touch-navigation.mjs";
import { verifyTouchEdgeCases } from "./touch-edge-cases.mjs";
import { verifyTouchSidebar } from "./touch-sidebar.mjs";

export async function reviewTouch(ctx, output) {
  const recover = async () => {
    // Preserve the failing gesture's evidence first. Then release its actual contact
    // and use ordinary navigation to prepare the next independently asserted gesture.
    await ctx.touch.cancel();
    await ctx.navigate("Page.reload", { ignoreCache: true });
    await ctx.resize(1280, 800);
    const document = await ctx.saved();
    for (const list of [ctx.source, ctx.destination]) {
      assert(document.lists.some(item => item.id === list.id), `the saved ${list.name} fixture survives recovery`);
    }
    await ctx.clickControl(named(`NavigateList(${ctx.source.id})`), "return to the saved touch fixture");
    if ((await ctx.snapshot()).elements.some(element => element.hit && button("Remove sort")(element))) {
      await ctx.clickControl(button("Remove sort"), "restore the fixture's manual order");
    }
    await ctx.until(async () => (await ctx.saved()).lists.find(list => list.id === ctx.source.id).sort === "Manual",
      "the recovered fixture's manual order is saved");
    await ctx.resize(390, 844);
    await ctx.dismissStatus();
  };
  const failures = await runReviewGroups(ctx, output, [
    { name: "touch-navigation", run: () => verifyCompactNavigation(ctx) },
    { name: "touch-actions", run: () => verifyTouchActions(ctx) },
    { name: "touch-drag", run: () => verifyTouchDrag(ctx) },
    { name: "touch-refresh", run: () => verifyScrollAndRefresh(ctx) },
    { name: "touch-cancel-recycle", run: () => verifyTouchEdgeCases(ctx) },
    { name: "touch-sidebar", run: () => verifyTouchSidebar(ctx) },
  ], { recover, reportName: "touch-reviews.json" });
  if (failures.length) {
    throw new AggregateError(failures.map(failure => failure.error),
      `Touch reviews failed: ${failures.map(failure => `${failure.name} (${failure.phase})`).join(", ")}`);
  }
}

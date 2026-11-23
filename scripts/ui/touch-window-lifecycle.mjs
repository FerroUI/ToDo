import assert from "node:assert/strict";
import { endSelection, heldDrag, noTransient, recoverTap, topTasks } from "./touch-combination-helpers.mjs";
import { prepareTabInterruption } from "./tab-interruption.mjs";
import { runPointerStage } from "./input-diagnostics.mjs";

export async function verifyWindowInterruption(ctx) {
  const { touch, saved, capture } = ctx;
  const tabs = await prepareTabInterruption(ctx);
  let failure;
  try {
    await runPointerStage(ctx, "touch tab interruption", async () => {
      for (const releaseWhileHidden of [false, true]) {
        const tasks = await topTasks(ctx), baseline = await saved();
        const drop = await heldDrag(ctx, tasks[1], tasks[0]);
        console.log(`lifecycle: deactivate with ${releaseWhileHidden ? "hidden" : "delayed"} release`);
        await tabs.deactivate();
        // Chromium may defer an input acknowledgement while its target is hidden.
        // Issue the real End now, observe rejection immediately, and use the separate
        // browser channel to reactivate before requiring its successful acknowledgement.
        const release = releaseWhileHidden ? touch.end({ settle: false }).then(
          () => ({ error: null }), error => ({ error })) : null;
        console.log("lifecycle: reactivate original target");
        await tabs.restore();
        if (release) {
          const result = await release;
          if (result.error) throw result.error;
          console.log("lifecycle: hidden release acknowledged");
        }
        await noTransient(ctx, "tab deactivation cancels native gesture feedback");
        if (!releaseWhileHidden) {
          await touch.move({ x: drop.x + 2, y: drop.y + 2 });
          await touch.end();
        }
        assert.deepEqual(await saved(), baseline, "tab interruption and delayed release cannot commit a drop");
        if (releaseWhileHidden) await capture("touch-window-return");
        // The very first fresh touch must work, including when the hidden release
        // did not reach the native pointer registry and left an old ID quarantined.
        await endSelection(ctx);
        await recoverTap(ctx, tasks[0]);
        assert.deepEqual(await saved(), baseline, "fresh input after tab return preserves the document");
        console.log("lifecycle: first fresh contact and ordinary tap recovered");
      }
    });
  } catch (error) {
    failure = error;
  } finally {
    try { await tabs.close(); } catch (error) { failure ??= error; }
  }
  if (failure) throw failure;
  console.log("ok: actual tab deactivation, delayed/hidden release, inert old contact and first-fresh-touch recovery");
}

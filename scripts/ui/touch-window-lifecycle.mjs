import assert from "node:assert/strict";
import { endSelection, heldDrag, noTransient, recoverTap, topTasks } from "./touch-combination-helpers.mjs";

export async function verifyWindowInterruption(ctx) {
  const { page, touch, saved, until, capture } = ctx;
  const original = await page.send("Target.getTargetInfo");
  assert(!original.error, original.error?.message);
  const other = await page.browserSend("Target.createTarget", { url: "about:blank", background: true });
  assert(!other.error, other.error?.message);
  const activate = async targetId => {
    const result = await page.browserSend("Target.activateTarget", { targetId });
    assert(!result.error, result.error?.message);
  };
  let failure;
  try {
    for (const releaseWhileHidden of [false, true]) {
      const tasks = await topTasks(ctx), baseline = await saved();
      const drop = await heldDrag(ctx, tasks[1], tasks[0]);
      console.log(`lifecycle: deactivate with ${releaseWhileHidden ? "hidden" : "delayed"} release`);
      await activate(other.targetId);
      await until(() => page.evaluate("document.hidden || !document.hasFocus()"), "the browser actually deactivates the app tab");
      // Chromium may defer an input acknowledgement while its target is hidden.
      // Issue the real End now, observe rejection immediately, and use the separate
      // browser channel to reactivate before requiring its successful acknowledgement.
      const release = releaseWhileHidden ? touch.end({ settle: false }).then(
        () => ({ error: null }), error => ({ error })) : null;
      console.log("lifecycle: reactivate original target");
      await activate(original.targetInfo.targetId);
      if (release) {
        const result = await release;
        if (result.error) throw result.error;
        console.log("lifecycle: hidden release acknowledged");
      }
      await until(() => page.evaluate("!document.hidden && document.hasFocus()"), "the original browser tab is active again");
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
  } catch (error) {
    failure = error;
  } finally {
    try { await activate(original.targetInfo.targetId); } catch (error) { failure ??= error; }
    try {
      const result = await page.browserSend("Target.closeTarget", { targetId: other.targetId });
      assert(!result.error, result.error?.message);
    } catch (error) { failure ??= error; }
  }
  if (failure) throw failure;
  console.log("ok: actual tab deactivation, delayed/hidden release, inert old contact and first-fresh-touch recovery");
}

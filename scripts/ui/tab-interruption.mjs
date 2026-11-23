import assert from "node:assert/strict";

// Clipboard reviews may emulate a focused document. That CDP setting survives
// reloads and suppresses real tab visibility/focus transitions, so clear it
// before starting the contact whose interruption this review will exercise.
export async function prepareTabInterruption({ page, until }) {
  const command = async (method, parameters, browser = false) => {
    const result = await (browser ? page.browserSend(method, parameters) : page.send(method, parameters));
    assert(!result.error, result.error?.message);
    return result;
  };
  await command("Emulation.setFocusEmulationEnabled", { enabled: false });
  const original = (await command("Target.getTargetInfo")).targetInfo.targetId;
  const activate = targetId => command("Target.activateTarget", { targetId }, true);
  const restore = async (label = "the original browser tab is active again") => {
    await activate(original);
    await until(() => page.evaluate("!document.hidden && document.hasFocus()"), label);
  };
  await restore("the real app tab is active before the contact begins");
  const other = (await command("Target.createTarget", { url: "about:blank", background: true }, true)).targetId;
  assert(other && other !== original, "the interruption uses a distinct browser tab");
  return {
    async deactivate(label = "the browser actually deactivates the app tab") {
      await activate(other);
      await until(() => page.evaluate("document.hidden || !document.hasFocus()"), label);
    },
    restore,
    async close() {
      let failure;
      try { await restore(); } catch (error) { failure = error; }
      try { await command("Target.closeTarget", { targetId: other }, true); }
      catch (error) { failure ??= error; }
      if (failure) throw failure;
    },
  };
}

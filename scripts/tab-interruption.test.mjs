import assert from "node:assert/strict";
import test from "node:test";
import { prepareTabInterruption } from "./ui/tab-interruption.mjs";

function harness({ ignoresActivation = false, refusesReset = false } = {}) {
  let emulated = true, active = "app";
  const calls = [], observations = [];
  return {
    calls, observations,
    page: {
      async send(method, parameters) {
        calls.push({ method, parameters });
        if (method === "Emulation.setFocusEmulationEnabled") {
          if (refusesReset) return { error: { message: "Focus emulation reset rejected" } };
          emulated = parameters.enabled;
          return {};
        }
        assert.equal(method, "Target.getTargetInfo");
        return { targetInfo: { targetId: "app" } };
      },
      async browserSend(method, parameters) {
        calls.push({ method, parameters });
        if (method === "Target.createTarget") {
          assert.deepEqual(parameters, { url: "about:blank", background: true });
          assert.equal(active, "app", "preparing the other tab must not interrupt the app early");
          return { targetId: "other" };
        }
        if (method === "Target.activateTarget") {
          if (!ignoresActivation) active = parameters.targetId;
          return {};
        }
        assert.equal(method, "Target.closeTarget");
        assert.equal(parameters.targetId, "other");
        assert.equal(active, "app", "restore the app before closing the disposable tab");
        return { success: true };
      },
      async evaluate(expression) {
        observations.push({ expression, emulated, active });
        if (expression === "!document.hidden && document.hasFocus()") return emulated || active === "app";
        assert.equal(expression, "document.hidden || !document.hasFocus()", "lifecycle reads browser state without synthesizing a DOM event");
        return !emulated && active !== "app";
      },
    },
    async until(check, label) {
      assert(await check(), label);
    },
  };
}

test("tab interruption clears persistent focus emulation before input and observes actual departure and return", async () => {
  const ui = harness(), tabs = await prepareTabInterruption(ui);
  assert.deepEqual(ui.calls[0], { method: "Emulation.setFocusEmulationEnabled", parameters: { enabled: false } });
  assert(ui.observations.every(value => !value.emulated));
  await tabs.deactivate();
  assert.equal(ui.observations.at(-1).active, "other");
  await tabs.restore();
  assert.equal(ui.observations.at(-1).active, "app");
  await tabs.close();
  assert.equal(ui.calls.at(-1).method, "Target.closeTarget");
});

test("an acknowledged activation without actual focus loss still fails the lifecycle assertion", async () => {
  const ui = harness({ ignoresActivation: true }), tabs = await prepareTabInterruption(ui);
  await assert.rejects(tabs.deactivate("native departure must occur"), /native departure must occur/);
  await tabs.close();
});

test("a rejected focus reset stops before creating a tab or starting the interaction", async () => {
  const ui = harness({ refusesReset: true });
  await assert.rejects(prepareTabInterruption(ui), /Focus emulation reset rejected/);
  assert.equal(ui.calls.length, 1);
  assert.deepEqual(ui.observations, []);
});

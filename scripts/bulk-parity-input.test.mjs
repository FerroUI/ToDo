import assert from "node:assert/strict";
import test from "node:test";
import { verifyBulkParity } from "./ui/bulk-parity.mjs";

for (const [phase, cleanupFailure] of [["clipboard permission", false], ["fixture creation", false], ["fixture creation", true]]) {
  test(`bulk review releases emulated focus after ${phase} failure${cleanupFailure ? " and preserves a cleanup error" : ""}`, async () => {
    const failure = new Error(`original ${phase} failure`);
    const focus = [], permissions = [];
    const ui = {
      resize: async () => {},
      saved: async () => ({ settings: {} }),
      clickControl: async () => { throw failure; },
      page: {
        send: async (method, parameters) => {
          assert.equal(method, "Emulation.setFocusEmulationEnabled");
          focus.push(parameters.enabled);
          if (!parameters.enabled && cleanupFailure) throw new Error("focus cleanup unavailable");
        },
        setPermission: async permission => {
          permissions.push(permission);
          if (phase === "clipboard permission") throw failure;
        },
      },
    };
    await assert.rejects(verifyBulkParity(ui), error => error === failure,
      "cleanup preserves the original review error for diagnostics");
    assert.deepEqual(focus, [true, false],
      "subsequent tab/pen/touch lifecycle reviews observe genuine browser focus");
    assert.equal(permissions.length, phase === "clipboard permission" ? 1 : 2);
    if (cleanupFailure) assert.match(failure.inputDiagnostics.focusCleanupError, /focus cleanup unavailable/);
  });
}

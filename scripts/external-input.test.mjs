import assert from "node:assert/strict";
import test from "node:test";
import { dragExternalInput } from "./ui/external-input.mjs";

function fixture(t, { brokenEscape = false } = {}) {
  t.mock.method(console, "log", () => {});
  const h = { targetRetained: false, lease: false, hint: false, reads: 0, enters: 0,
    tasks: [], commands: [], events: [] };
  const over = () => { h.events.push("dragover"); h.hint = h.lease; };
  const snapshot = async () => ({ elements: h.hint ? [{ name: "ExternalTaskDropHint" }] : [] });
  h.ui = { snapshot, inputTrace: async () => ({ events: h.events.slice() }),
    until: async (check, label) => {
      if (!await check()) throw new Error(`Timed out: ${label}`);
    },
    page: {
      async send(command, args) {
        assert.equal(command, "Input.dispatchDragEvent");
        assert.equal(args.dragOperationsMask, undefined);
        assert.equal(args.data.dragOperationsMask, 1, "external task offers permit only COPY");
        h.commands.push(args);
        // Chromium keeps its DOM target after source-only dragCancel. A repeated
        // Enter on that target becomes Over; NONE -> Drop instead raises Leave.
        // This independent protocol model reproduces the failed second hover.
        if (args.type === "dragEnter") {
          if (!h.targetRetained) {
            h.events.push("dragenter"); h.enters++; h.lease = true;
            h.targetRetained = true; h.hint = true;
          } else over();
        } else if (args.type === "dragOver") {
          over();
        } else if (args.type === "drop") {
          over(); // CDP updates the accepted operation before forwarding Drop.
          if (h.lease) {
            h.events.push("drop"); h.reads++;
            h.tasks.push(args.data.items[0].data);
          } else h.events.push("dragleave");
          h.targetRetained = false; h.lease = false; h.hint = false;
        } else assert.equal(args.type, "dragCancel");
        return {};
      },
      async press(key) {
        assert.equal(key, "Escape"); h.events.push("Escape");
        if (!brokenEscape) { h.lease = false; h.hint = false; }
      },
      async evaluate() {},
    },
  };
  return h;
}

const target = { name: "TaskDropSurface", x: 312, y: 178, width: 936, height: 524 };
const plain = text => [{ mimeType: "text/plain", data: text }];

test("cancelled external input rejects its full late payload and permits a new drag on the same target", async t => {
  const h = fixture(t), canceled = plain("Canceled external task");
  await dragExternalInput(h.ui, target, canceled, { accept: false, hover: true });
  assert.deepEqual(h.tasks, []);
  assert.equal(h.reads, 0, "the canceled transfer must never materialize its payload");
  assert.equal(h.hint, false);
  assert.equal(h.targetRetained, false, "both browser source and target lifetimes finish");
  assert.equal(h.commands.at(-1).type, "drop");
  assert.equal(h.commands.at(-1).data.items, canceled, "late rejection uses the original full payload");
  await dragExternalInput(h.ui, target, plain("Fresh external task"), { hover: true });
  assert.equal(h.enters, 2, "a new operation must receive a real target Enter");
  assert.deepEqual(h.tasks, ["Fresh external task"]);
  assert.equal(h.reads, 1);
});

test("failed Escape cancellation remains the original error before any target cleanup can hide it", async t => {
  const h = fixture(t, { brokenEscape: true });
  await assert.rejects(dragExternalInput(h.ui, target, plain("Must remain pending"), { accept: false, hover: true }), error => {
    assert.match(error.message, /Escape removes external-drop feedback before a late drop/);
    assert.equal(error.inputDiagnostics.phase, "Escape cancellation");
    assert.equal(error.inputDiagnostics.trace.events.at(-1), "Escape");
    assert.deepEqual(error.inputDiagnostics.snapshot.elements, [{ name: "ExternalTaskDropHint" }]);
    return true;
  });
  assert.equal(h.commands.some(command => command.type === "drop"), false);
  assert.equal(h.targetRetained, true, "a failed assertion retains its original observed state");
  assert.deepEqual(h.tasks, []);
});

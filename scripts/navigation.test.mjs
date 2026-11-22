import test from "node:test";
import assert from "node:assert/strict";
import { createNavigation } from "../crates/todo-browser/wwwroot/navigation.js";

function setup(state = { authentication: "keep" }) {
  let pop;
  let handled = true;
  let calls = 0;
  const pushes = [];
  const history = { state, backs: 0, pushState(value, title, url) {
    this.state = value; pushes.push({ value, title, url });
  }, back() { this.backs++; } };
  const navigation = createNavigation({ history,
    addEventListener: (_, callback) => { pop = callback; },
    removeEventListener: () => {},
    goBack: () => { calls++; navigation.synchronize(false); return handled; },
  });
  return { navigation, history, pushes, pop: value => pop({ state: value }),
    calls: () => calls, result: value => { handled = value; } };
}

test("app navigation preserves the document URL and unrelated history state", () => {
  const host = setup();
  host.navigation.synchronize(true);
  host.navigation.synchronize(true);
  assert.equal(host.pushes.length, 1);
  assert.equal(host.pushes[0].value.authentication, "keep");
  assert.equal(host.pushes[0].url, undefined);
  host.navigation.synchronize(false);
  assert.equal(host.history.backs, 0, "closing a pane never causes asynchronous document navigation");
});

test("browser Back dismisses one live application surface and remains in the document", () => {
  const host = setup();
  host.navigation.synchronize(true);
  host.pop({ authentication: "keep" });
  assert.equal(host.calls(), 1);
  assert.equal(host.history.backs, 0);
  assert.equal(host.pushes.length, 1, "closing the last surface does not grow history");
});

test("a guard restored by reload never navigates or mutates the app during startup", () => {
  const host = setup({ ferrouiTodoNavigation: true });
  host.navigation.synchronize(false);
  assert.equal(host.history.backs, 0);
  assert.equal(host.calls(), 0);
  host.pop({ ferrouiTodoNavigation: true });
  assert.equal(host.calls(), 0, "returning from an external document restores the app unchanged");
});

test("Back passes through a stale guard after a pane was closed through its own button", () => {
  const host = setup();
  host.navigation.synchronize(true);
  host.navigation.synchronize(false);
  host.result(false);
  host.pop({ authentication: "keep" });
  assert.equal(host.calls(), 1);
  assert.equal(host.history.backs, 1);
});

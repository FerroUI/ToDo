const KEY = "ferrouiTodoNavigation";
let current;
let desired = false;

// One same-document entry lets browser/OS Back dismiss an app surface first.
// Keep the original URL and unrelated history state, including authentication queries.
export function createNavigation({ history, addEventListener, removeEventListener, goBack }) {
  let armed = history.state?.[KEY] === true;
  let available = false;
  let popping = false;
  const synchronize = value => {
    available = !!value;
    if (popping) return;
    if (available && !armed) {
      history.pushState({ ...(history.state && typeof history.state === "object" ? history.state : {}), [KEY]: true }, "");
      armed = true;
    }
  };
  const pop = event => {
    // Returning from another document must preserve this application's current
    // surface. Never navigate automatically during initial load or reload.
    if (event.state?.[KEY] === true) { armed = true; return; }
    if (!armed) return;
    armed = false;
    popping = true;
    let handled = false;
    try { handled = goBack(); }
    finally { popping = false; }
    if (handled) synchronize(available);
    else history.back(); // The surface was already dismissed through its own UI.
  };
  addEventListener("popstate", pop);
  return { synchronize, dispose: () => removeEventListener("popstate", pop) };
}

export function attachNavigation(runtime, host = window) {
  current?.dispose();
  current = createNavigation({
    history: host.history,
    addEventListener: host.addEventListener.bind(host),
    removeEventListener: host.removeEventListener.bind(host),
    goBack: () => runtime.navigateBack(),
  });
  current.synchronize(desired);
}

export function synchronizeNavigation(available) {
  desired = !!available;
  current?.synchronize(desired);
}

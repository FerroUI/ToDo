import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";
import { attachRuntime, today, width, reportError } from "./host.js";
import { prepareMicrosoft, attachMicrosoft } from "./graph.js";

try {
  await prepareMicrosoft();
  const loaded = await loadModule({ script: "./todo-browser.js", element: document.getElementById("out") });
  if (loaded) {
    const runtime = await loaded.createRuntime();
    FerroExports.attach(runtime);
    attachRuntime(runtime);
    globalThis.todoApp = runtime;
    runtime.runMain(globalThis.location.search);
    void attachMicrosoft(runtime);
    new ResizeObserver(() => runtime.resizeWindow(width())).observe(document.getElementById("out"));
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", event => runtime.updateTheme(event.matches));
    const tick = () => {
      const now = new Date();
      runtime.tick(today(), now.getHours() * 60 + now.getMinutes());
    };
    const flush = () => {
      try { runtime.flushEdits(); }
      catch (error) { reportError(error); }
    };
    tick();
    setInterval(tick, 30_000);
    document.addEventListener("visibilitychange", () => { if (document.hidden) flush(); else tick(); });
    window.addEventListener("focus", tick);
    window.addEventListener("pagehide", flush);
    window.addEventListener("beforeunload", flush);
    if ("serviceWorker" in navigator && !new URLSearchParams(location.search).has("nooffline")) {
      navigator.serviceWorker.register("./app-sw.js").catch(error => {
        // Local storage still works when the browser disallows offline app installation.
        console.info("Offline installation is unavailable:", error.message);
      });
    }
  }
} catch (error) {
  document.getElementById("loading-status").textContent = `To Do could not start. ${error.message || error}`;
  document.querySelector(".loading-line")?.remove();
  console.error(error);
  reportError(error);
}

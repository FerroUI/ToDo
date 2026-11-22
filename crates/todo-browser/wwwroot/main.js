import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";
import { attachRuntime, today, width, reportError } from "./host.js";
import { prepareMicrosoft, attachMicrosoft } from "./graph.js";
import { registerOffline } from "./offline.js";
import { loadEmojiFont } from "./fonts.js";

try {
  await prepareMicrosoft();
  const [loaded, emojiFont] = await Promise.all([
    loadModule({ script: new URL("./todo-browser.js", import.meta.url).href, element: document.getElementById("out") }),
    loadEmojiFont(),
  ]);
  if (loaded) {
    const runtime = await loaded.createRuntime();
    FerroExports.attach(runtime);
    attachRuntime(runtime);
    globalThis.todoApp = runtime;
    runtime.runMain(globalThis.location.search, emojiFont);
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
    registerOffline(navigator.serviceWorker, location.href).catch(error => {
      // Local storage still works when the browser disallows offline app installation.
      console.info("Offline installation is unavailable:", error.message);
    });
  }
} catch (error) {
  document.getElementById("loading-status").textContent = `To Do could not start. ${error.message || error}`;
  document.querySelector(".loading-line")?.remove();
  console.error(error);
  reportError(error);
}

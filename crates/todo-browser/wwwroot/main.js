import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";
import { attachRuntime, today, width, reportError, acceptWorkspaceStorage } from "./host.js";
import { prepareMicrosoft, attachMicrosoft } from "./graph.js";
import { registerOffline } from "./offline.js";
import { loadCjkFont, loadEmojiFont } from "./fonts.js";
import { attachNavigation } from "./navigation.js";
import { attachLifecycle } from "./lifecycle.js";
import { attachPush } from "./push.js";

try {
  await prepareMicrosoft();
  const [loaded, emojiFont, cjkFont] = await Promise.all([
    loadModule({ script: new URL("./todo-browser.js", import.meta.url).href, element: document.getElementById("out") }),
    loadEmojiFont(),
    loadCjkFont(),
  ]);
  if (loaded) {
    const runtime = await loaded.createRuntime();
    FerroExports.attach(runtime);
    attachRuntime(runtime);
    globalThis.todoApp = runtime;
    runtime.runMain(globalThis.location.search, emojiFont, cjkFont);
    attachPush(runtime);
    attachNavigation(runtime);
    void attachMicrosoft(runtime, { workspaceChanged: acceptWorkspaceStorage });
    new ResizeObserver(() => runtime.resizeWindow(width())).observe(document.getElementById("out"));
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", event => runtime.updateTheme(event.matches));
    const tick = () => {
      const now = new Date();
      runtime.tick(today(), now.getHours() * 60 + now.getMinutes());
    };
    attachLifecycle(runtime, tick, reportError);
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

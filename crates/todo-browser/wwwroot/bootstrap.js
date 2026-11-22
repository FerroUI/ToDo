import { prepareOfflinePreview } from "./offline.js";

try {
  // An older installed worker may ignore nooffline. Finish its bounded, scoped
  // handoff before loading any runtime modules or creating an editable document.
  const start = await prepareOfflinePreview({
    href: location.href,
    serviceWorker: navigator.serviceWorker,
    storage: () => sessionStorage,
    reload: () => location.replace(location.href),
  });
  if (start) {
    for (const name of ["main.js", "todo-browser.js", "ferroui.js", "ferroui-loader.js", "todo_browser.wasm"]) {
      const link = document.createElement("link");
      link.href = new URL(name, import.meta.url).href;
      link.rel = name.endsWith(".wasm") ? "preload" : "modulepreload";
      if (name.endsWith(".wasm")) { link.as = "fetch"; link.type = "application/wasm"; }
      link.crossOrigin = "anonymous";
      document.head.append(link);
    }
    await import("./main.js");
  }
} catch (error) {
  document.getElementById("loading-status").textContent = `To Do could not start. ${error.message || error}`;
  document.querySelector(".loading-line")?.remove();
  console.error(error);
}

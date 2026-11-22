// Node timers remain live when Chromium suspends a hidden tab's animation frames.
export async function withDeadline(operation, label, timeout = 15_000) {
  let timer;
  try {
    return await Promise.race([
      Promise.resolve().then(operation),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`Browser operation timed out after ${timeout}ms: ${label}`)), timeout);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

// The upstream driver closes over its raw CDP transport. Bound every public async
// method as well as send/evaluate so composed input and diagnostic calls cannot hang.
export function boundPage(page, timeout = 15_000) {
  for (const [name, operation] of Object.entries(page)) {
    if (typeof operation !== "function" || name === "on") continue;
    page[name] = (...args) => {
      const detail = name === "send" || name === "browserSend" ? ` ${args[0]}${args[1]?.type ? ` (${args[1].type})` : ""}`
        : name === "evaluate" ? ` ${String(args[0]).replace(/\s+/g, " ").slice(0, 100)}` : "";
      return withDeadline(() => operation.apply(page, args), `${name}${detail}`, timeout);
    };
  }
  return page;
}

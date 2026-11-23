import { readConfig } from "./config.mjs";
import { DurableStore } from "./store.mjs";
import { DeliveryQueue } from "./queue.mjs";
import { createReminderServer } from "./server.mjs";

if (process.platform !== "linux") throw new Error("The durable reminder service requires Linux; use the supplied container on other hosts.");
const config = readConfig(), store = await DurableStore.open(config.path);
const queue = new DeliveryQueue(store, config);
// Deliberately log neither request URLs, authorization nor task content.
const server = createReminderServer(store, config, { onError: () => console.error("Reminder request failed; check storage availability.") });
server.listen(config.port, config.host, () => console.info(`Reminder service listening on ${config.host}:${server.address().port}.`));
const timer = setInterval(() => queue.tick().catch(() => console.error("Reminder delivery failed; check storage availability.")), 1000);
let stopping = false;
async function stop() {
  if (stopping) return; stopping = true; clearInterval(timer);
  await new Promise(resolve => server.close(resolve));
  await queue.close(); await store.close();
}
process.on("SIGTERM", () => void stop()); process.on("SIGINT", () => void stop());
server.on("error", error => { console.error(`Cannot listen: ${error.code || "unknown error"}.`); void stop().finally(() => { process.exitCode = 1; }); });

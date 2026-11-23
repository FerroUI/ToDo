import { randomBytes, createECDH } from "node:crypto";
import { generateVAPIDKeys } from "web-push";
import { readConfig } from "../src/config.mjs";
import { MemoryStore } from "../src/store.mjs";
import { Schedules } from "../src/schedules.mjs";

export const origin = "https://tasks.example.test";
export const random = (length = 32) => randomBytes(length).toString("base64url");
export function config(overrides = {}) {
  const vapid = generateVAPIDKeys();
  return { ...readConfig({ TODO_PUSH_ORIGINS: origin, TODO_PUSH_VAPID_PUBLIC_KEY: vapid.publicKey,
    TODO_PUSH_VAPID_PRIVATE_KEY: vapid.privateKey, TODO_PUSH_VAPID_SUBJECT: "mailto:admin@example.test",
    TODO_PUSH_ENROLLMENT_KEY: random() }), ...overrides };
}
export function subscription() {
  const key = createECDH("prime256v1"); key.generateKeys();
  return { endpoint: `https://fcm.googleapis.com/fcm/send/${random()}`, keys: { p256dh: key.getPublicKey().toString("base64url"), auth: random(16) } };
}
export async function fixture(overrides = {}) {
  let clock = Date.UTC(2026, 10, 23, 12);
  const settings = config(overrides), store = new MemoryStore(), schedules = new Schedules(store, settings, () => clock);
  const enrollment = { id: random(16), capability: random(), scope: `${origin}/ToDo/`, subscription: subscription() };
  await schedules.enroll(enrollment, origin);
  const workspace = random();
  const job = (changes = {}) => ({ id: random(), at: clock, expiresAt: clock + settings.expiryMs, title: "Task reminder", ...changes });
  return { config: settings, store, schedules, enrollment, workspace, job,
    now: () => clock, advance: value => { clock += value; },
    update: (jobs, revision = 0, extra = {}) => schedules.update(enrollment.id, enrollment.capability, origin, `"${revision}"`, { workspace, jobs, ...extra }),
  };
}

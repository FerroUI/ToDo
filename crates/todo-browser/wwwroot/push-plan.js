const TOKEN = /^[A-Za-z0-9_-]{43}$/;
export const randomToken = (length = 32) => base64url(crypto.getRandomValues(new Uint8Array(length)));
export function base64url(value) {
  return btoa(String.fromCharCode(...new Uint8Array(value))).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}
export async function digest(value) { return base64url(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value))); }
export const workspaceToken = (seed, account) => digest(JSON.stringify(["workspace-v1", seed, account]));

export function reminderInstant(date, minutes) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date || "");
  if (!match || !Number.isInteger(minutes) || minutes < 0 || minutes >= 1440) return null;
  const [year, month, day] = match.slice(1).map(Number);
  if (year < 1 || year > 9999) return null;
  const target = new Date(0); target.setFullYear(year, month - 1, day); target.setHours(Math.floor(minutes / 60), minutes % 60, 0, 0);
  const sameDay = value => value.getFullYear() === year && value.getMonth() === month - 1 && value.getDate() === day;
  if (!sameDay(target)) return null; // Invalid dates and entirely skipped civil days.
  // For a spring-forward gap, native civil-time ticks become due at the first
  // clock minute after the gap, rather than N minutes into the next hour.
  if (target.getHours() * 60 + target.getMinutes() > minutes) {
    for (let count = 0; count < 1440; count++) {
      const previous = new Date(target.getTime() - 60_000);
      if (!sameDay(previous) || previous.getHours() * 60 + previous.getMinutes() < minutes) break;
      target.setTime(previous.getTime());
    }
  }
  return Number.isSafeInteger(target.getTime()) ? target.getTime() : null;
}

export async function reminderIdentity(workspace, taskId, date, minutes) {
  const at = reminderInstant(date, minutes);
  if (!TOKEN.test(workspace) || at === null || !["number", "string"].includes(typeof taskId)) return null;
  const occurrence = await digest(JSON.stringify(["occurrence-v1", workspace, String(taskId), date, minutes]));
  return { occurrence, id: await digest(JSON.stringify(["delivery-v1", occurrence, at])), at, taskId: String(taskId), date, minutes };
}

export async function buildPlan(document, workspace, config, now = Date.now()) {
  if (!document || !Array.isArray(document.tasks)) throw new Error("The active task document is unavailable.");
  const candidates = []; let omitted = 0;
  for (const task of document.tasks) {
    if (task.completed || !task.reminder || task.reminder.notified) continue;
    const date = task.reminder.date, minutes = task.reminder.minutes_after_midnight;
    const at = reminderInstant(date, minutes);
    if (at === null || at > now + config.horizonMs || at + config.expiryMs <= now) { omitted++; continue; }
    candidates.push({ task, at });
  }
  candidates.sort((left, right) => left.at - right.at || String(left.task.id).localeCompare(String(right.task.id)));
  omitted += Math.max(0, candidates.length - config.maxJobs);
  const jobs = await Promise.all(candidates.slice(0, config.maxJobs).map(async ({ task }) => {
    const identity = await reminderIdentity(workspace, task.id, task.reminder.date, task.reminder.minutes_after_midnight);
    if (!identity) throw new Error("The task reminder identity is invalid.");
    const title = String(task.title || "Task reminder").replace(/[\u0000-\u001f\u007f]/g, " ").slice(0, 200).trim() || "Task reminder";
    return { ...identity, title, expiresAt: identity.at + config.expiryMs };
  }));
  return { plan: Object.fromEntries(jobs.map(job => [job.id, job])), omitted };
}

export function wireJobs(plan) {
  return Object.values(plan).map(({ id, at, expiresAt, title }) => ({ id, at, expiresAt, title }));
}

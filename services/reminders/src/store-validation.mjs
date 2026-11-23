import { token } from "./security.mjs";

const integer = value => Number.isSafeInteger(value) && value >= 0;
const object = value => value && typeof value === "object" && !Array.isArray(value);
function validUrl(value, origin) {
  try {
    const url = new URL(value);
    return ["https:", "http:"].includes(url.protocol) && !url.username && !url.password && !url.search && !url.hash
      && (origin ? url.origin === origin && url.pathname.endsWith("/") : url.origin === value);
  } catch { return false; }
}
function validSubscription(value) {
  if (value === null) return true;
  try {
    const url = new URL(value.endpoint);
    return object(value) && url.protocol === "https:" && !url.username && !url.password && !url.port && !url.hash
      && typeof value.endpoint === "string" && value.endpoint.length <= 4096 && object(value.keys)
      && token(value.keys.p256dh, 87) && Buffer.from(value.keys.p256dh, "base64url")[0] === 4 && token(value.keys.auth, 22)
      && (value.expirationTime == null || integer(value.expirationTime));
  } catch { return false; }
}
function validJob(id, job) {
  return token(id) && object(job) && id === job.id && token(job.workspace) && integer(job.at) && integer(job.expiresAt)
    && job.expiresAt > job.at && typeof job.title === "string" && job.title.trim() && job.title.length <= 200
    && !/[\u0000-\u001f\u007f]/.test(job.title) && integer(job.attempts) && job.attempts <= 100 && integer(job.nextAt)
    && (job.claim === null || (object(job.claim) && token(job.claim.id, 22) && integer(job.claim.until)))
    && (job.deliveredAt == null || integer(job.deliveredAt)) && (job.failedAt == null || integer(job.failedAt));
}
export function validateStore(state) {
  if (state?.version !== 1 || !object(state.installations) || Object.keys(state.installations).length > 10000
    || Object.entries(state.installations).some(([id, item]) => !token(id, 22) || !object(item) || !token(item.authHash)
      || !integer(item.revision) || !integer(item.expiresAt) || !integer(item.createdAt) || !integer(item.updatedAt)
      || !validUrl(item.origin) || !validUrl(item.scope, item.origin) || !validSubscription(item.subscription)
      || (item.workspace != null && !token(item.workspace)) || !object(item.jobs) || Object.keys(item.jobs).length > 10000
      || Object.entries(item.jobs).some(([key, job]) => !validJob(key, job) || job.workspace !== item.workspace))) {
    throw new Error("Invalid reminder store; restore a valid private backup before starting.");
  }
}

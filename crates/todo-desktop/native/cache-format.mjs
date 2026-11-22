import { createCipheriv, createDecipheriv, createHash, randomBytes } from "node:crypto";
import { realpathSync } from "node:fs";

export const MAX_CACHE_BYTES = 4 * 1024 * 1024;
const MAGIC = Buffer.from("TODOAUTH1");
export const CACHE_HEADER_BYTES = MAGIC.length + 12 + 16;
const ACCOUNT_FIELDS = ["homeAccountId", "tenantId", "environment", "localAccountId"];

export function accountIdentity(account) {
  const identity = {};
  for (const field of ACCOUNT_FIELDS) {
    const value = account?.[field];
    if (typeof value === "string" && value && value.length <= 1024 && !/[\s\x00-\x1f]/.test(value)) identity[field] = value;
  }
  if (!identity.homeAccountId) throw new Error("Microsoft did not provide a usable account identity.");
  return identity;
}

export function selectedAccount(accounts, identity) {
  const expected = accountIdentity(identity);
  const matches = accounts.filter(account => Object.entries(expected).every(([field, value]) => account[field] === value));
  // A home ID can have more than one tenant profile. Never infer a selection
  // from the first cached account, including after an MSAL version upgrade.
  return matches.length === 1 ? matches[0] : null;
}

export function cacheScope(directory, configuration) {
  const context = JSON.stringify({ version: 1, profile: realpathSync(directory), clientId: configuration.clientId.toLowerCase(),
    tenant: configuration.tenant.toLowerCase(), cloud: configuration.cloud });
  return { id: createHash("sha256").update(context).digest("hex"), context };
}

function associatedData(scope, generation) {
  if (!/^[a-f0-9]{64}$/.test(scope.id) || !/^[a-f0-9]{32}$/.test(generation)) throw new Error("Saved sign-in has an invalid identity.");
  return Buffer.from(JSON.stringify([scope.context, generation]));
}

export function encryptCache(scope, generation, account, cache, key) {
  if (!Buffer.isBuffer(key) || key.length !== 32 || typeof cache !== "string") throw new Error("Secure sign-in storage is unavailable.");
  const plain = Buffer.from(JSON.stringify({ version: 1, account: accountIdentity(account), cache }));
  if (plain.length > MAX_CACHE_BYTES) throw new Error("The saved Microsoft sign-in exceeds its size limit.");
  try {
    const nonce = randomBytes(12), cipher = createCipheriv("aes-256-gcm", key, nonce);
    cipher.setAAD(associatedData(scope, generation));
    const encrypted = Buffer.concat([cipher.update(plain), cipher.final()]);
    return Buffer.concat([MAGIC, nonce, cipher.getAuthTag(), encrypted]);
  } finally { plain.fill(0); }
}

export function decryptCache(scope, generation, bytes, key) {
  if (!Buffer.isBuffer(key) || key.length !== 32 || bytes.length <= CACHE_HEADER_BYTES || bytes.length > MAX_CACHE_BYTES + CACHE_HEADER_BYTES
    || !bytes.subarray(0, MAGIC.length).equals(MAGIC)) throw new Error("Saved sign-in could not be authenticated.");
  let plain;
  try {
    const decipher = createDecipheriv("aes-256-gcm", key, bytes.subarray(MAGIC.length, MAGIC.length + 12));
    decipher.setAAD(associatedData(scope, generation));
    decipher.setAuthTag(bytes.subarray(MAGIC.length + 12, CACHE_HEADER_BYTES));
    plain = Buffer.concat([decipher.update(bytes.subarray(CACHE_HEADER_BYTES)), decipher.final()]);
    const payload = JSON.parse(plain.toString("utf8"));
    if (payload.version !== 1 || typeof payload.cache !== "string") throw new Error("Invalid saved sign-in.");
    return { account: accountIdentity(payload.account), cache: payload.cache };
  } catch { throw new Error("Saved sign-in could not be authenticated."); }
  finally { plain?.fill(0); }
}

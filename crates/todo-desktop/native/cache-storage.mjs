import { closeSync, fsyncSync, openSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { atomicWrite, privateDirectory, readBounded } from "./storage.mjs";
import { CACHE_HEADER_BYTES, MAX_CACHE_BYTES } from "./cache-format.mjs";

function syncDirectory(directory) {
  if (process.platform === "win32") return; // Node does not expose a Windows directory flush handle.
  const descriptor = openSync(directory, "r");
  try { fsyncSync(descriptor); }
  catch (error) { if (!["EINVAL", "ENOTSUP", "EOPNOTSUPP"].includes(error.code)) throw error; }
  finally { closeSync(descriptor); }
}

function durableWrite(path, bytes) {
  atomicWrite(path, bytes);
  syncDirectory(dirname(path));
}

export function cacheFiles(directory, scope) {
  const folder = privateDirectory(join(directory, "secure-signin"));
  syncDirectory(directory);
  const marker = join(folder, `${scope.id}.json`), payload = join(folder, `${scope.id}.bin`);
  return {
    readMarker() {
      let value;
      try { value = JSON.parse(readBounded(marker, 512)); }
      catch (error) { if (error.code === "ENOENT") return { enabled: false }; throw error; }
      if (value.version !== 1 || typeof value.enabled !== "boolean"
        || (value.enabled && !/^[a-f0-9]{32}$/.test(value.generation))) throw new Error("Saved sign-in selection is invalid.");
      return value;
    },
    writeMarker(value) { durableWrite(marker, JSON.stringify({ version: 1, ...value })); },
    readPayload() { return readBounded(payload, MAX_CACHE_BYTES + CACHE_HEADER_BYTES); },
    writePayload(bytes) { durableWrite(payload, bytes); },
    deletePayload() {
      try { unlinkSync(payload); syncDirectory(folder); }
      catch (error) { if (error.code !== "ENOENT") throw error; }
    }
  };
}

import { spawn } from "node:child_process";

let executable, disabled = false;
export function configureCredentials(path, demo = false) {
  executable = typeof path === "string" && path ? path : undefined;
  disabled = demo;
}

export async function credential(operation, scope, value) {
  if (!executable || disabled) throw new Error("The operating system credential store is unavailable.");
  const request = Buffer.from(JSON.stringify({ operation, scope, ...(value === undefined ? {} : { value }) }));
  if (request.length > 4096) throw new Error("The secure storage request exceeds its size limit.");
  return new Promise((resolve, reject) => {
    // No secret appears in argv, environment, or logs. The helper is part of the
    // installed Rust executable; no native Node addon/runtime install is needed.
    const child = spawn(executable, ["--credential-vault"], { stdio: ["pipe", "pipe", "ignore"], windowsHide: true });
    let settled = false, chunks = [], length = 0;
    const finish = (error, response) => {
      if (settled) return;
      settled = true; clearTimeout(timer);
      request.fill(0); for (const chunk of chunks) chunk.fill(0); chunks = [];
      if (error) { child.kill(); reject(new Error("The operating system credential store is unavailable.")); }
      else resolve(response);
    };
    const timer = setTimeout(() => finish(true), 7000);
    child.on("error", () => finish(true));
    child.stdin.on("error", () => finish(true));
    child.stdout.on("data", chunk => {
      if (settled) { chunk.fill(0); return; }
      length += chunk.length;
      if (length > 1024) finish(true); else chunks.push(chunk);
    });
    child.on("close", code => {
      if (settled) return;
      try {
        const response = JSON.parse(Buffer.concat(chunks).toString("utf8"));
        if (code !== 0 || !response || Array.isArray(response) || !["ok", "missing"].includes(response.status)) return finish(true);
        if (response.status === "missing" && operation !== "read") return finish(true);
        if (operation === "read" && response.status === "ok" && !/^[a-f0-9]{64}$/.test(response.value)) return finish(true);
        const fields = operation === "read" && response.status === "ok" ? ["status", "value"] : ["status"];
        if (Object.keys(response).some(field => !fields.includes(field))) return finish(true);
        finish(false, response.status === "missing" ? null : response.value ?? true);
      } catch { finish(true); }
    });
    child.stdin.end(request, () => request.fill(0));
  });
}

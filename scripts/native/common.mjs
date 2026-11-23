import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";

export function run(command, args, { cwd, env = process.env, timeout = 15 * 60_000, capture = false } = {}) {
    return new Promise((resolve, reject) => {
        const child = spawn(command, args, { cwd, env, shell: false, windowsHide: true,
            stdio: ["ignore", capture ? "pipe" : "inherit", capture ? "pipe" : "inherit"] });
        let out = "", error = "", bytes = 0;
        const timer = setTimeout(() => { child.kill(); reject(new Error(`${command} timed out`)); }, timeout);
        const collect = (chunk, stderr) => {
            bytes += chunk.length;
            if (bytes > 64 * 1024 * 1024) { child.kill(); reject(new Error(`${command} output exceeded 64 MiB`)); return; }
            if (stderr) error += chunk; else out += chunk;
        };
        child.stdout?.on("data", chunk => collect(chunk, false));
        child.stderr?.on("data", chunk => collect(chunk, true));
        child.once("error", reason => { clearTimeout(timer); reject(reason); });
        child.once("close", (code, signal) => {
            clearTimeout(timer);
            if (code === 0) resolve(out.trim());
            else reject(new Error(`${command} failed (${signal || code})${error ? `: ${error.trim().slice(0, 4000)}` : ""}`));
        });
    });
}

export async function sha256(path) {
    const hash = createHash("sha256");
    for await (const chunk of createReadStream(path)) hash.update(chunk);
    return hash.digest("hex");
}

export function platform(value = process.platform) {
    if (!["linux", "darwin", "win32"].includes(value)) throw new Error(`Unsupported native platform: ${value}`);
    return value;
}
export function architecture(value = process.arch) {
    if (!["x64", "arm64"].includes(value)) throw new Error(`Unsupported native architecture: ${value}`);
    return value;
}
export function version(value, { node = false } = {}) {
    const normalized = String(value).replace(/^v/, "");
    if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(normalized)) throw new Error("A release version must contain exactly three numeric components");
    const parts = normalized.split(".").map(Number);
    if (parts.some(part => part > 65535) || node && parts[0] < 22) throw new Error(node ? "Node.js 22 or newer is required" : "Version components must fit the Windows package version");
    return normalized;
}
export function options(args, allowed) {
    const result = {};
    for (let i = 0; i < args.length; i += 2) {
        const name = args[i]?.replace(/^--/, "");
        if (!args[i]?.startsWith("--") || !allowed.includes(name) || !args[i + 1] || args[i + 1].startsWith("--") || name in result) throw new Error(`Invalid option: ${args[i]}`);
        result[name] = args[i + 1];
    }
    return result;
}

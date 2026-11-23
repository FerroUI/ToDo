#!/usr/bin/env node
import { options } from "./native/common.mjs";
import { fetchRuntime } from "./native/node-runtime.mjs";

try {
    const args = options(process.argv.slice(2), ["out", "version", "platform", "arch", "keyring"]);
    if (!args.out) throw new Error("Usage: node scripts/fetch-native-runtime.mjs --out <new-directory> [--version 24.x.y] [--platform linux|darwin|win32] [--arch x64|arm64] [--keyring <trusted-node-release-keys.kbx>]");
    const result = await fetchRuntime({ out: args.out, nodeVersion: args.version, targetPlatform: args.platform,
        targetArch: args.arch, keyring: args.keyring || process.env.TODO_NODE_RELEASE_KEYRING });
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
} catch (error) { process.stderr.write(`${error.message}\n`); process.exitCode = 1; }

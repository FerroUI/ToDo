#!/usr/bin/env node
import { options } from "./native/common.mjs";
import { packageNative } from "./native/package.mjs";

try {
    const args = options(process.argv.slice(2), ["repo", "version", "platform", "arch", "profile", "binary", "runtime", "out", "formats", "cargo-metadata"]);
    const result = await packageNative(args);
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
} catch (error) { process.stderr.write(`${error.message}\n`); process.exitCode = 1; }

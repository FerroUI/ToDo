import { mkdir, mkdtemp, readFile, writeFile, copyFile, rename, rm, chmod, stat } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { Readable, Transform } from "node:stream";
import { pipeline } from "node:stream/promises";
import { architecture, platform, version, run, sha256 } from "./common.mjs";

export function nodeRelease(nodeVersion, targetPlatform, targetArch) {
    const release = `v${version(nodeVersion, { node: true })}`;
    const os = platform(targetPlatform) === "win32" ? "win" : targetPlatform;
    const root = `node-${release}-${os}-${architecture(targetArch)}`;
    const archive = `${root}.${os === "win" ? "zip" : "tar.gz"}`;
    return { version: release, root, archive, source: `https://nodejs.org/dist/${release}/${archive}`,
        checksums: `https://nodejs.org/dist/${release}/SHASUMS256.txt` };
}

export function releaseChecksum(manifest, archive) {
    const entries = manifest.split(/\r?\n/).map(line => /^([a-f0-9]{64})\s+\*?([^\s]+)$/i.exec(line.trim())).filter(Boolean)
        .filter(entry => entry[2] === archive);
    if (entries.length !== 1) throw new Error("The official Node checksum manifest must contain exactly one matching archive");
    return entries[0][1].toLowerCase();
}

async function download(url, destination, maximum) {
    const response = await fetch(url, { signal: AbortSignal.timeout(120_000), redirect: "error" });
    if (!response.ok || !response.body) throw new Error(`Node download failed: HTTP ${response.status}`);
    if (Number(response.headers.get("content-length")) > maximum) throw new Error("Node download is larger than expected");
    let bytes = 0;
    const bounded = new Transform({ transform(chunk, _, done) {
        bytes += chunk.length;
        done(bytes > maximum ? new Error("Node download exceeded its size limit") : null, chunk);
    } });
    await pipeline(Readable.fromWeb(response.body), bounded, createWriteStream(destination, { flags: "wx" }));
}

/** Fetch the full official archive, verify SHA-256 before extraction, retain its license.
 * Set keyring to an independently trusted Node release keyring for PGP verification too.
 */
export async function fetchRuntime({ out, nodeVersion = process.version, targetPlatform = process.platform, targetArch = process.arch, keyring }) {
    const release = nodeRelease(nodeVersion, targetPlatform, targetArch);
    out = resolve(out);
    await mkdir(dirname(out), { recursive: true });
    const temporary = await mkdtemp(join(dirname(out), ".node-runtime-"));
    try {
        const sums = join(temporary, "SHASUMS256.txt");
        if (keyring) {
            const signed = `${sums}.asc`;
            await download(`${release.checksums}.asc`, signed, 2 * 1024 * 1024);
            await run("gpgv", ["--keyring", resolve(keyring), "--output", sums, signed]);
        } else await download(release.checksums, sums, 2 * 1024 * 1024);
        const expected = releaseChecksum(await readFile(sums, "utf8"), release.archive);
        const archive = join(temporary, release.archive);
        await download(release.source, archive, 350 * 1024 * 1024);
        if (await sha256(archive) !== expected) throw new Error("Node archive SHA-256 did not match the official release manifest");
        const extracted = join(temporary, "extracted");
        await mkdir(extracted);
        if (targetPlatform === "win32") {
            // Paths are environment data; they are never inserted into PowerShell source.
            await run("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; Expand-Archive -LiteralPath $env:TODO_NODE_ARCHIVE -DestinationPath $env:TODO_NODE_EXTRACTED"],
                { env: { ...process.env, TODO_NODE_ARCHIVE: archive, TODO_NODE_EXTRACTED: extracted } });
        } else await run("tar", ["-xzf", archive, "-C", extracted, `${release.root}/bin/node`, `${release.root}/LICENSE`]);
        const ready = join(temporary, "ready");
        await mkdir(ready);
        const binaryName = targetPlatform === "win32" ? "node.exe" : "node";
        const binary = join(ready, binaryName);
        const source = join(extracted, release.root, ...(targetPlatform === "win32" ? [binaryName] : ["bin", binaryName]));
        await copyFile(source, binary);
        await chmod(binary, 0o755);
        await copyFile(join(extracted, release.root, "LICENSE"), join(ready, "LICENSE.txt"));
        await copyFile(sums, join(ready, "SHASUMS256.txt"));
        const metadata = { version: release.version, platform: targetPlatform, arch: targetArch, source: release.source,
            archiveSha256: expected, binarySha256: await sha256(binary), checksumSource: release.checksums, pgpVerified: Boolean(keyring) };
        await writeFile(join(ready, "runtime.json"), `${JSON.stringify(metadata, null, 2)}\n`);
        try { await stat(out); throw new Error(`Runtime output already exists: ${out}. Choose a new output directory.`); }
        catch (error) { if (error.code !== "ENOENT") throw error; }
        await rename(ready, out);
        return metadata;
    } finally { await rm(temporary, { recursive: true, force: true }); }
}

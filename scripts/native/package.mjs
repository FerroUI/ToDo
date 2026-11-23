import { mkdir, mkdtemp, readFile, writeFile, rm, stat } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { platform, architecture, version, run, sha256 } from "./common.mjs";
import { stageBundle } from "./bundle.mjs";
import { packageLinux, packageMac, packageWindows } from "./platforms.mjs";

const repository = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
export const defaultFormats = { linux: ["tar", "deb"], darwin: ["app", "dmg"], win32: ["zip", "msix", "installer"] };

export function packageFormats(targetPlatform, value) {
    const allowed = defaultFormats[platform(targetPlatform)];
    const result = value ? value.split(",") : [...allowed];
    if (!result.length || new Set(result).size !== result.length || result.some(format => !allowed.includes(format))) {
        throw new Error(`Formats for ${targetPlatform}: ${allowed.join(",")}`);
    }
    return result;
}

export async function packageNative(args = {}) {
    const repo = resolve(args.repo || repository);
    const targetPlatform = platform(args.platform), arch = architecture(args.arch);
    if (targetPlatform !== process.platform || arch !== process.arch) throw new Error("Package on the target platform and architecture so the bundled runtime and OS packaging tools can be verified");
    const cargo = await readFile(join(repo, "Cargo.toml"), "utf8");
    const section = cargo.split("[workspace.package]")[1]?.split("\n[")[0];
    const release = version(args.version || /(?:^|\n)version\s*=\s*"([^"]+)"/.exec(section || "")?.[1]);
    const profile = args.profile || "dist";
    if (!["dist", "release", "debug"].includes(profile)) throw new Error("Package profile must be dist, release, or debug");
    const binary = resolve(args.binary || join(repo, "target", profile, targetPlatform === "win32" ? "todo-desktop.exe" : "todo-desktop"));
    if (!(await stat(binary)).isFile()) throw new Error("Build todo-desktop before packaging it");
    const runtime = resolve(args.runtime || join(repo, "target/native/node-runtime"));
    const out = resolve(args.out || join(repo, "target/native/packages"));
    const name = `ferroui-todo-${release}-${{ linux: "linux", darwin: "macos", win32: "windows" }[targetPlatform]}-${arch}`;
    const formats = packageFormats(targetPlatform, args.formats);
    await mkdir(out, { recursive: true });
    const work = await mkdtemp(join(out, ".package-"));
    try {
        let cargoMetadata;
        if (args["cargo-metadata"]) cargoMetadata = JSON.parse(await readFile(resolve(args["cargo-metadata"]), "utf8"));
        else {
            const rustc = await run("rustc", ["-vV"], { cwd: repo, capture: true });
            const host = /^host: ([a-z0-9_-]+)$/m.exec(rustc)?.[1];
            if (!host) throw new Error("The Rust compiler did not identify the build host");
            cargoMetadata = JSON.parse(await run("cargo", ["metadata", "--locked", "--format-version", "1", "--filter-platform", host], { cwd: repo, capture: true }));
        }
        const bundle = await stageBundle({ repo, directory: join(work, name), binary, runtime, release,
            targetPlatform, targetArch: arch, cargoMetadata });
        bundle.manifest.buildProfile = profile;
        await writeFile(join(bundle.resources, "native-package-manifest.json"), `${JSON.stringify(bundle.manifest, null, 2)}\n`);
        const driver = { linux: packageLinux, darwin: packageMac, win32: packageWindows }[targetPlatform];
        const created = await driver({ bundle, repo, work, out, name, release, arch, formats, env: process.env });
        const artifacts = [];
        for (const artifact of created) {
            artifacts.push({ ...artifact, path: basename(artifact.path), bytes: (await stat(artifact.path)).size, sha256: await sha256(artifact.path) });
        }
        const report = { ...bundle.manifest, artifacts, generatedAt: new Date().toISOString(),
            verification: "Packaging commands and requested signatures completed. Installation, desktop notifications and account sign-in require platform acceptance testing." };
        await writeFile(join(out, `${name}.json`), `${JSON.stringify(report, null, 2)}\n`);
        await writeFile(join(out, `${name}.sha256`), artifacts.map(artifact => `${artifact.sha256}  ${artifact.path}`).join("\n") + "\n");
        return report;
    } finally { await rm(work, { recursive: true, force: true }); }
}

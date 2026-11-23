import { cp, copyFile, mkdir, readdir, readFile, stat, writeFile, chmod } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { architecture, platform, run, sha256, version } from "./common.mjs";
import { nodeRelease, releaseChecksum } from "./node-runtime.mjs";

async function file(source, target) {
    await mkdir(dirname(target), { recursive: true });
    await copyFile(source, target);
}

export async function copyServiceAssets(repo, destination) {
    const web = join(repo, "crates/todo-browser/wwwroot");
    for (const [source, target] of [
        [join(repo, "crates/todo-desktop/native"), "native"],
        [join(web, "cloud"), "web/cloud"], [join(web, "backup"), "web/backup"],
        [join(web, "vendor/pst-writer"), "web/vendor/pst-writer"],
    ]) {
        await cp(source, join(destination, target), { recursive: true,
            filter: async path => (await stat(path)).isDirectory() || /\.(?:m?js|json|txt)$/.test(path)
                || /^(?:licen[cs]e|copying|notice)(?:$|[._-])/i.test(basename(path)) });
    }
    for (const name of ["notes.js", "backgrounds.js"]) await file(join(web, name), join(destination, "web", name));
    await writeFile(join(destination, "package.json"), '{"type":"module","private":true}\n');
}

async function notices(directory, depth = 0) {
    const result = [];
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        const path = join(directory, entry.name);
        if (entry.isFile() && /^(?:licen[cs]e|copying|notice|copyright|authors)(?:$|[._-])/i.test(entry.name)) result.push(path);
        else if (depth < 1 && entry.isDirectory() && !["src", "target", "node_modules", ".git"].includes(entry.name)) result.push(...await notices(path, depth + 1));
    }
    return result;
}

export async function copyLicenses(repo, destination, metadata) {
    const fixed = [
        ["LICENSE", "application/LICENSE"], ["NOTICE.md", "application/NOTICE.md"],
        ["Cargo.lock", "Cargo.lock"], ["FerroUI/LICENSE", "FerroUI/LICENSE"], ["FerroUI/NOTICE.md", "FerroUI/NOTICE.md"],
        ["crates/todo-controls/assets/fonts/OFL.txt", "Noto-Color-Emoji/OFL.txt"],
        ["crates/todo-browser/wwwroot/vendor/MSAL-LICENSE.txt", "MSAL-LICENSE.txt"],
        ["crates/todo-browser/wwwroot/vendor/UNICODE-LICENSE.txt", "UNICODE-LICENSE.txt"],
    ];
    for (const [source, name] of fixed) await file(join(repo, source), join(destination, name));
    const root = metadata.packages.find(package_ => package_.name === "todo-desktop");
    if (!root || !metadata.resolve) throw new Error("Cargo metadata is missing the todo-desktop dependency graph");
    const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
    const packages = new Map(metadata.packages.map(package_ => [package_.id, package_]));
    const seen = new Set(), pending = [root.id], index = [];
    while (pending.length) {
        const id = pending.pop();
        if (seen.has(id)) continue;
        seen.add(id);
        pending.push(...(nodes.get(id)?.dependencies || []));
        const package_ = packages.get(id);
        if (!package_) throw new Error("Cargo metadata contains an unresolved package");
        const source = dirname(package_.manifest_path);
        const name = `${package_.name}-${package_.version}`;
        const files = new Set(await notices(source));
        if (package_.license_file) files.add(resolve(source, package_.license_file));
        let counter = 0;
        const copied = [];
        for (const path of files) {
            const target = `${name}/${counter++}-${basename(path)}`;
            await file(path, join(destination, "crates", target));
            copied.push(`crates/${target}`);
        }
        await file(package_.manifest_path, join(destination, "crates", name, "Cargo.toml"));
        index.push({ name: package_.name, version: package_.version, license: package_.license,
            repository: package_.repository, notices: copied });
    }
    index.sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
    await writeFile(join(destination, "rust-dependencies.json"), `${JSON.stringify(index, null, 2)}\n`);
    return index.filter(package_ => !package_.notices.length).map(package_ => `${package_.name}@${package_.version}`);
}

export async function verifyRuntime(directory, targetPlatform, targetArch) {
    const metadata = JSON.parse(await readFile(join(directory, "runtime.json"), "utf8"));
    version(metadata.version, { node: true });
    if (metadata.platform !== platform(targetPlatform) || metadata.arch !== architecture(targetArch)) throw new Error("Bundled Node platform/architecture does not match the application package");
    const release = nodeRelease(metadata.version, targetPlatform, targetArch);
    if (metadata.source !== release.source || metadata.checksumSource !== release.checksums
        || !/^[a-f0-9]{64}$/.test(metadata.archiveSha256) || typeof metadata.pgpVerified !== "boolean"
        || releaseChecksum(await readFile(join(directory, "SHASUMS256.txt"), "utf8"), release.archive) !== metadata.archiveSha256) {
        throw new Error("Bundled Node release provenance is missing or inconsistent; use fetch-native-runtime.mjs");
    }
    const binary = join(directory, targetPlatform === "win32" ? "node.exe" : "node");
    if (await sha256(binary) !== metadata.binarySha256) throw new Error("Bundled Node bytes do not match runtime.json");
    if ((await stat(join(directory, "LICENSE.txt"))).size === 0) throw new Error("Bundled Node license is empty");
    const environment = { ...process.env };
    delete environment.NODE_OPTIONS;
    delete environment.NODE_PATH;
    const actual = JSON.parse(await run(binary, ["-p", "JSON.stringify({version:process.version,platform:process.platform,arch:process.arch})"], { env: environment, capture: true, timeout: 30_000 }));
    if (version(actual.version) !== version(metadata.version) || actual.platform !== metadata.platform || actual.arch !== metadata.arch) throw new Error("Bundled Node self-identification does not match its verified archive metadata");
    return metadata;
}

export async function stageBundle({ repo, directory, binary, runtime, release, targetPlatform, targetArch, cargoMetadata }) {
    release = version(release);
    const metadata = await verifyRuntime(runtime, targetPlatform, targetArch);
    const app = targetPlatform === "darwin" ? join(directory, "To Do.app") : directory;
    const resources = targetPlatform === "darwin" ? join(app, "Contents/Resources") : app;
    const executables = targetPlatform === "darwin" ? join(app, "Contents/MacOS") : app;
    const executable = join(executables, targetPlatform === "win32" ? "todo-desktop.exe" : "todo-desktop");
    await file(binary, executable);
    await chmod(executable, 0o755);
    await cp(runtime, join(resources, "runtime"), { recursive: true });
    await copyServiceAssets(repo, join(resources, "service-assets"));
    const missingNotices = await copyLicenses(repo, join(resources, "licenses"), cargoMetadata);
    const manifest = { application: "FerroUI ToDo", version: release, platform: targetPlatform, arch: targetArch,
        sourceRevision: process.env.GITHUB_SHA || null, node: metadata,
        binarySha256BeforeSigning: await sha256(executable), missingCrateNoticeFiles: missingNotices,
        reminderScope: "Active workspace, current signed-in OS user; opt-in; no network push or wake from power-off" };
    await writeFile(join(resources, "native-package-manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
    await writeFile(join(resources, "README.txt"), `FerroUI To Do ${release}\n\nThe native application includes its service runtime and vendor notices.\nLocal tasks work without an account. Microsoft integration requires a public client registration configured in Settings.\n\nBackground reminders are off until enabled in Settings. They use the active workspace and the current signed-in OS session. They do not wake a powered-off computer or fetch cloud changes while the app is closed.\n\nBefore removing a portable, Debian, macOS, or MSIX package, close To Do and run its executable with --disable-background-reminders. For each custom data directory, also pass --data-dir with that directory's path. The Windows per-user installer removes the default data directory's reminder registration during uninstall; custom directories still need the command above. Task data is retained.\n\nUnsigned packages require your operating system's local installation approval. A package built with signing hooks is signed only when the artifact report says so.\n`);
    return { app, resources, executable, manifest };
}

export async function directorySize(directory) {
    let bytes = 0;
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) bytes += await directorySize(path);
        else if (entry.isFile()) bytes += (await stat(path)).size;
    }
    return bytes;
}

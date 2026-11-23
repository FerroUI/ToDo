import test from "node:test";
import assert from "node:assert/strict";
import { cp, copyFile, link, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { architecture, options, platform, run, sha256, version } from "./native/common.mjs";
import { nodeRelease, releaseChecksum } from "./native/node-runtime.mjs";
import { copyServiceAssets, stageBundle, verifyRuntime } from "./native/bundle.mjs";
import { debControl, macInfoPlist, msixManifest, nsis, windowsInstaller } from "./native/manifests.mjs";
import { packageFormats } from "./native/package.mjs";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..");
async function temporary(t) {
    const path = await mkdtemp(join(tmpdir(), "todo-packaging-"));
    t.after(() => rm(path, { recursive: true, force: true }));
    return path;
}
async function put(root, name, contents = "fixture\n") {
    const path = join(root, name);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, contents);
    return path;
}
async function runtimeFixture(root) {
    await mkdir(root, { recursive: true });
    const executable = join(root, process.platform === "win32" ? "node.exe" : "node");
    try { await link(process.execPath, executable); } catch { await copyFile(process.execPath, executable); }
    const release = nodeRelease(process.version, process.platform, process.arch);
    const metadata = { version: process.version, platform: process.platform, arch: process.arch,
        binarySha256: await sha256(executable), archiveSha256: "a".repeat(64), source: release.source,
        checksumSource: release.checksums, pgpVerified: false };
    await put(root, "LICENSE.txt", "Fixture license: this runtime is used only by packaging tests\n");
    await put(root, "SHASUMS256.txt", `${metadata.archiveSha256}  ${release.archive}\n`);
    await put(root, "runtime.json", JSON.stringify(metadata));
    return { executable, metadata };
}

test("native release inputs reject unsupported targets, versions and option injection", () => {
    assert.equal(version("v24.12.1", { node: true }), "24.12.1");
    for (const bad of ["22.1", "22.01.1", "22.1.1/../../evil", "22.1.1\n", "65536.0.0"]) assert.throws(() => version(bad));
    assert.throws(() => version("20.12.0", { node: true }));
    assert.throws(() => platform("../../linux"));
    assert.throws(() => architecture("ia32"));
    assert.throws(() => options(["--out", "first", "--out", "second"], ["out"]));
    assert.throws(() => options(["--command", "echo"], ["out"]));
    assert.deepEqual(packageFormats("linux", "tar,deb"), ["tar", "deb"]);
    assert.throws(() => packageFormats("linux", "tar,msix"));
    assert.throws(() => packageFormats("win32", "zip,zip"));
});

test("official Node archive selection and exact checksums reject ambiguous manifests", () => {
    const windows = nodeRelease("24.12.1", "win32", "arm64");
    assert.equal(windows.archive, "node-v24.12.1-win-arm64.zip");
    assert.equal(nodeRelease("24.12.1", "darwin", "x64").archive, "node-v24.12.1-darwin-x64.tar.gz");
    const digest = "f".repeat(64), line = `${digest}  ${windows.archive}\n`;
    assert.equal(releaseChecksum(line, windows.archive), digest);
    assert.throws(() => releaseChecksum(line + line, windows.archive));
    assert.throws(() => releaseChecksum(line, windows.archive + ".evil"));
    assert.throws(() => releaseChecksum("invalid digest  " + windows.archive, windows.archive));
});

test("package manifests preserve literal paths and enforce user-level installation", () => {
    assert.equal(nsis('C:\\$DATA\\a"b'), 'C:\\$$DATA\\a$\\"b');
    const installer = windowsInstaller({ release: "1.2.3", source: 'C:\\Tasks $5"', output: 'C:\\out\\To Do.exe' });
    assert.match(installer, /RequestExecutionLevel user/);
    assert.match(installer, /--disable-background-reminders/);
    assert.match(installer, /Tasks \$\$5\$\\"/);
    assert.doesNotMatch(installer, /\bHKLM\b/);
    const msix = msixManifest("1.2.3", "x64", 'CN=Test & "Partners"');
    assert.match(msix, /CN=Test &amp; &quot;Partners&quot;/);
    assert.match(msix, /Version="1\.2\.3\.0"/);
    assert.match(msix, /packagedClassicApp/);
    assert.match(macInfoPlist("1.2.3"), /com\.ferroui\.todo/);
    assert.match(debControl("1.2.3", "arm64", 12, "2.39"), /libc6 \(>= 2\.39\)/);
    assert.throws(() => msixManifest("1.2.3", "x64", "CN=Bad\nInjected"));
    assert.throws(() => debControl("1.2.3", "x64", 12, "2.39\nInjected"));
});

test("the packaging command runner drains output before resolving and never invokes a shell", async () => {
    const value = '$HOME `literal` "quotes" $(not a command)';
    assert.equal(await run(process.execPath, ["-e", "process.stdout.write(process.argv[1])", value], { capture: true }), value);
    const large = await run(process.execPath, ["-e", "process.stdout.write('x'.repeat(512*1024))"], { capture: true });
    assert.equal(large.length, 512 * 1024);
    await assert.rejects(run(process.execPath, ["-e", "process.stderr.write('failed fixture');process.exit(2)"], { capture: true }), /failed fixture/);
});

test("packaged Node is rehashed and self-identifies before it is accepted", async t => {
    const root = await temporary(t), runtime = join(root, "runtime");
    const { metadata } = await runtimeFixture(runtime);
    assert.deepEqual(await verifyRuntime(runtime, process.platform, process.arch), metadata);
    await put(runtime, "runtime.json", JSON.stringify({ ...metadata, binarySha256: "0".repeat(64) }));
    await assert.rejects(verifyRuntime(runtime, process.platform, process.arch), /bytes do not match/);
    await put(runtime, "runtime.json", JSON.stringify({ ...metadata, source: "https://untrusted.invalid/node.zip" }));
    await assert.rejects(verifyRuntime(runtime, process.platform, process.arch), /provenance/);
    await put(runtime, "runtime.json", JSON.stringify(metadata));
    await assert.rejects(verifyRuntime(runtime, process.platform, process.arch === "x64" ? "arm64" : "x64"), /architecture/);
});

test("service resource packages include the real worker, shared formats and vendor notices", async t => {
    const root = await temporary(t);
    await copyServiceAssets(repo, root);
    for (const name of ["native/worker.mjs", "native/vendor/LICENSES.txt", "web/notes.js", "web/backgrounds.js",
        "web/cloud/workspaces.js", "web/backup/pst.js", "web/vendor/pst-writer/index.js", "web/vendor/pst-writer/LICENSE"]) {
        assert.ok((await readFile(join(root, name))).length > 0, name);
    }
    assert.equal(JSON.parse(await readFile(join(root, "package.json"), "utf8")).type, "module");
});

test("complete staging carries runtime, source licenses and transitive crate license records", async t => {
    const root = await temporary(t), fixture = join(root, "repository"), runtime = join(root, "runtime");
    await runtimeFixture(runtime);
    for (const name of ["LICENSE", "NOTICE.md", "Cargo.lock", "FerroUI/LICENSE", "FerroUI/NOTICE.md",
        "crates/todo-controls/assets/fonts/OFL.txt", "crates/todo-browser/wwwroot/vendor/MSAL-LICENSE.txt",
        "crates/todo-browser/wwwroot/vendor/UNICODE-LICENSE.txt", "app/Cargo.toml", "app/LICENSE", "dependency/Cargo.toml", "dependency/COPYING"]) await put(fixture, name);
    for (const name of ["crates/todo-desktop/native/worker.mjs", "crates/todo-browser/wwwroot/cloud/module.js",
        "crates/todo-browser/wwwroot/backup/module.js", "crates/todo-browser/wwwroot/vendor/pst-writer/module.mjs",
        "crates/todo-browser/wwwroot/notes.js", "crates/todo-browser/wwwroot/backgrounds.js"]) await put(fixture, name, "export {};\n");
    const binary = await put(root, "fixture-application", "Application bytes are not executed by the packaging staging test\n");
    const cargoMetadata = { packages: [
        { id: "app", name: "todo-desktop", version: "1.2.3", license: "MIT", manifest_path: join(fixture, "app/Cargo.toml") },
        { id: "dep", name: "dependency", version: "1.0.0", license: "BSD-3-Clause", manifest_path: join(fixture, "dependency/Cargo.toml") },
    ], resolve: { nodes: [{ id: "app", dependencies: ["dep"] }, { id: "dep", dependencies: [] }] } };
    const bundle = await stageBundle({ repo: fixture, directory: join(root, "bundle"), binary, runtime, release: "1.2.3",
        targetPlatform: process.platform, targetArch: process.arch, cargoMetadata });
    assert.equal(await sha256(bundle.executable), await sha256(binary));
    assert.ok((await readFile(join(bundle.resources, "runtime/LICENSE.txt"))).length);
    const index = JSON.parse(await readFile(join(bundle.resources, "licenses/rust-dependencies.json"), "utf8"));
    assert.deepEqual(index.map(package_ => package_.name), ["dependency", "todo-desktop"]);
    assert.ok(index.every(package_ => package_.notices.length === 1));
    assert.deepEqual(bundle.manifest.missingCrateNoticeFiles, []);
    assert.ok((await readFile(join(bundle.resources, "native-package-manifest.json"))).length);
});

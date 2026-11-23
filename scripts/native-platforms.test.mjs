import test from "node:test";
import assert from "node:assert/strict";
import { cp, mkdir, mkdtemp, readFile, readlink, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { minimumGlibc, minimumMacOS, packageLinux, packageMac, packageWindows, peImports, standaloneWindowsImports, windowsSdkTool, windowsSigning } from "./native/platforms.mjs";
import { debianDependencies, inspectDebianDependencies, linkedDebianDependencies } from "./native/debian.mjs";

const MACHO_LEGACY = "Load command 4\n      cmd LC_VERSION_MIN_MACOSX\n  cmdsize 16\n  version 11.0\n      sdk 15.0\n";
const MACHO_NODE = "Load command 8\n      cmd LC_BUILD_VERSION\n  cmdsize 32\n platform 1\n    minos 13.5\n      sdk 15.4\n   ntools 1\n     tool 3\n  version 1115.7.3\n";

function peImage({ imports = ["KERNEL32.dll", "api-ms-win-crt-runtime-l1-1-0.dll"], delayed = [], machine = 0x8664 } = {}) {
    const bytes = Buffer.alloc(4096), pe = 0x80, optional = pe + 24, directories = optional + 112, section = optional + 240;
    bytes.write("MZ"); bytes.writeUInt32LE(pe, 0x3c); bytes.write("PE\0\0", pe);
    bytes.writeUInt16LE(machine, pe + 4); bytes.writeUInt16LE(1, pe + 6); bytes.writeUInt16LE(240, pe + 20);
    bytes.writeUInt16LE(0x20b, optional); bytes.writeBigUInt64LE(0x1_4000_0000n, optional + 24);
    bytes.writeUInt32LE(512, optional + 60); bytes.writeUInt32LE(16, optional + 108);
    bytes.write(".rdata", section); bytes.writeUInt32LE(3584, section + 8); bytes.writeUInt32LE(0x1000, section + 12);
    bytes.writeUInt32LE(3584, section + 16); bytes.writeUInt32LE(512, section + 20);
    let nameOffset = 0x700;
    for (const [index, offset, stride, names] of [[1, 0x200, 20, imports], [13, 0x400, 32, delayed]]) {
        if (!names.length) continue;
        bytes.writeUInt32LE(0x1000 + offset - 512, directories + index * 8);
        bytes.writeUInt32LE((names.length + 1) * stride, directories + index * 8 + 4);
        for (let entry = 0; entry < names.length; entry++) {
            const descriptor = offset + entry * stride;
            if (index === 13) bytes.writeUInt32LE(1, descriptor);
            bytes.writeUInt32LE(0x1000 + nameOffset - 512, descriptor + (index === 1 ? 12 : 4));
            bytes.write(names[entry], nameOffset, "ascii"); nameOffset += names[entry].length + 1;
        }
    }
    return bytes;
}

async function fixture(t, platform) {
    const root = await mkdtemp(join(tmpdir(), "todo packaging $ ' 日本語 café "));
    t.after(() => rm(root, { recursive: true, force: true }));
    const repo = join(root, "repo"), work = join(root, "work"), out = join(root, "out");
    const directory = join(work, "FerroUI-ToDo-1.2.3");
    const app = platform === "darwin" ? join(directory, "To Do.app") : directory;
    const resources = platform === "darwin" ? join(app, "Contents/Resources") : app;
    const executable = platform === "darwin" ? join(app, "Contents/MacOS/todo-desktop") : join(app, platform === "win32" ? "todo-desktop.exe" : "todo-desktop");
    await mkdir(join(resources, "runtime"), { recursive: true });
    await mkdir(join(resources, "licenses/application"), { recursive: true });
    if (platform === "darwin") await mkdir(join(app, "Contents/MacOS"));
    await writeFile(executable, platform === "win32" ? peImage() : "test desktop executable");
    await writeFile(join(resources, "runtime", platform === "win32" ? "node.exe" : "node"), platform === "win32" ? peImage() : "test runtime");
    await writeFile(join(resources, "licenses/application/LICENSE"), "MIT test license");
    await mkdir(join(repo, "crates/todo-desktop/assets"), { recursive: true });
    const png = Buffer.alloc(24);
    Buffer.from("89504e470d0a1a0a", "hex").copy(png);
    png.writeUInt32BE(128, 16); png.writeUInt32BE(128, 20);
    await writeFile(join(repo, "crates/todo-desktop/assets/icon.png"), png);
    await writeFile(join(repo, "LICENSE"), "MIT test license");
    await writeFile(join(repo, "NOTICE.md"), "Test notices");
    return { repo, work, out, root, name: "FerroUI-ToDo-1.2.3", release: "1.2.3", arch: "x64",
        bundle: { app, resources, executable, manifest: {} } };
}

test("Linux package staging preserves runtime bytes and raises the libc dependency to the imported ABI", { skip: process.platform === "win32" }, async t => {
    const options = await fixture(t, "linux"), calls = [];
    const artifacts = await packageLinux({ ...options, formats: ["tar", "deb"], env: {}, runner: async (command, args, extra) => {
        calls.push({ command, args, extra });
        if (command === "dpkg-shlibdeps") return "shlibs:Depends=libc6 (>= 2.38), libstdc++6 (>= 14), libgcc-s1 (>= 4.2)\n";
        return command === "readelf" ? args[1].endsWith("/node") ? "GLIBC_2.9 GLIBC_2.40" : "GLIBC_2.37 GLIBCXX_3.4.30" : "";
    } });
    const staged = join(options.work, "deb");
    assert.equal(await readFile(join(staged, "opt/ferroui-todo/runtime/node"), "utf8"), "test runtime");
    assert.equal(await readlink(join(staged, "usr/bin/ferroui-todo")), "/opt/ferroui-todo/todo-desktop");
    assert.match(await readFile(join(staged, "DEBIAN/control"), "utf8"), /libc6 \(>= 2\.40\)/);
    assert.match(await readFile(join(staged, "DEBIAN/control"), "utf8"), /libstdc\+\+6 \(>= 14\)/);
    assert.match(await readFile(join(staged, "DEBIAN/control"), "utf8"), /libgcc-s1 \(>= 4\.2\)/);
    assert.match(await readFile(join(staged, "usr/share/applications/ferroui-todo.desktop"), "utf8"), /Exec=\/opt\/ferroui-todo\/todo-desktop/);
    assert.equal(artifacts[1].minimumGlibc, "2.40");
    assert.ok(artifacts.every(artifact => artifact.signed === false));
    assert.deepEqual(calls.map(call => call.command), ["tar", "readelf", "readelf", "dpkg-shlibdeps", "dpkg-deb"]);
    assert.deepEqual(calls[3].args, ["-O", `-e${options.bundle.executable}`, `-e${join(options.bundle.resources, "runtime/node")}`]);
    assert.ok(calls[4].args.includes("--root-owner-group"));
    assert.ok(calls[0].args.includes(options.bundle.app.split("/").at(-1)));
});

test("Debian dependency inspection preserves versioned and alternative requirements and fails closed", async t => {
    const linked = linkedDebianDependencies("shlibs:Depends=libc6 (>= 2.35), libstdc++6 (>= 14), libblas3 | libblas.so.3\n");
    const merged = debianDependencies("2.35", linked);
    assert.equal((merged.match(/libc6 \(>= 2\.35\)/g) || []).length, 1);
    assert.equal((merged.match(/libstdc\+\+6/g) || []).length, 1);
    assert.ok(merged.includes("libblas3 | libblas.so.3"));
    assert.ok(merged.includes("libgtk-3-0"), "runtime-loaded GUI libraries retain explicit dependencies");
    assert.throws(() => linkedDebianDependencies(""), /dependency declaration/);
    assert.throws(() => linkedDebianDependencies("shlibs:Depends=\n"), /dependency declaration/);
    assert.throws(() => linkedDebianDependencies("shlibs:Depends=libc6\nshlibs:Depends=libc6\n"), /one dependency/);
    assert.throws(() => debianDependencies("2.35", ["libc6\nInjected: yes"]), /dependency declaration/);
    assert.throws(() => debianDependencies("2.35", ["libc6\n"]), /dependency declaration/);
    const options = await fixture(t, "linux");
    await assert.rejects(inspectDebianDependencies([options.bundle.executable], options.work, {}, async (command, args) => {
        assert.equal(command, "dpkg-shlibdeps");
        assert.ok(!args.includes("--ignore-missing-info"));
        throw new Error("no dependency information found for a linked library");
    }), /no dependency information/);
});

test("GLIBC requirements compare numeric version components and ignore C++ namespace versions", () => {
    assert.equal(minimumGlibc(["GLIBC_2.9 GLIBC_2.40 GLIBCXX_9.9"], "2.28"), "2.40");
    assert.equal(minimumGlibc(["GLIBC_3.1 GLIBC_2.99 GLIBC_3.0"], "2.35"), "3.1");
    assert.equal(minimumGlibc(["no imported glibc symbols"], "2.35"), "2.35");
});

test("macOS signing precedes notarization and ZIP reports application signature separately", { skip: process.platform === "win32" }, async t => {
    const options = await fixture(t, "darwin"), calls = [];
    const artifacts = await packageMac({ ...options, formats: ["app", "dmg"], env: {
        TODO_APPLE_SIGNING_IDENTITY: "Developer ID Application: Example", TODO_APPLE_NOTARY_PROFILE: "existing-keychain-profile",
    }, runner: async (command, args, extra) => {
        calls.push({ command, args, extra });
        if (command.endsWith("otool")) return args[1].endsWith("/node") ? MACHO_NODE : MACHO_LEGACY;
        if (command.endsWith("ditto") && args.length === 2) await cp(args[0], args[1], { recursive: true });
        return args[0] === "notarytool" ? JSON.stringify({ status: "Accepted", id: "test-submission" }) : "";
    } });
    const sign = calls.filter(call => call.command.endsWith("codesign") && call.args.includes("--sign"));
    assert.equal(sign.length, 3);
    assert.equal(sign[0].args.at(-1), join(options.bundle.resources, "runtime/node"));
    assert.ok(sign.every(call => !call.args.includes("--deep")));
    assert.ok(sign.slice(0, 2).every(call => call.args.includes("runtime") && call.args.includes("--timestamp")));
    assert.ok(calls.some(call => call.args[0] === "stapler" && call.args[1] === "validate" && call.args[2] === options.bundle.app));
    assert.equal(await readlink(join(options.work, "dmg/Applications")), "/Applications");
    assert.equal(artifacts[0].signed, false, "a ZIP itself is not an Apple signed container");
    assert.equal(artifacts[0].applicationSigned, true);
    assert.equal(artifacts[0].applicationNotarized, true);
    assert.equal(artifacts[1].signed, true);
    assert.equal(artifacts[1].notarized, true);
    assert.equal(artifacts[1].minimumMacOS, "13.5");
    assert.match(await readFile(join(options.bundle.app, "Contents/Info.plist"), "utf8"), /LSMinimumSystemVersion<\/key><string>13\.5<\/string>/);
    assert.equal(JSON.parse(await readFile(join(options.bundle.resources, "native-package-manifest.json"), "utf8")).minimumMacOS, "13.5");
});

test("macOS requirements combine legacy and modern Mach-O commands and reject unknown platforms", () => {
    assert.equal(minimumMacOS([MACHO_LEGACY, MACHO_NODE]), "13.5");
    assert.equal(minimumMacOS([MACHO_NODE.replace("minos 13.5", "minos 14.2.1"), MACHO_LEGACY]), "14.2.1");
    assert.throws(() => minimumMacOS([MACHO_NODE.replace("platform 1", "platform 2")]), /native macOS/);
    assert.throws(() => minimumMacOS([MACHO_NODE.replace("minos 13.5", "minos bad")]), /malformed/);
    assert.throws(() => minimumMacOS([""]), /deployment-version metadata/);
});

test("notarization refusal never advances to stapling or successful artifact reporting", async t => {
    const options = await fixture(t, "darwin"), calls = [];
    await assert.rejects(packageMac({ ...options, formats: ["app"], env: {
        TODO_APPLE_SIGNING_IDENTITY: "Developer ID Application: Example", TODO_APPLE_NOTARY_PROFILE: "existing-profile",
    }, runner: async (command, args) => {
        calls.push({ command, args });
        if (command.endsWith("otool")) return MACHO_NODE;
        return args[0] === "notarytool" ? '{"status":"Invalid","id":"refused"}' : "";
    } }), /not accepted \(Invalid; submission refused\)/);
    assert.ok(!calls.some(call => call.args[0] === "stapler"));
});

test("Windows packaging keeps source paths out of PowerShell source and validates MSIX by default", async t => {
    const options = await fixture(t, "win32"), calls = [];
    const artifacts = await packageWindows({ ...options, formats: ["zip", "msix", "installer"], env: {
        TODO_MAKEAPPX: "makeappx-test.exe", TODO_MAKENSIS: "makensis-test.exe", TODO_MSIX_PUBLISHER: "CN=Test & Example",
    }, runner: async (command, args, extra) => { calls.push({ command, args, extra }); return ""; } });
    assert.equal(artifacts.length, 3);
    assert.ok(artifacts.every(artifact => artifact.signed === false && artifact.applicationSigned === false));
    assert.equal(artifacts[2].uninstallerSigned, false);
    const powershell = calls.filter(call => call.command === "powershell.exe");
    assert.equal(powershell.length, 2);
    assert.ok(powershell.every(call => !call.args.at(-1).includes(options.root)));
    assert.equal(powershell[0].extra.env.TODO_PACKAGE_SOURCE, options.bundle.app);
    const makeappx = calls.find(call => call.command === "makeappx-test.exe");
    assert.ok(makeappx.args.includes("SHA256"));
    assert.ok(!makeappx.args.includes("/nv"), "schema and package validation must remain enabled");
    const manifest = await readFile(join(options.work, "msix/AppxManifest.xml"), "utf8");
    assert.match(manifest, /Publisher="CN=Test &amp; Example"/);
    assert.match(manifest, /packagedClassicApp/);
    const installer = await readFile(join(options.work, "installer.nsi"), "utf8");
    assert.equal(installer.codePointAt(0), 0xfeff, "NSIS must decode UTF-8 paths independently of the Windows ANSI code page");
    assert.match(installer, /RequestExecutionLevel user/);
    assert.match(installer, /--disable-background-reminders/);
    assert.ok(installer.includes(options.bundle.app.replaceAll("$", () => "$$")), "NSIS paths must preserve literal dollar signs");
    assert.ok(calls.every(call => !/schtasks|launchctl|systemctl|msiexec/i.test(call.command)));
});

test("PE import inspection includes delay imports and accepts inbox UCRT dependencies", () => {
    const bytes = peImage({ imports: ["KERNEL32.dll", "UCRTBASE.dll", "msvcrt.dll", "msvcp_win.dll"],
        delayed: ["api-ms-win-crt-runtime-l1-1-0.dll", "USER32.dll"], machine: 0xaa64 });
    const parsed = peImports(bytes);
    assert.equal(parsed.machine, 0xaa64);
    assert.deepEqual(parsed.imports, ["api-ms-win-crt-runtime-l1-1-0.dll", "kernel32.dll", "msvcp_win.dll", "msvcrt.dll", "ucrtbase.dll", "user32.dll"]);
    assert.deepEqual(standaloneWindowsImports(bytes, "arm64"), parsed.imports);
    assert.throws(() => standaloneWindowsImports(bytes, "x64"), /architecture does not match/);
    assert.throws(() => standaloneWindowsImports(peImage({ delayed: ["MSVCP140.dll"] }), "x64"), /unbundled compiler runtime DLLs: msvcp140.dll/);
    assert.throws(() => standaloneWindowsImports(peImage({ imports: ["VCRUNTIME140_1.dll"] }), "x64"), /unbundled compiler runtime DLLs: vcruntime140_1.dll/);
    assert.throws(() => standaloneWindowsImports(peImage({ imports: ["ucrtbased.dll"] }), "x64"), /unbundled compiler runtime/);
});

test("malformed PE import addresses and truncated directories fail closed", () => {
    const outside = peImage();
    outside.writeUInt32LE(0xffff0000, 0x200 + 12);
    assert.throws(() => peImports(outside), /no unique file-backed section/);
    const unterminated = peImage();
    unterminated.writeUInt32LE(40, 0x80 + 24 + 112 + 8 + 4);
    assert.throws(() => peImports(unterminated), /unterminated/);
    assert.throws(() => peImports(peImage().subarray(0, 64)), /outside the file/);
});

test("Windows packaging stops before signing or archives when either executable needs the VC redistributable", async t => {
    for (const which of ["application", "node"]) {
        const options = await fixture(t, "win32");
        const target = which === "application" ? options.bundle.executable : join(options.bundle.resources, "runtime/node.exe");
        await writeFile(target, peImage({ delayed: ["vcruntime140.dll"] }));
        await assert.rejects(packageWindows({ ...options, formats: ["zip"], env: {}, runner: () => assert.fail("must not invoke a platform tool") }), /unbundled compiler runtime DLLs: vcruntime140.dll/);
    }
});

test("Windows signing rejects conflicting credentials and redacts PFX secrets on command failure", async t => {
    assert.throws(() => windowsSigning({ TODO_WINDOWS_CERT_THUMBPRINT: "a".repeat(40), TODO_WINDOWS_PFX: "test.pfx" }), /either/);
    assert.throws(() => windowsSigning({ TODO_WINDOWS_CERT_THUMBPRINT: "not a thumbprint" }), /40 hexadecimal/);
    assert.throws(() => windowsSigning({ TODO_WINDOWS_PFX_PASSWORD: "unused" }), /requires/);
    assert.equal(windowsSigning({}), null);
    const options = await fixture(t, "win32"), password = "fixture-pfx-secret";
    await assert.rejects(packageWindows({ ...options, formats: ["zip"], env: {
        TODO_SIGNTOOL: "signtool-test.exe", TODO_WINDOWS_PFX: "/fixture.pfx", TODO_WINDOWS_PFX_PASSWORD: password,
    }, runner: async () => { throw new Error(`fixture signing failure ${password}`); } }), error => {
        assert.match(error.message, /fixture signing failure \[redacted\]/);
        assert.ok(!error.message.includes(password));
        return true;
    });
});

test("SDK discovery prefers the newest numeric installed version without requiring global PATH changes", async t => {
    const root = await mkdtemp(join(tmpdir(), "todo-sdk-"));
    t.after(() => rm(root, { recursive: true, force: true }));
    for (const version of ["10.0.9999.0", "10.0.26100.0"]) {
        await mkdir(join(root, version, "x64"), { recursive: true });
        await writeFile(join(root, version, "x64/makeappx.exe"), "fixture");
    }
    assert.equal(await windowsSdkTool("makeappx", { WindowsSdkBinPath: root }), join(root, "10.0.26100.0/x64/makeappx.exe"));
    assert.equal(await windowsSdkTool("makeappx", { TODO_MAKEAPPX: "/explicit/tool.exe" }), "/explicit/tool.exe");
});

import { copyFile, cp, mkdir, readdir, readFile, rm, stat, symlink, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { run } from "./common.mjs";
import { directorySize } from "./bundle.mjs";
import { debControl, desktopEntry, macInfoPlist, msixManifest, nodeEntitlements, windowsInstaller } from "./manifests.mjs";
import { inspectDebianDependencies } from "./debian.mjs";

async function file(source, target) {
    await mkdir(dirname(target), { recursive: true });
    await copyFile(source, target);
}

function selections(formats, supported) {
    const requested = new Set(formats);
    if (!requested.size || [...requested].some(format => !supported.includes(format))) {
        throw new Error(`Choose supported package formats: ${supported.join(", ")}`);
    }
    return requested;
}

/** The largest imported GLIBC symbol is the executable's actual minimum ABI. */
export function minimumGlibc(outputs, floor = "2.35") {
    let maximum = floor.split(".").map(Number);
    for (const output of outputs) {
        for (const match of output.matchAll(/\bGLIBC_(\d+)\.(\d+)(?:\.(\d+))?\b/g)) {
            const candidate = match.slice(1).filter(value => value !== undefined).map(Number);
            if (candidate.some((value, index) => value > (maximum[index] || 0) && candidate.slice(0, index).every((prefix, part) => prefix === (maximum[part] || 0)))) maximum = candidate;
        }
    }
    return maximum.join(".");
}

export async function packageLinux({ bundle, repo, work, out, name, release, arch, formats, env = process.env, runner = run }) {
    const requested = selections(formats, ["tar", "deb"]);
    const artifacts = [];
    await mkdir(out, { recursive: true });
    if (requested.has("tar")) {
        const path = join(out, `${name}.tar.gz`);
        await runner("tar", ["--owner=0", "--group=0", "-czf", path, "-C", dirname(bundle.app), basename(bundle.app)], { env });
        artifacts.push({ path, signed: false, applicationSigned: false });
    }
    if (requested.has("deb")) {
        const root = join(work, "deb");
        await cp(bundle.app, join(root, "opt/ferroui-todo"), { recursive: true });
        await mkdir(join(root, "usr/bin"), { recursive: true });
        await symlink("/opt/ferroui-todo/todo-desktop", join(root, "usr/bin/ferroui-todo"));
        const icon = join(repo, "crates/todo-desktop/assets/icon.png");
        const png = await readFile(icon);
        if (png.length < 24 || png.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a") throw new Error("The native application icon must be a PNG");
        const width = png.readUInt32BE(16), height = png.readUInt32BE(20);
        if (!width || !height || width > 4096 || height > 4096) throw new Error("Invalid native icon dimensions");
        await file(icon, join(root, `usr/share/icons/hicolor/${width}x${height}/apps/ferroui-todo.png`));
        await mkdir(join(root, "usr/share/applications"), { recursive: true });
        await writeFile(join(root, "usr/share/applications/ferroui-todo.desktop"), desktopEntry());
        await file(join(repo, "LICENSE"), join(root, "usr/share/doc/ferroui-todo/copyright"));
        await file(join(repo, "NOTICE.md"), join(root, "usr/share/doc/ferroui-todo/NOTICE.md"));
        const executables = [bundle.executable, join(bundle.resources, "runtime/node")];
        const symbols = await Promise.all(executables.map(path =>
            runner("readelf", ["--version-info", path], { env, capture: true })));
        const minimum = minimumGlibc(symbols);
        const linked = await inspectDebianDependencies(executables, work, env, runner);
        const installedKiB = Math.ceil(await directorySize(root) / 1024);
        await mkdir(join(root, "DEBIAN"), { mode: 0o755 });
        await writeFile(join(root, "DEBIAN/control"), debControl(release, arch, installedKiB, minimum, linked), { mode: 0o644 });
        const path = join(out, `${name}.deb`);
        await runner("dpkg-deb", ["--build", "--root-owner-group", root, path], { env });
        artifacts.push({ path, signed: false, applicationSigned: false, minimumGlibc: minimum, linkedDependencies: linked });
    }
    return artifacts;
}

async function macIcon(repo, work, resources, env, runner) {
    const iconset = join(work, "ToDo.iconset");
    await mkdir(iconset);
    const source = join(repo, "crates/todo-desktop/assets/icon.png");
    for (const size of [16, 32, 128, 256, 512]) {
        for (const scale of [1, 2]) {
            const pixels = size * scale;
            const destination = join(iconset, `icon_${size}x${size}${scale === 2 ? "@2x" : ""}.png`);
            await runner("/usr/bin/sips", ["--resampleHeightWidth", String(pixels), String(pixels), source, "--out", destination], { env, capture: true });
        }
    }
    await runner("/usr/bin/iconutil", ["--convert", "icns", "--output", join(resources, "ToDo.icns"), iconset], { env });
}

async function notarize(path, profile, env, runner) {
    const output = await runner("/usr/bin/xcrun", ["notarytool", "submit", path, "--keychain-profile", profile, "--wait", "--output-format", "json"],
        { env, capture: true, timeout: 60 * 60_000 });
    let result;
    try { result = JSON.parse(output); } catch { throw new Error("Apple notarization returned an invalid result"); }
    if (result.status !== "Accepted") throw new Error(`Apple notarization was not accepted (${result.status || "unknown"}; submission ${result.id || "unknown"})`);
}

async function staple(path, env, runner) {
    await runner("/usr/bin/xcrun", ["stapler", "staple", path], { env });
    await runner("/usr/bin/xcrun", ["stapler", "validate", path], { env });
}

/** Use linked image requirements, including the bundled service runtime, for the app gate. */
export function minimumMacOS(outputs, floor = "13.5") {
    const parts = value => {
        if (!/^\d+\.\d+(?:\.\d+)?$/.test(value)) throw new Error("A Mach-O minimum macOS version is malformed");
        const numbers = value.split(".").map(Number);
        if (numbers.some(number => number > 65535)) throw new Error("A Mach-O minimum macOS version is out of range");
        return [numbers[0], numbers[1], numbers[2] || 0];
    };
    let maximum = parts(floor);
    for (const output of outputs) {
        let found = false;
        for (const block of output.split(/^\s*Load command\s+\d+\s*$/m)) {
            let value;
            if (/^\s*cmd\s+LC_BUILD_VERSION\s*$/m.test(block)) {
                if (!/^\s*platform\s+(?:1|MACOS)\s*$/mi.test(block)) continue;
                value = /^\s*minos\s+(\S+)\s*$/m.exec(block)?.[1];
            } else if (/^\s*cmd\s+LC_VERSION_MIN_MACOSX\s*$/m.test(block)) {
                value = /^\s*version\s+(\S+)\s*$/m.exec(block)?.[1];
            } else continue;
            if (!value) throw new Error("A Mach-O image has no readable minimum macOS version");
            const candidate = parts(value);
            if (candidate.some((number, index) => number > maximum[index] && candidate.slice(0, index).every((prefix, part) => prefix === maximum[part]))) maximum = candidate;
            found = true;
        }
        if (!found) throw new Error("The package must contain native macOS Mach-O images with deployment-version metadata");
    }
    return `${maximum[0]}.${maximum[1]}${maximum[2] ? `.${maximum[2]}` : ""}`;
}

export async function packageMac({ bundle, repo, work, out, name, release, formats, env = process.env, runner = run }) {
    const requested = selections(formats, ["app", "dmg"]);
    const identity = env.TODO_APPLE_SIGNING_IDENTITY?.trim();
    const profile = env.TODO_APPLE_NOTARY_PROFILE?.trim();
    if (identity === "-") throw new Error("Use a Developer ID identity for distributable macOS signing; ad-hoc signing is not a release signature");
    if (profile && !identity) throw new Error("Apple notarization requires TODO_APPLE_SIGNING_IDENTITY");
    const requirements = await Promise.all([bundle.executable, join(bundle.resources, "runtime/node")].map(path =>
        runner("/usr/bin/otool", ["-l", path], { env, capture: true })));
    const minimum = minimumMacOS(requirements);
    await mkdir(out, { recursive: true });
    await writeFile(join(bundle.app, "Contents/Info.plist"), macInfoPlist(release, minimum));
    bundle.manifest.minimumMacOS = minimum;
    await writeFile(join(bundle.resources, "native-package-manifest.json"), `${JSON.stringify(bundle.manifest, null, 2)}\n`);
    await macIcon(repo, work, bundle.resources, env, runner);
    let applicationSigned = false, applicationNotarized = false;
    if (identity) {
        const entitlements = join(work, "node-entitlements.plist");
        await writeFile(entitlements, nodeEntitlements);
        await runner("/usr/bin/codesign", ["--force", "--sign", identity, "--options", "runtime", "--timestamp", "--entitlements", entitlements, join(bundle.resources, "runtime/node")], { env });
        // Sign individual nested executables first; never use --deep for signing.
        await runner("/usr/bin/codesign", ["--force", "--sign", identity, "--options", "runtime", "--timestamp", bundle.app], { env });
        await runner("/usr/bin/codesign", ["--verify", "--deep", "--strict", "--verbose=2", bundle.app], { env });
        applicationSigned = true;
    }
    if (profile) {
        const submission = join(work, "notarization.app.zip");
        await runner("/usr/bin/ditto", ["-c", "-k", "--sequesterRsrc", "--keepParent", bundle.app, submission], { env });
        await notarize(submission, profile, env, runner);
        await staple(bundle.app, env, runner);
        applicationNotarized = true;
    }
    const artifacts = [];
    if (requested.has("app")) {
        const path = join(out, `${name}.app.zip`);
        await rm(path, { force: true });
        await runner("/usr/bin/ditto", ["-c", "-k", "--sequesterRsrc", "--keepParent", bundle.app, path], { env });
        artifacts.push({ path, signed: false, applicationSigned, applicationNotarized, notarized: applicationNotarized, minimumMacOS: minimum });
    }
    if (requested.has("dmg")) {
        const contents = join(work, "dmg");
        await mkdir(contents);
        // ditto preserves the signed bundle's extended metadata and stapled ticket.
        await runner("/usr/bin/ditto", [bundle.app, join(contents, "To Do.app")], { env });
        await symlink("/Applications", join(contents, "Applications"));
        const path = join(out, `${name}.dmg`);
        await runner("/usr/bin/hdiutil", ["create", "-volname", "To Do", "-srcfolder", contents, "-ov", "-format", "UDZO", path], { env });
        let signed = false, notarized = false;
        if (identity) {
            await runner("/usr/bin/codesign", ["--force", "--sign", identity, "--timestamp", path], { env });
            await runner("/usr/bin/codesign", ["--verify", "--strict", "--verbose=2", path], { env });
            signed = true;
        }
        if (profile) { await notarize(path, profile, env, runner); await staple(path, env, runner); notarized = true; }
        artifacts.push({ path, signed, notarized, applicationSigned, applicationNotarized, minimumMacOS: minimum });
    }
    return artifacts;
}

async function exists(path) {
    try { return (await stat(path)).isFile(); } catch (error) { if (error.code === "ENOENT") return false; throw error; }
}

/** Discover the latest installed SDK, or honor the release operator's explicit tool. */
export async function windowsSdkTool(name, env = process.env) {
    const explicit = env[name === "makeappx" ? "TODO_MAKEAPPX" : "TODO_SIGNTOOL"];
    if (explicit) return explicit;
    const roots = [...new Set([env.WindowsSdkVerBinPath, env.WindowsSdkBinPath,
        join(env["ProgramFiles(x86)"] || "C:\\Program Files (x86)", "Windows Kits", "10", "bin")].filter(Boolean))];
    for (const root of roots) {
        let versions = [];
        try { versions = (await readdir(root, { withFileTypes: true })).filter(entry => entry.isDirectory() && /^\d+\.\d+\.\d+\.\d+$/.test(entry.name))
            .map(entry => entry.name).sort((left, right) => right.localeCompare(left, "en", { numeric: true })); }
        catch (error) { if (error.code !== "ENOENT") throw error; }
        for (const directory of [root, ...versions.map(version => join(root, version))]) {
            for (const arch of process.arch === "arm64" ? ["arm64", "x64", "x86"] : ["x64", "x86"]) {
                const path = join(directory, arch, `${name}.exe`);
                if (await exists(path)) return path;
            }
        }
    }
    throw new Error(`The Windows SDK ${name}.exe tool is unavailable; install the Windows SDK or set ${name === "makeappx" ? "TODO_MAKEAPPX" : "TODO_SIGNTOOL"}`);
}

export function windowsSigning(env = process.env) {
    const thumbprint = env.TODO_WINDOWS_CERT_THUMBPRINT?.trim();
    const pfx = env.TODO_WINDOWS_PFX;
    if (thumbprint && pfx) throw new Error("Select either a certificate thumbprint or a PFX file for Windows signing");
    if (thumbprint && !/^[a-f0-9]{40}$/i.test(thumbprint)) throw new Error("The Windows certificate thumbprint must contain 40 hexadecimal digits");
    if (env.TODO_WINDOWS_PFX_PASSWORD && !pfx) throw new Error("A Windows PFX password requires TODO_WINDOWS_PFX");
    if (!thumbprint && !pfx) return null;
    const timestamp = env.TODO_WINDOWS_TIMESTAMP_URL || "http://timestamp.digicert.com";
    const url = new URL(timestamp);
    if (!["http:", "https:"].includes(url.protocol) || url.username || url.password) throw new Error("Use an HTTP(S) RFC 3161 timestamp endpoint without embedded credentials");
    const args = ["sign", "/fd", "SHA256", "/tr", timestamp, "/td", "SHA256"];
    if (thumbprint) args.push("/sha1", thumbprint);
    else {
        args.push("/f", resolve(pfx));
        if (env.TODO_WINDOWS_PFX_PASSWORD) args.push("/p", env.TODO_WINDOWS_PFX_PASSWORD);
    }
    return { args, password: env.TODO_WINDOWS_PFX_PASSWORD || "" };
}

const WINDOWS_ASSETS = String.raw`$ErrorActionPreference='Stop';
Add-Type -AssemblyName System.Drawing;
$source=[System.Drawing.Image]::FromFile($env:TODO_PACKAGE_ICON);
try {
 foreach($spec in @(@('StoreLogo.png',50),@('Square44x44Logo.png',44),@('Square150x150Logo.png',150))) {
  $size=[int]$spec[1];
  $bitmap=New-Object System.Drawing.Bitmap($size,$size);
  $graphics=[System.Drawing.Graphics]::FromImage($bitmap);
  try {
   $graphics.Clear([System.Drawing.Color]::Transparent);
   $graphics.InterpolationMode=[System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic;
   $scale=[Math]::Min($size/$source.Width,$size/$source.Height);
   $width=[int]($source.Width*$scale); $height=[int]($source.Height*$scale);
   $graphics.DrawImage($source,[int](($size-$width)/2),[int](($size-$height)/2),$width,$height);
   $bitmap.Save((Join-Path $env:TODO_PACKAGE_ASSETS $spec[0]),[System.Drawing.Imaging.ImageFormat]::Png);
  } finally { $graphics.Dispose(); $bitmap.Dispose(); }
 }
} finally { $source.Dispose(); }`;

const WINDOWS_ZIP = String.raw`$ErrorActionPreference='Stop';
Add-Type -AssemblyName System.IO.Compression.FileSystem;
if([System.IO.File]::Exists($env:TODO_PACKAGE_ZIP)) { [System.IO.File]::Delete($env:TODO_PACKAGE_ZIP); }
[System.IO.Compression.ZipFile]::CreateFromDirectory($env:TODO_PACKAGE_SOURCE,$env:TODO_PACKAGE_ZIP,[System.IO.Compression.CompressionLevel]::Optimal,$true);`;

const WINDOWS_PUBLISHER = String.raw`$ErrorActionPreference='Stop';
if($env:TODO_WINDOWS_CERT_THUMBPRINT) {
 (Get-Item -LiteralPath ('Cert:\CurrentUser\My\'+$env:TODO_WINDOWS_CERT_THUMBPRINT)).Subject;
} else {
 $password=New-Object System.Security.SecureString;
 if($env:TODO_WINDOWS_PFX_PASSWORD) { $password=ConvertTo-SecureString $env:TODO_WINDOWS_PFX_PASSWORD -AsPlainText -Force; }
 try { ((Get-PfxData -FilePath $env:TODO_WINDOWS_PFX -Password $password).EndEntityCertificates | Select-Object -First 1).Subject; }
 finally { $password.Dispose(); }
}`;

function redact(error, password) {
    if (!password) return error;
    return new Error(String(error.message || error).replaceAll(password, "[redacted]"));
}

/** Parse normal and delay-load DLL imports without executing the image or an SDK tool.
 * PE/COFF layout: https://learn.microsoft.com/windows/win32/debug/pe-format
 */
export function peImports(bytes) {
    const fail = message => { throw new Error(`Invalid Windows PE image: ${message}`); };
    const range = (offset, length) => {
        if (!Number.isSafeInteger(offset) || offset < 0 || length < 0 || offset + length > bytes.length) fail("a header or import points outside the file");
        return offset;
    };
    const u16 = offset => bytes.readUInt16LE(range(offset, 2));
    const u32 = offset => bytes.readUInt32LE(range(offset, 4));
    if (!Buffer.isBuffer(bytes) || bytes.length < 64 || bytes[0] !== 0x4d || bytes[1] !== 0x5a) fail("DOS header is missing");
    const pe = u32(0x3c);
    if (pe < 64 || bytes.toString("hex", range(pe, 24), pe + 4) !== "50450000") fail("PE signature is missing");
    const machine = u16(pe + 4), count = u16(pe + 6), optionalSize = u16(pe + 20), optional = pe + 24;
    const magic = u16(optional);
    if (![0x10b, 0x20b].includes(magic)) fail("optional-header format is unsupported");
    const directoryOffset = magic === 0x20b ? 112 : 96;
    range(optional, optionalSize);
    if (optionalSize < directoryOffset || count < 1 || count > 96) fail("section or optional-header count is invalid");
    const directories = u32(optional + directoryOffset - 4);
    if (directories > 16 || optionalSize < directoryOffset + directories * 8) fail("data directories exceed the optional header");
    const headerSize = u32(optional + 60);
    range(0, headerSize);
    if (headerSize < optional + optionalSize + count * 40) fail("section headers exceed the header region");
    const imageBase = magic === 0x20b ? bytes.readBigUInt64LE(range(optional + 24, 8)) : BigInt(u32(optional + 28));
    const sections = [];
    for (let index = 0; index < count; index++) {
        const section = range(optional + optionalSize + index * 40, 40);
        const address = u32(section + 12), size = u32(section + 16), offset = u32(section + 20);
        range(offset, size);
        sections.push({ address, size, offset });
    }
    const rva = (address, size = 1) => {
        if (!Number.isSafeInteger(address) || address < 0 || address > 0xffff_ffff) fail("an RVA is out of range");
        if (address < headerSize && address + size <= headerSize) return range(address, size);
        const matches = sections.filter(section => address >= section.address && address + size <= section.address + section.size);
        if (matches.length !== 1) fail("an RVA has no unique file-backed section");
        return range(matches[0].offset + address - matches[0].address, size);
    };
    const dllName = address => {
        let name = "";
        for (let index = 0; index < 260; index++) {
            const byte = bytes[rva(address + index)];
            if (byte === 0) {
                if (!name || /[\\/]/.test(name)) fail("an imported DLL name is not a file name");
                return name.toLowerCase();
            }
            if (byte < 32 || byte > 126) fail("an imported DLL name is not ASCII");
            name += String.fromCharCode(byte);
        }
        fail("an imported DLL name is unterminated");
    };
    const imports = new Set();
    for (const [directoryIndex, stride] of [[1, 20], [13, 32]]) {
        if (directories <= directoryIndex) continue;
        const directory = optional + directoryOffset + directoryIndex * 8;
        const address = u32(directory), size = u32(directory + 4);
        if (address === 0 && size === 0) continue;
        if (!address || size < stride) fail("an import directory has an invalid size");
        let terminated = false;
        for (let offset = 0; offset + stride <= size && offset / stride < 4096; offset += stride) {
            const descriptor = rva(address + offset, stride);
            if (bytes.subarray(descriptor, descriptor + stride).every(byte => byte === 0)) { terminated = true; break; }
            let name = u32(descriptor + (directoryIndex === 1 ? 12 : 4));
            if (!name) fail("an import descriptor has no DLL name");
            if (directoryIndex === 13) {
                const attributes = u32(descriptor);
                if (attributes & ~1) fail("delay-import attributes are unsupported");
                if (!(attributes & 1)) {
                    const relative = BigInt(name) - imageBase;
                    if (relative <= 0n || relative > 0xffff_ffffn) fail("a delay-import virtual address is invalid");
                    name = Number(relative);
                }
            }
            imports.add(dllName(name));
        }
        if (!terminated) fail("an import directory is unterminated");
    }
    return { machine, imports: [...imports].sort() };
}

export function standaloneWindowsImports(bytes, arch) {
    const parsed = peImports(bytes);
    const expected = { x64: 0x8664, arm64: 0xaa64 }[arch];
    if (!expected || parsed.machine !== expected) throw new Error(`Windows executable architecture does not match ${arch}`);
    // Windows 10+ includes ucrtbase.dll and api-ms-win-crt-* API-set forwarders.
    // Compiler-versioned VC/MFC/OpenMP runtimes and the debug UCRT are not inbox
    // dependencies, and this package deliberately has no admin redist installer.
    const redistributable = parsed.imports.filter(name => /^(?:(?:vcruntime|msvcp|msvcr|vcomp|concrt|vccorlib|mfc|mfcm)\d[^/]*|ucrtbased)\.dll$/i.test(name)
        || /^(?:lib)?clang_rt\..*dynamic.*\.dll$/i.test(name));
    if (redistributable.length) throw new Error(`Windows executable requires unbundled compiler runtime DLLs: ${redistributable.join(", ")}. Rebuild with -C target-feature=+crt-static before packaging.`);
    return parsed.imports;
}

export async function packageWindows({ bundle, repo, work, out, name, release, arch, formats, env = process.env, runner = run }) {
    const requested = selections(formats, ["zip", "msix", "installer"]);
    const signing = windowsSigning(env);
    const tool = signing ? await windowsSdkTool("signtool", env) : null;
    const importedLibraries = {};
    for (const [name, path] of [["todo-desktop.exe", bundle.executable], ["runtime/node.exe", join(bundle.resources, "runtime/node.exe")]]) {
        try { importedLibraries[name] = standaloneWindowsImports(await readFile(path), arch); }
        catch (error) { throw new Error(`${name}: ${error.message}`); }
    }
    await writeFile(join(bundle.resources, "windows-runtime-imports.json"), `${JSON.stringify({ arch, importedLibraries }, null, 2)}\n`);
    await mkdir(out, { recursive: true });
    const sign = async path => {
        if (!signing) return false;
        try {
            await runner(tool, [...signing.args, path], { env, capture: true });
            await runner(tool, ["verify", "/pa", "/all", path], { env, capture: true });
        } catch (error) { throw redact(error, signing.password); }
        return true;
    };
    const applicationSigned = await sign(bundle.executable);
    const artifacts = [];
    if (requested.has("zip")) {
        const path = join(out, `${name}.zip`);
        await runner("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", WINDOWS_ZIP],
            { env: { ...env, TODO_PACKAGE_SOURCE: bundle.app, TODO_PACKAGE_ZIP: path } });
        artifacts.push({ path, signed: false, applicationSigned });
    }
    if (requested.has("msix")) {
        const stage = join(work, "msix");
        await cp(bundle.app, stage, { recursive: true });
        const assets = join(stage, "Assets");
        await mkdir(assets);
        await runner("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", WINDOWS_ASSETS],
            { env: { ...env, TODO_PACKAGE_ICON: join(repo, "crates/todo-desktop/assets/icon.png"), TODO_PACKAGE_ASSETS: assets } });
        let publisher = env.TODO_MSIX_PUBLISHER?.trim();
        if (!publisher && signing) {
            try { publisher = await runner("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", WINDOWS_PUBLISHER], { env, capture: true }); }
            catch (error) { throw redact(error, signing.password); }
            if (!publisher) throw new Error("The Windows signing certificate subject is unavailable; set TODO_MSIX_PUBLISHER explicitly");
        }
        await writeFile(join(stage, "AppxManifest.xml"), msixManifest(release, arch, publisher || "CN=FerroUI"));
        const path = join(out, `${name}.msix`);
        const makeappx = await windowsSdkTool("makeappx", env);
        await runner(makeappx, ["pack", "/o", "/h", "SHA256", "/d", stage, "/p", path], { env });
        const signed = await sign(path);
        artifacts.push({ path, signed, applicationSigned, publisher: publisher || "CN=FerroUI" });
    }
    if (requested.has("installer")) {
        const path = join(out, `${name}-setup.exe`);
        const script = join(work, "installer.nsi");
        // NSIS otherwise decodes BOM-less source with the Windows ANSI code page.
        await writeFile(script, "\ufeff" + windowsInstaller({ release, source: bundle.app, output: path }));
        let makensis = env.TODO_MAKENSIS;
        if (!makensis) {
            const installed = join(env["ProgramFiles(x86)"] || "C:\\Program Files (x86)", "NSIS", "makensis.exe");
            makensis = await exists(installed) ? installed : "makensis.exe";
        }
        await runner(makensis, ["/V3", script], { env });
        const signed = await sign(path);
        artifacts.push({ path, signed, applicationSigned, uninstallerSigned: false });
    }
    return artifacts;
}

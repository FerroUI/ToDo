# Native packages and release verification

The repository packages the native FerroUI desktop executable with the Node.js
runtime needed by its isolated account, synchronization, attachment, and archive
worker. Local task editing does not require a Microsoft account. The packaged
worker and its vendor modules do not require a separate npm installation.

The packaging scripts produce Linux archives and Debian packages, macOS
application bundles and disk images, and Windows portable archives, MSIX packages,
and per-user installers. They run on the target operating system and architecture
so they can inspect the actual runtime and invoke the platform's packaging tools.
The build workflow produces unsigned verification artifacts. A successful build
does not establish installation, notification delivery, account sign-in, or
signing acceptance on a user's desktop.

## Build and package

Check out the repository and its pinned FerroUI submodule. Install the Rust
toolchain from `rust-toolchain.toml`, Node.js 24 for the release scripts, and the
native build dependencies described in [NATIVE-MICROSOFT.md](NATIVE-MICROSOFT.md).
Use the toolchain in the repository rather than silently updating the framework.

```bash
cargo test --locked -p todo-desktop --lib
node --test scripts/native-*.test.mjs
cargo build --locked --profile dist-unwind -p todo-desktop
node scripts/fetch-native-runtime.mjs --out target/native/node-runtime
node scripts/package-native.mjs --runtime target/native/node-runtime --out target/native/packages
```

The runtime output directory must not already exist. This prevents a failed
download from replacing a previously verified runtime. To reproduce a specific
release, select its exact Node version explicitly with `--version X.Y.Z` on the
fetch command and retain `runtime.json` and `SHASUMS256.txt`. Without that option,
the downloader uses the exact version of the Node process running the script.
The CI workflow selects the Node 24 release line; the resulting artifact records
the concrete patch version and hashes, rather than claiming that the floating CI
selection is a permanent release pin.

The package command defaults to the workspace version, the `dist-unwind` profile,
`target/dist-unwind/todo-desktop` (or `.exe`), and all formats for the current platform.
Native distribution uses unwinding so a transient synchronous OS data-transfer
failure can be caught at the materialization boundary before task mutation. The
browser retains its own build profile. An explicit `dist` or `release` native
build uses aborting panics and cannot provide that native error containment.
Supported overrides are:

| Option | Meaning |
| --- | --- |
| `--version X.Y.Z` | Numeric package version; MSIX and Windows version resources append `.0`. |
| `--profile NAME` | `dist-unwind`, `release-unwind`, `dist`, `release`, or `debug`; the report records this choice. |
| `--binary PATH` | Explicit desktop executable. |
| `--runtime PATH` | Directory created by the runtime downloader. |
| `--out PATH` | Artifact and report directory. |
| `--formats LIST` | Comma-separated formats from the table below. |
| `--repo PATH` | Repository containing the source resources and licenses. |
| `--cargo-metadata PATH` | Previously captured locked Cargo metadata for the build host. |
| `--platform linux\|darwin\|win32`, `--arch x64\|arm64` | Explicit target validation; these must match the packaging host. |

| Platform | Formats | Output and installation layout |
| --- | --- | --- |
| Linux | `tar,deb` | Portable `.tar.gz`; Debian package with `/opt/ferroui-todo`, `/usr/bin/ferroui-todo`, and a desktop entry. |
| macOS | `app,dmg` | `To Do.app` inside a ZIP; `.dmg` containing the application and an Applications shortcut. |
| Windows | `zip,msix,installer` | Portable ZIP; MSIX; NSIS installer under the current user's `%LOCALAPPDATA%\Programs\FerroUI ToDo`. |

The application files are accompanied by a JSON artifact report and a `.sha256`
file. Each artifact entry records its hash, size, and completed signing operations.
`native-package-manifest.json` inside the package records the source revision when
CI supplies it, runtime provenance, the executable hash before signing, and any
Rust dependencies for which the source package supplied no separate notice file.
Review that list as part of distribution preparation. The package includes the
application and FerroUI notices, font/vendor licenses, the locked dependency
graph, and the available license/notice files and Cargo metadata for transitive
Rust dependencies.

### Runtime verification and trust

`fetch-native-runtime.mjs` downloads a specific official Node archive and
`SHASUMS256.txt` from `https://nodejs.org/dist/vX.Y.Z/`. It requires HTTPS, rejects
redirects, caps response sizes and time, requires exactly one matching checksum
entry, and verifies the archive before extraction. It retains only the runtime,
its license, the checksum list, and `runtime.json`; it does not ship npm or a
second dependency installation.

TLS plus a checksum from the same origin provides download integrity under that
origin's trust. It is not independent release-signature verification. For a
release using an already trusted Node release-key keyring, use:

```bash
node scripts/fetch-native-runtime.mjs --out target/native/node-runtime --version X.Y.Z --keyring /secure/node-release-keys.kbx
```

`TODO_NODE_RELEASE_KEYRING` supplies the same option. The script then verifies
`SHASUMS256.txt.asc` with `gpgv` and records `pgpVerified: true` only after success.
Provision and authenticate the keyring separately using the
[Node release-key guidance](https://github.com/nodejs/node#verifying-binaries);
the downloader does not promote an unverified downloaded key into a trust root.

Before staging, the packager rechecks the checksum manifest, runtime binary hash,
target architecture, and Node's reported version/platform/architecture. It clears
`NODE_OPTIONS` and `NODE_PATH` while executing this verification. These checks
detect inconsistency and unintended modification; unsigned metadata inside a
locally writable staging directory is not a substitute for package signing.

At runtime, `TODO_NODE_BINARY` is an explicit developer/operator override.
Otherwise the desktop looks beside its own executable for `runtime/node[.exe]`,
then in `Contents/Resources/runtime/node` for an application bundle, and finally
uses `node` from `PATH` for a developer installation. It does not search the
current working directory. The native worker requires Node 22 or later; release
CI currently bundles Node 24. Worker JavaScript and shared archive/sync modules
are carried in `service-assets`, alongside the embedded executable resources.

## Platform requirements

### Linux

Build tools include `clang`, `pkg-config`, fontconfig/freetype development
packages, `readelf` from binutils, `tar`, and `dpkg-deb`. The desktop runtime needs
the declared fontconfig, freetype, X11, EGL/GL, and GTK libraries. The Debian
control file lists those dependencies instead of assuming the bundled Node
runtime makes the GUI fully static.

The Debian package computes the maximum imported GLIBC version across the
desktop executable and Node, with a floor of 2.35 matching the Ubuntu 22.04 CI
baseline. It also uses `dpkg-shlibdeps` to derive versioned requirements from both
executables, including their actual C++ and GCC runtime symbol requirements, and
retains explicit dependencies for dynamically loaded GUI libraries. Build Debian
packages on a Debian/Ubuntu host with its package metadata available; missing
linked-library dependency information stops packaging. Building on a newer
distribution may increase the minimum supported library versions. The portable
archive has the same underlying binary/library requirements. A working desktop
session and notification service are required for interactive behavior.

### macOS

Use Xcode command-line tools, `sips`, `iconutil`, `ditto`, `hdiutil`, `otool`, and
the platform signing tools. The packager reads both Mach-O binaries' actual
minimum OS versions and writes the greater requirement into `Info.plist`, with
a minimum floor of macOS 13.5 for the Node 24 release configuration. It supports
Intel and ARM64 hosts, producing a matching architecture rather than claiming a
universal binary. CI currently builds ARM64.

The executable is in `To Do.app/Contents/MacOS`; its runtime, service assets,
license collection, icon, and package manifest are in `Contents/Resources`.
Move the application to its intended final location before enabling background
reminders, since the OS registration stores the executable path.

### Windows

Use the MSVC Rust toolchain, Visual Studio native build tools, LLVM/libclang, the
Windows SDK, and NSIS. CI builds with static MSVC runtime linkage:

```powershell
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --locked --profile dist-unwind -p todo-desktop
```

Before creating any Windows artifact, the packager reads both executables' PE
normal and delay-import tables. It rejects non-inbox Visual C++/MFC/OpenMP runtime
DLL dependencies, debug UCRT, and dynamically linked Clang ASan. This avoids
shipping a per-user installer that secretly depends on an administrator-installed
redistributable. Inbox UCRT/API-set dependencies are accepted. The package records
the inspected imports in `windows-runtime-imports.json`. Dependencies loaded
dynamically outside those import tables still require desktop acceptance testing.

MSIX declares a full-trust packaged desktop application and Windows 10 build
19041 or later. The NSIS installer uses current-user shortcuts and HKCU metadata;
it does not request elevation. Windows SDK tools are discovered from installed
SDK version directories, or can be selected with `TODO_MAKEAPPX` and
`TODO_SIGNTOOL`. `TODO_MAKENSIS` selects an explicit NSIS compiler.

## Signing hooks

Signing is opt-in. An unsigned CI artifact is reported as unsigned. The scripts
never infer a signing identity, create a trust certificate, log in to an account,
or submit a store release.

### Apple Developer ID and notarization

Set `TODO_APPLE_SIGNING_IDENTITY` to an available Developer ID application
identity. Ad-hoc `-` signing is rejected as a release identity. The script signs
the nested Node runtime with the hardened-runtime JIT entitlements it needs,
then signs the application, and verifies the result strictly. It does not use
recursive `--deep` signing to guess the bundle structure.

Optionally set `TODO_APPLE_NOTARY_PROFILE` to a previously provisioned
`notarytool` keychain profile. This requires a signing identity. The script
submits the application archive, requires an `Accepted` response, staples and
validates the application, then creates the distributable archive. A requested
DMG is also signed and, when configured, notarized and stapled. A failed or
rejected submission stops successful artifact reporting.

ZIP containers are never reported as cryptographically signed. Their report
instead distinguishes `applicationSigned` and `applicationNotarized` from the
container's own `signed` field. See Apple's
[notarization workflow](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
for provisioning and release acceptance.

### Windows Authenticode and MSIX

Supply one of the following credential sources:

| Environment variable | Meaning |
| --- | --- |
| `TODO_WINDOWS_CERT_THUMBPRINT` | Certificate in the current user's personal certificate store. |
| `TODO_WINDOWS_PFX` | PFX file used instead of a certificate-store thumbprint. |
| `TODO_WINDOWS_PFX_PASSWORD` | Optional password for that PFX; requires the PFX path. |
| `TODO_WINDOWS_TIMESTAMP_URL` | RFC 3161 timestamp endpoint; defaults to DigiCert's timestamp service. |
| `TODO_MSIX_PUBLISHER` | Explicit MSIX publisher subject; otherwise obtained from the selected certificate, or `CN=FerroUI` for unsigned packaging. |

Conflicting credential sources are rejected. The selected certificate's subject
must match the MSIX identity. Signing errors redact the supplied PFX password.
The desktop executable and requested installable containers are signed when
configured. A ZIP reports the application's signature separately. The NSIS
generated uninstaller is currently reported as `uninstallerSigned: false`, even
when the application and outer installer are signed; the report does not conceal
that separate binary's status. See Microsoft's
[SignTool documentation](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool)
and [MSIX signing guidance](https://learn.microsoft.com/en-us/windows/msix/package/sign-app-package-using-signtool).

## Background reminders, updates, and removal

Background reminders are off by default. Enabling the native Settings option
schedules the nearest pending reminder from the active persisted workspace for
the current OS user. Local saves reconcile the registration off the UI thread.
Account activation cancels/replaces the previous active workspace's schedule;
inactive account snapshots are not read or modified by the helper. Portable and
PST imports do not transfer this machine's opt-in permission.

| Platform | Registration |
| --- | --- |
| Linux | Per-user systemd service and persistent nearest-event timer. |
| macOS | Per-user LaunchAgent with a nearest calendar event and login catch-up. Unchanged loaded registrations are left alone; replacing a loaded schedule suppresses another immediate `RunAtLoad` invocation. |
| Windows | Per-user Task Scheduler entry with an interactive-user token, nearest time trigger, login trigger, and `StartWhenAvailable`. |

These jobs require the user's OS session and the corresponding scheduler and
notification service. They do not fetch cloud changes while the app is closed,
process every inactive account, wake a powered-off computer, or guarantee desktop
display while notifications are suppressed. Successful OS acceptance is followed
by durable reminder acknowledgement. A crash between acceptance and persistence
may repeat delivery; a failed notification or failed save does not mark it as
delivered. The GUI owns the data-directory lock while running, so a scheduled
helper cannot race its task mutations.

The executable recognizes the following modes before initializing the GUI:

```text
todo-desktop --deliver-reminders [--data-dir PATH]
todo-desktop --reconcile-reminders [--data-dir PATH]
todo-desktop --disable-background-reminders [--data-dir PATH]
```

The first two are scheduler maintenance entrypoints. They use strict persisted
document loading and never replace corrupt data with an empty document. The
disable command persists the opt-out and removes the current user's registration;
it fails with an instruction to close the application if the GUI owns the lock.
Scheduler and delivery failures are shown by native Settings, including recorded
headless failures from the previous run.

Before moving/removing a portable bundle, Debian installation, macOS application,
or MSIX package, close To Do and run `--disable-background-reminders` from the
installed executable. Pass each custom `--data-dir` that has a registration.
The Windows per-user NSIS uninstaller runs this command for the default data
directory and stops removal if cleanup fails. It does not discover every custom
data directory. Packages preserve task data on uninstall. After installing or
moving an update, open that installation to reconcile its executable path, and
enable background reminders again if they were disabled for removal.

## CI and acceptance scope

`.github/workflows/native-release.yml` supports manual dispatch and path-scoped
push verification of the native release tooling. The current matrix is:

| Runner | Artifact architecture | Main build constraints |
| --- | --- | --- |
| `ubuntu-22.04` | Linux x64 | GLIBC baseline and Debian tooling. |
| `macos-15` | macOS ARM64 | Deployment target 13.5, Mach-O inspection. |
| `windows-2022` | Windows x64 | Static CRT, LLVM, Windows SDK, NSIS. |

Each job runs native helper and JavaScript service/packaging tests, compiles the
actual desktop binary, downloads and verifies the matching runtime, packages all
formats, and uploads artifacts plus hash/signing reports. The workflow has read
access to repository contents and does not publish a GitHub Release. Signing
credentials are deliberately not configured by the unsigned validation workflow.

The pure tests cover reminder planning, ownership locks, accepted-versus-failed
delivery persistence, shutdown coalescing, service-format escaping, macOS
catch-up/rearm behavior, bounded child-process output, runtime hashes, resource
staging, manifests, platform dependency inspection, and signing command ordering.
Packaging tests use fixtures and recorded command runners where a real OS tool
is unavailable. They do not install a scheduler, send desktop notifications,
import signing credentials, or prove that an installer has run successfully.
Release acceptance must exercise launch, upgrade/uninstall, native clipboard and
drag/drop, notification permission/suppression, closed-process reminder delivery,
and optional account sign-in on each supported desktop platform. Signed release
acceptance additionally requires actual provisioned certificates and Apple
notarization credentials.

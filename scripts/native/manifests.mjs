import { architecture, version } from "./common.mjs";

function text(value) {
    value = String(value);
    if (/[\u0000-\u001f\u007f]/.test(value)) throw new Error("Package metadata must not contain control characters");
    return value;
}
export function xml(value) { return text(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&apos;"); }
export function nsis(value) { return text(value).replaceAll("$", () => "$$").replaceAll('"', '$\\"'); }

export function macInfoPlist(release, minimumMacOS = "13.5") {
    release = version(release);
    if (!/^\d+\.\d+(?:\.\d+)?$/.test(minimumMacOS)) throw new Error("Invalid minimum macOS version");
    return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>To Do</string><key>CFBundleDisplayName</key><string>To Do</string>
<key>CFBundleIdentifier</key><string>com.ferroui.todo</string><key>CFBundleExecutable</key><string>todo-desktop</string>
<key>CFBundlePackageType</key><string>APPL</string><key>CFBundleIconFile</key><string>ToDo</string>
<key>CFBundleShortVersionString</key><string>${release}</string><key>CFBundleVersion</key><string>${release}</string>
<key>LSMinimumSystemVersion</key><string>${minimumMacOS}</string><key>NSHighResolutionCapable</key><true/>
<key>NSHumanReadableCopyright</key><string>FerroUI contributors. MIT licensed.</string>
</dict></plist>\n`;
}

export const nodeEntitlements = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>com.apple.security.cs.allow-jit</key><true/>
<key>com.apple.security.cs.allow-unsigned-executable-memory</key><true/></dict></plist>\n`;

export function msixManifest(release, arch, publisher = "CN=FerroUI") {
    release = `${version(release)}.0`;
    arch = architecture(arch);
    return `<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
 xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
 xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10"
 xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
 IgnorableNamespaces="uap uap10 rescap">
 <Identity Name="FerroUI.ToDo" Publisher="${xml(publisher)}" Version="${release}" ProcessorArchitecture="${arch}"/>
 <Properties><DisplayName>To Do</DisplayName><PublisherDisplayName>FerroUI</PublisherDisplayName><Logo>Assets\\StoreLogo.png</Logo></Properties>
 <Resources><Resource Language="en-us"/></Resources>
 <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0"/></Dependencies>
 <Applications><Application Id="ToDo" Executable="todo-desktop.exe" uap10:RuntimeBehavior="packagedClassicApp" uap10:TrustLevel="mediumIL">
  <uap:VisualElements DisplayName="To Do" Description="Tasks, lists, reminders and optional Microsoft synchronization"
   Square150x150Logo="Assets\\Square150x150Logo.png" Square44x44Logo="Assets\\Square44x44Logo.png" BackgroundColor="transparent"/>
 </Application></Applications>
 <Capabilities><rescap:Capability Name="runFullTrust"/></Capabilities>
</Package>\n`;
}

export function desktopEntry() {
    return `[Desktop Entry]\nType=Application\nName=To Do\nComment=Tasks, lists and reminders\nExec=/opt/ferroui-todo/todo-desktop\nIcon=ferroui-todo\nTerminal=false\nCategories=Office;ProjectManagement;\nStartupNotify=true\n`;
}

export function debControl(release, arch, installedKiB, minimumGlibc = "2.35") {
    if (!Number.isSafeInteger(installedKiB) || installedKiB < 0) throw new Error("Invalid installed package size");
    if (!/^\d+\.\d+(?:\.\d+)?$/.test(minimumGlibc)) throw new Error("Invalid minimum glibc version");
    return `Package: ferroui-todo\nVersion: ${version(release)}\nArchitecture: ${architecture(arch) === "x64" ? "amd64" : "arm64"}\nMaintainer: FerroUI contributors <noreply@ferroui.com>\nSection: utils\nPriority: optional\nInstalled-Size: ${installedKiB}\nDepends: libc6 (>= ${minimumGlibc}), libstdc++6, libgcc-s1, libfontconfig1, libfreetype6, libx11-6, libxi6, libxrandr2, libxcursor1, libxrender1, libxext6, libegl1, libgl1, libgtk-3-0\nRecommends: dbus-user-session, xdg-utils\nHomepage: https://github.com/FerroUI/ToDo\nDescription: Native FerroUI task manager\n Tasks, lists, reminders, portable archives and optional Microsoft integration.\n Includes the Node.js runtime used by the isolated native service worker.\n`;
}

export function windowsInstaller({ release, source, output }) {
    release = version(release);
    return String.raw`Unicode True
!include "MUI2.nsh"
Name "To Do"
OutFile "${nsis(output)}"
InstallDir "$LOCALAPPDATA\Programs\FerroUI ToDo"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ManifestSupportedOS Win10
VIProductVersion "${release}.0"
VIAddVersionKey "ProductName" "To Do"
VIAddVersionKey "FileDescription" "FerroUI To Do per-user installer"
VIAddVersionKey "FileVersion" "${release}"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${nsis(source)}\licenses\application\LICENSE"
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Section "To Do"
 SetShellVarContext current
 SetOutPath "$INSTDIR"
 File /r "${nsis(source)}\*"
 WriteUninstaller "$INSTDIR\Uninstall.exe"
 CreateShortcut "$SMPROGRAMS\To Do.lnk" "$INSTDIR\todo-desktop.exe"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\FerroUIToDo" "DisplayName" "To Do"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\FerroUIToDo" "DisplayVersion" "${release}"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\FerroUIToDo" "Publisher" "FerroUI"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\FerroUIToDo" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\FerroUIToDo" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
 WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\FerroUIToDo" "NoModify" 1
 WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\FerroUIToDo" "NoRepair" 1
SectionEnd
Section "Uninstall"
 SetShellVarContext current
 ClearErrors
 ExecWait '$\"$INSTDIR\todo-desktop.exe$\" --disable-background-reminders' $0
 IfErrors blocked
 IntCmp $0 0 remove blocked blocked
 blocked:
  MessageBox MB_ICONEXCLAMATION|MB_OK "Close To Do, then run uninstall again. Background reminders could not be removed safely." /SD IDOK
  SetErrorLevel 2
  Abort
 remove:
  Delete "$SMPROGRAMS\To Do.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\FerroUIToDo"
  DeleteRegKey HKCU "Software\Classes\AppUserModelId\FerroUI.ToDo"
  RMDir /r "$INSTDIR"
SectionEnd
`;
}

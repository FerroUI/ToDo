# Notices

## Application

The application code in this repository, including its task domain, view models, declarative views, control templates, icon paths, and landscape geometry, is provided under the [MIT license](LICENSE).

Microsoft To Do is the visual and behavioral reference for this independent implementation. Microsoft To Do, Microsoft, Windows, WinUI, Microsoft Graph, and Microsoft Entra are names or marks of their respective owners. This project is not Microsoft's application and is not endorsed by Microsoft. It does not include Microsoft's proprietary To Do application source, logos, screenshot assets, wallpaper library, or completion sound recording. The application uses original vector artwork and a synthesized completion sound.

## FerroUI

The `FerroUI/` Git submodule is distributed under its [MIT license](FerroUI/LICENSE). Its [NOTICE.md](FerroUI/NOTICE.md) records the origins and licenses of framework components, including Avalonia-derived code, WinUI theme resources, ReactiveUI, DynamicData, Rx, rendering, text, and platform dependencies. Preserve those notices and any component-specific license files when redistributing the framework or an application that includes it.

FerroUI's WinUI theme is based on the MIT-licensed Microsoft WinUI sources; see [its component notice](FerroUI/src/FerroUI.Themes.WinUI/NOTICE.md). The bundled Inter font is covered by [its component notice](FerroUI/src/FerroUI.Fonts.Inter/NOTICE.md). Fonts and renderer dependencies retain their upstream licenses. This application's MIT license does not replace any dependency license.

## Noto Color Emoji

`crates/todo-controls/assets/fonts/Noto-COLRv1.ttf` is the complete Noto Color Emoji
v2.057 vector font from [googlefonts/noto-emoji](https://github.com/googlefonts/noto-emoji),
pinned to commit `99701e07d3d1fe1bdbcdb14964547f0fd592966f`. It is distributed under
the SIL Open Font License 1.1, included in
[OFL.txt](crates/todo-controls/assets/fonts/OFL.txt). Its source and integrity are
recorded in the [font provenance](crates/todo-controls/assets/fonts/README.md).
Font copyright: Copyright 2022 Google Inc.
The browser distributes the font and license as separate offline-capable assets;
the desktop application embeds the same font. The application's MIT license does
not replace the font license.

## WenQuanYi Micro Hei

`crates/todo-controls/assets/fonts/WenQuanYiMicroHei.ttf` is the complete
proportional face from the official
[WenQuanYi Micro Hei 0.2.0-beta release](https://sourceforge.net/projects/wqy/files/wqy-microhei/0.2.0-beta/),
distributed under its **Apache License 2.0** option. The extraction preserves
all glyphs and character maps; it only repacks the first face of the source TTC
as a standalone TrueType font. The [font provenance](crates/todo-controls/assets/fonts/README.md)
records the original archive and derived-font hashes and reproducible recipe.

Digitized data © 2007 Google Corporation; © 2008–2009 WenQuanYi Project Board
of Trustees. Preserve the upstream
[Apache license](crates/todo-controls/assets/fonts/WenQuanYiMicroHei-LICENSE.txt),
[authors and contributor credits](crates/todo-controls/assets/fonts/WenQuanYiMicroHei-AUTHORS.txt),
and [release notes](crates/todo-controls/assets/fonts/WenQuanYiMicroHei-README.txt),
which include attribution for Ume-Font punctuation. These files accompany the
separate browser asset and native packages. The application's MIT license does
not replace the font license.

## Microsoft Authentication Library

`crates/todo-browser/wwwroot/vendor/msal-browser-5.30.0.min.js` is the bundled Microsoft Authentication Library for JavaScript browser package. It is used only for optional Microsoft account authentication.

- Project: [AzureAD/microsoft-authentication-library-for-js](https://github.com/AzureAD/microsoft-authentication-library-for-js)
- Copyright: Microsoft Corporation
- License: MIT; included in [MSAL-LICENSE.txt](crates/todo-browser/wwwroot/vendor/MSAL-LICENSE.txt)

## Unicode CLDR time-zone data

`crates/todo-browser/wwwroot/cloud/windows-zones.js` contains the Windows-to-IANA primary time-zone mapping derived from [Unicode CLDR windowsZones.xml](https://github.com/unicode-org/cldr/blob/main/common/supplemental/windowsZones.xml). The mapping lets the Graph adapter interpret Microsoft reminder time zones.

- Copyright: Unicode, Inc.
- License: Unicode License v3; included in [UNICODE-LICENSE.txt](crates/todo-browser/wwwroot/vendor/UNICODE-LICENSE.txt)

## Rust dependencies

`Cargo.lock` identifies the resolved dependency versions. Each dependency retains its package license and notices. The browser's Emscripten, Skia, HarfBuzz, font, and JavaScript runtime dependencies are also covered by the notices supplied with their upstream distributions and the FerroUI submodule.

## Optional Web Push reminder service

`services/reminders/package-lock.json` pins `web-push` **4.0.0** and its resolved
dependencies for the separately deployed reminder service. `web-push` supplies
the Web Push encryption and VAPID implementation; its upstream code is not
modified by this application.

- Project: [web-push-libs/web-push](https://github.com/web-push-libs/web-push)
- Package: [web-push 4.0.0](https://www.npmjs.com/package/web-push/v/4.0.0)
- License: Mozilla Public License 2.0, included in
  [WEB-PUSH-LICENSE.txt](services/reminders/WEB-PUSH-LICENSE.txt)

The service's npm installation and container preserve dependency license files.
Its transitive packages retain their own licenses as recorded in the lockfile
and installed packages. The application's MIT license does not replace those
licenses. The static browser app does not bundle this server dependency or its
private VAPID key.

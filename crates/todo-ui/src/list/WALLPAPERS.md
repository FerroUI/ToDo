# Original wallpapers

`wallpaper.rs` contains six original geometric scenes: Mountains, Coast, Forest,
Dunes, Lake, and Night sky. Their paths and colors are authored in this source;
no Microsoft artwork, external photographs, network images, or generated bitmap
assets are bundled. Geometry uses a 1200 × 800 coordinate space. A native
`Viewbox` applies uniform fill, clips the crop, and a theme-dependent overlay keeps
the white heading readable. Wallpaper layers do not participate in hit testing.

To add a scene, choose a stable `scene:` identifier, add it to the domain
`backgrounds::SCENES` list, and add a `Scene` here with an accessible short name,
sky color, and closed native `Geometry` paths. Keep the top third quiet for the
heading. The native geometry test verifies every path and the shared identifiers.
The gallery uses the same rendering code as the full list. Do not substitute
remote image URLs or licensed artwork without documenting its source/license.

## Custom images

The native file picker accepts JPEG and PNG. Before decoding, the app requires a
known file size of at most 8 MiB and inspects the file header to reject dimensions
over 8192 pixels on either side or 12 million decoded pixels. The native bitmap
decoder then resizes the longest side to at most 1200 pixels. JPEG encoding tries
bounded quality/scale levels until its output fits 192 KiB. The resulting
`data:image/jpeg;base64,` value is self-contained; neither paths nor filesystem
permissions become part of the document. Malformed or unrenderable legacy values
fall back to the normal accent-colored heading and background.

List backgrounds live in the existing `TaskList.background` field. Smart-list
backgrounds use `Settings.smart_backgrounds`; `none` explicitly disables My Day's
default scene. Changing a color preserves its wallpaper. Both forms support undo,
JSON/portable backups, independent account workspaces, and app-owned OneDrive
device-settings synchronization. Outlook PST does not carry list presentation.

Encoded backgrounds share a 2 MiB aggregate document budget. This is checked
before changing a list, changing settings, duplicating a list, or accepting a
restored document. It leaves room within the existing 4 MiB OneDrive replica and
32 MiB document limits. Each native image cache retains at most three images and
keys by the complete image value, so colliding list IDs in different accounts
cannot reuse the wrong wallpaper.

File selection is asynchronous. Its result is applied only while the original
view and monotonically increasing view-model revision still match. An intervening
account change, navigation, or document update requires choosing again. On the
browser the official framework file-dialog polyfill uses a transient native file
input, avoiding persistent File System Access grants for imported images.

import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { deflateSync } from "node:zlib";
import { button, named } from "./touch-input.mjs";

// A small, original RGB gradient fixture: no external image or test-only app API.
function png(width, height) {
  const crc = bytes => {
    let value = 0xffffffff;
    for (const byte of bytes) { value ^= byte; for (let bit = 0; bit < 8; bit++) value = value >>> 1 ^ (value & 1 ? 0xedb88320 : 0); }
    return (value ^ 0xffffffff) >>> 0;
  };
  const chunk = (name, data) => {
    const result = Buffer.alloc(data.length + 12);
    result.writeUInt32BE(data.length); result.write(name, 4, "ascii"); data.copy(result, 8);
    result.writeUInt32BE(crc(result.subarray(4, -4)), result.length - 4); return result;
  };
  const header = Buffer.alloc(13); header.writeUInt32BE(width); header.writeUInt32BE(height, 4); header[8] = 8; header[9] = 2;
  const stride = width * 3 + 1, rows = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const offset = y * stride + x * 3 + 1;
    rows[offset] = 40 + Math.floor(80 * x / width); rows[offset + 1] = 70 + Math.floor(90 * y / height); rows[offset + 2] = 155;
  }
  return Buffer.concat([Buffer.from("89504e470d0a1a0a", "hex"), chunk("IHDR", header), chunk("IDAT", deflateSync(rows)), chunk("IEND", Buffer.alloc(0))]);
}

export async function verifyWallpapers(ui, output) {
  const { page, saved, snapshot, resize, navigate, control, clickControl, edit, until, capture } = ui;
  const directory = path.resolve(output, "wallpaper-fixtures"); await mkdir(directory, { recursive: true });
  const original = path.join(directory, "original-gradient.png"), oversized = path.join(directory, "oversized-header.png");
  await writeFile(original, png(1600, 900)); await writeFile(oversized, png(8193, 4));
  let chooser, listening = true, failure;
  page.on("Page.fileChooserOpened", event => { if (listening) chooser = event; });
  const command = async (method, parameters) => {
    const result = await page.send(method, parameters); assert(!result.error, `${method}: ${result.error?.message}`); return result;
  };
  const visible = async (match, label) => {
    for (let attempt = 0; attempt < 12; attempt++) {
      const view = await snapshot(), target = view.elements.find(match);
      if (target?.hit && target.y > 80 && target.y + target.height < view.height - 60) return target;
      await page.wheel(view.width / 2, view.height / 2 + 100, 0, target?.y < 80 ? -180 : 180);
    }
    return control(match, label);
  };
  const open = async () => {
    await clickControl(button("List options"), "wallpaper list options");
    await clickControl(element => element.type.endsWith("MenuItem") && element.automationName === "Change theme", "wallpaper picker");
  };
  const choose = async file => {
    await visible(named("ChooseBackgroundImage"), "custom image picker"); chooser = null;
    await clickControl(named("ChooseBackgroundImage"), "choose a custom image");
    await until(() => chooser, "native storage provider opens a browser file chooser");
    assert(chooser.backendNodeId, "the framework picker uses a transient file input");
    await command("DOM.setFileInputFiles", { backendNodeId: chooser.backendNodeId, files: [file] });
  };
  try {
    await command("Page.setInterceptFileChooserDialog", { enabled: true });
    await resize(1280, 900);
    await clickControl(button("New list"), "wallpaper fixture list"); await edit("ListName", "Wallpaper review", true);
    const list = (await saved()).lists.find(item => item.name === "Wallpaper review"); assert(list);
    const current = async () => (await saved()).lists.find(item => item.id === list.id);
    await open(); await visible(named("WallpaperForest"), "original Forest scene");
    await clickControl(named("WallpaperForest"), "choose Forest scene");
    await until(async () => (await current()).background === "scene:forest", "original scene is durable");
    assert((await snapshot()).elements.some(named("ListWallpaper")), "native wallpaper is mounted");
    await edit("NewTask", "Tasks remain interactive over a wallpaper", true);
    await open(); await clickControl(button("Blue list theme"), "change accent while retaining the scene");
    assert.equal((await current()).background, "scene:forest");
    await capture("wallpaper-original-forest");
    await open(); await choose(original);
    const custom = await until(async () => { const value = (await current()).background; return value?.startsWith("data:image/jpeg;base64,") && value; }, "native decoding and JPEG compression save a portable image", 20_000);
    const bytes = Buffer.from(custom.split(",")[1], "base64");
    assert(bytes.length <= 192 * 1024); assert.equal(bytes.readUInt16BE(0), 0xffd8);
    await control(named("NewTask"), "custom picker closes after saving");
    await resize(390, 844); await capture("wallpaper-custom-narrow");
    await resize(1280, 900); await navigate("Page.reload", { ignoreCache: true });
    await clickControl(button("Wallpaper review"), "restored wallpaper list");
    assert.equal((await current()).background, custom, "custom image survives reload without a file handle or grant");
    assert((await snapshot()).elements.some(named("ListWallpaper")));
    await open(); await choose(oversized);
    await until(async () => (await snapshot()).elements.some(element => element.text?.includes("8192 pixels")), "oversized dimensions are rejected before decoding");
    assert.equal((await current()).background, custom, "a rejected image preserves the chosen wallpaper");
    await visible(named("ClearBackground"), "remove wallpaper"); await clickControl(named("ClearBackground"), "remove wallpaper");
    await until(async () => (await current()).background === "none", "explicitly removing a background persists");
    assert(!(await snapshot()).elements.some(named("ListWallpaper")), "removal restores the ordinary native list surface");
    await clickControl(button("List options"), "remove wallpaper fixture options");
    await clickControl(element => element.type.endsWith("MenuItem") && element.automationName === "Delete list", "delete wallpaper fixture");
    await clickControl(button("Delete"), "confirm wallpaper fixture cleanup");
    await until(async () => !(await saved()).lists.some(item => item.id === list.id), "wallpaper fixture removed");
    if ((await snapshot()).elements.some(element => element.hit && button("Dismiss message")(element))) await clickControl(button("Dismiss message"), "dismiss wallpaper status");
    await resize(1280, 800);
    console.log("ok: original native wallpaper selection, independent accent, real custom PNG picker/JPEG conversion, narrow layout, reload, oversized header rejection and removal");
  } catch (error) { failure = error; throw error; }
  finally {
    listening = false;
    try { await command("Page.setInterceptFileChooserDialog", { enabled: false }); }
    catch (error) { if (!failure) throw error; }
  }
}

import assert from "node:assert/strict";

// Physical keys retain shortcut/keydown coverage. Unicode uses the browser's normal
// emoji/IME insertion route; supplementary characters are not physical virtual keys.
// https://chromedevtools.github.io/devtools-protocol/tot/Input/#method-insertText
export async function typeText(page, text) {
  if (/[^\u0000-\u007f]/u.test(text)) {
    const result = await page.send("Input.insertText", { text });
    assert(!result.error, result.error?.message);
  } else {
    await page.type(text);
  }
}

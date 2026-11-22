import { parse } from "./vendor/parse5-8.0.0.mjs";

// The shared Graph codec needs only these inert DOM operations. parse5 supplies
// HTML5 tree construction and entity decoding; it never loads URLs or executes
// scripts. A regex replacement would corrupt malformed/nested Outlook HTML.
export class NativeDOMParser {
  parseFromString(source, type) {
    if (type !== "text/html") throw new Error("Native task notes support HTML parsing only.");
    const document = parse(String(source));
    const wrap = node => ({
      get textContent() {
        const text = value => value.nodeName === "#text" ? value.value : (value.childNodes || []).map(text).join("");
        return text(node);
      },
      append(value) { (node.childNodes ||= []).push({ nodeName: "#text", value: String(value), parentNode: node }); },
      replaceWith(value) {
        const parent = node.parentNode;
        if (parent) parent.childNodes.splice(parent.childNodes.indexOf(node), 1, { nodeName: "#text", value: String(value), parentNode: parent });
      },
    });
    const all = [];
    const visit = node => { all.push(node); for (const child of node.childNodes || []) visit(child); };
    visit(document);
    return {
      body: wrap(all.find(node => node.tagName === "body") || document),
      querySelectorAll(selector) {
        const names = selector.split(",").map(name => name.trim());
        if (names.some(name => !/^[a-z]+$/.test(name))) throw new Error("Native HTML conversion does not support this selector.");
        return all.filter(node => names.includes(node.tagName)).map(wrap);
      },
    };
  }
}

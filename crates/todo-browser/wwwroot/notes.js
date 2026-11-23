// HTML is parsed into an allowlisted note model, never inserted into a DOM.
// Keep this canonical representation aligned with todo-domain::notes.
const MAX_NOTE_BYTES = 1024 * 1024;
export const escapeNote = value => String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;")
  .replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
export function safeNoteLink(value) {
  value = String(value || "").trim();
  if (!/^https?:\/\//i.test(value) || /[\s\u0000-\u001f\u007f-\u009f\\<>"]/.test(value)) return null;
  const host = value.replace(/^https?:\/\//i, "").split(/[/?#]/, 1)[0];
  return host && !host.includes("@") ? value : null;
}
const entities = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: "\u00a0", bull: "•", ndash: "–", mdash: "—", hellip: "…", copy: "©", reg: "®", trade: "™", lsquo: "‘", rsquo: "’", ldquo: "“", rdquo: "”" };
function decode(value) {
  return value.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (match, name) => {
    if (entities[name]) return entities[name];
    if (!name.startsWith("#")) return match;
    const n = /^#x/i.test(name) ? Number.parseInt(name.slice(2), 16) : Number(name.slice(1));
    return n >= 0 && n <= 0x10ffff && !(n >= 0xd800 && n <= 0xdfff) ? String.fromCodePoint(n || 0xfffd) : match;
  });
}
const block = name => /^(p|div|li|ul|ol|pre|blockquote|h[1-6]|tr)$/.test(name);
const voidTag = name => /^(br|img|hr|meta|link|input|wbr|source|area|embed|base)$/.test(name);
function attrs(raw) {
  const result = {};
  for (const match of raw.matchAll(/([^\s=/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+)))?/g)) {
    const name = match[1].toLowerCase();
    if (!Object.hasOwn(result, name)) result[name] = decode(match[2] ?? match[3] ?? match[4] ?? "");
  }
  return result;
}
export function parseNoteHtml(html) {
  html = String(html || "");
  if (new TextEncoder().encode(html).length > MAX_NOTE_BYTES) throw new Error("A rich note exceeds 1 MiB.");
  const paragraphs = [], stack = [{ tag: "", run: {}, pre: false, blocked: false }], lists = [];
  let paragraph = null;
  const finish = () => { if (paragraph) paragraphs.push(paragraph); paragraph = null; };
  const push = (text, context) => {
    if (!text) return;
    paragraph ||= { list: null, runs: [] };
    const run = { ...context.run, text }, last = paragraph.runs.at(-1);
    if (last && ["bold", "italic", "underline", "strike", "link"].every(key => last[key] === run[key])) last.text += text;
    else paragraph.runs.push(run);
  };
  let position = 0;
  while (position < html.length) {
    if (html.startsWith("<!--", position)) {
      const end = html.indexOf("-->", position + 4); position = end < 0 ? html.length : end + 3; continue;
    }
    const context = stack.at(-1);
    if (html[position] !== "<") {
      const found = html.indexOf("<", position), end = found < 0 ? html.length : found;
      if (!context.blocked) {
        let text = decode(html.slice(position, end));
        if (context.pre) text = text.replace(/\r\n?/g, "\n");
        else {
          text = text.replace(/[ \r\n\t\f]+/g, " ");
          if (!paragraph?.runs.length || /\s$/.test(paragraph.runs.at(-1).text)) text = text.replace(/^ /, "");
          if (!paragraph && !text.trim()) text = "";
        }
        push(text, context);
      }
      position = end; continue;
    }
    let end = position + 1, quote = null;
    for (; end < html.length; end++) {
      const c = html[end];
      if (quote) { if (c === quote) quote = null; }
      else if (c === "'" || c === '"') quote = c;
      else if (c === ">") break;
    }
    if (end === html.length) { if (!context.blocked) push(decode(html.slice(position)), context); break; }
    const raw = html.slice(position + 1, end).trim(); position = end + 1;
    const match = /^\/?\s*([a-z0-9]+)/i.exec(raw); if (!match) continue;
    const tag = match[1].toLowerCase(), closing = raw.startsWith("/");
    if (closing) {
      const index = stack.findLastIndex(c => c.tag === tag); if (index >= 1) stack.length = index;
      if (context.blocked) continue;
      if (block(tag)) finish();
      if (tag === "ul" || tag === "ol") { const index = lists.findLastIndex(item => item.tag === tag); if (index >= 0) lists.length = index; }
      continue;
    }
    const attributes = attrs(raw.slice(match[0].length));
    const next = { ...context, tag, run: { ...context.run }, blocked: context.blocked || /^(script|style|head|iframe|object|svg|math|template|noscript)$/.test(tag) };
    if (!next.blocked) {
      if (block(tag)) {
        finish();
        if (tag !== "ul" && tag !== "ol") {
          let list = null;
          if (tag === "li" && lists.length) {
            const item = lists.at(-1);
            list = item.tag === "ul" ? 0 : positive(attributes.value, item.number);
            if (item.tag === "ol") item.number = Math.min(list + 1, 0xffffffff);
          }
          paragraph = { list, runs: [] };
        }
      }
      if (tag === "ul" || tag === "ol") {
        if (lists.length >= 64) throw new Error("A rich note has too many nested lists.");
        lists.push({ tag, number: positive(attributes.start, 1) });
      }
      if (tag === "br") push("\n", next);
      if (tag === "b" || tag === "strong") next.run.bold = true;
      if (tag === "i" || tag === "em") next.run.italic = true;
      if (tag === "u") next.run.underline = true;
      if (["s", "strike", "del"].includes(tag)) next.run.strike = true;
      if (tag === "a") next.run.link = safeNoteLink(attributes.href);
      if (tag === "pre") next.pre = true;
      for (const item of (attributes.style || "").split(";")) {
        const colon = item.indexOf(":"); if (colon < 0) continue;
        const name = item.slice(0, colon).trim().toLowerCase(), value = item.slice(colon + 1).trim().toLowerCase();
        if (name === "font-weight") next.run.bold = value === "bold" || /^\d+$/.test(value) && Number(value) >= 600;
        if (name === "font-style") next.run.italic = ["italic", "oblique"].includes(value);
        if (name === "text-decoration" || name === "text-decoration-line") {
          next.run.underline = value.split(/\s+/).includes("underline"); next.run.strike = value.split(/\s+/).includes("line-through");
        }
        if (name === "white-space") next.pre = ["pre", "pre-wrap", "break-spaces"].includes(value);
      }
    }
    if (!voidTag(tag) && !raw.endsWith("/")) {
      if (stack.length >= 128) throw new Error("A rich note has too many nested elements.");
      stack.push(next);
    }
  }
  finish(); return paragraphs.length ? paragraphs : [{ list: null, runs: [] }];
}
function positive(value, fallback) { return /^\d+$/.test(value) && Number(value) > 0 && Number(value) <= 0xffffffff ? Number(value) : fallback; }
export function noteText(paragraphs) {
  return paragraphs.map(p => (p.list === null ? "" : p.list === 0 ? "• " : `${p.list}. `) + p.runs.map(run => run.text).join("")).join("\n");
}
export function writeNoteHtml(paragraphs) {
  let html = "", list = null;
  for (const paragraph of paragraphs) {
    const kind = paragraph.list === null ? null : paragraph.list > 0;
    if (kind !== list) {
      if (list !== null) html += list ? "</ol>" : "</ul>";
      if (kind !== null) html += kind ? "<ol>" : "<ul>";
      list = kind;
    }
    html += paragraph.list === null ? '<p style="white-space:pre-wrap">' : paragraph.list === 0 ? '<li style="white-space:pre-wrap">' : `<li value="${paragraph.list}" style="white-space:pre-wrap">`;
    for (const run of paragraph.runs) {
      const tags = [run.bold && "strong", run.italic && "em", run.underline && "u", run.strike && "s"].filter(Boolean), link = safeNoteLink(run.link);
      html += tags.map(tag => `<${tag}>`).join("");
      if (link) html += `<a href="${escapeNote(link)}">`;
      html += escapeNote(run.text).replaceAll("\n", "<br>");
      if (link) html += "</a>";
      html += tags.reverse().map(tag => `</${tag}>`).join("");
    }
    html += paragraph.list === null ? "</p>" : "</li>";
  }
  if (list !== null) html += list ? "</ol>" : "</ul>";
  return html;
}
export function richNoteFields(html) {
  const paragraphs = parseNoteHtml(html), rich = paragraphs.some(p => p.list !== null || p.runs.some(r => r.bold || r.italic || r.underline || r.strike || r.link));
  return { notes: noteText(paragraphs), notes_html: rich ? writeNoteHtml(paragraphs) : null };
}
export function noteFields(task) {
  const notes = task.notes || "";
  if (task.notes_html) { const rich = richNoteFields(task.notes_html); if (rich.notes === notes) return rich; }
  return { notes, notes_html: null };
}
export function noteBody(task) {
  const value = noteFields(task);
  return { contentType: "html", content: value.notes_html || writeNoteHtml(value.notes.replace(/\r\n?/g, "\n").split("\n").map(text => ({ list: null, runs: [{ text }] }))) };
}

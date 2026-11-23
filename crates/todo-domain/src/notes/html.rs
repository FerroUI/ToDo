use super::{MAX_NOTE_BYTES, NoteParagraph, NoteRun, RichNote, escape_html, safe_link};

#[derive(Clone, Default)]
struct Context {
    tag: String,
    run: NoteRun,
    pre: bool,
    blocked: bool,
}

/// Tolerant bounded tokenizer. It never forwards source markup: the serializer
/// emits its own allowlisted elements and attributes from the resulting model.
pub(super) fn parse(source: &str) -> Result<RichNote, &'static str> {
    if source.len() > MAX_NOTE_BYTES {
        return Err("A rich note exceeds 1 MiB");
    }
    let mut note = RichNote::default();
    let mut paragraph: Option<NoteParagraph> = None;
    let mut stack = vec![Context::default()];
    let mut lists: Vec<(String, u32)> = Vec::new();
    let mut rest = source;
    while !rest.is_empty() {
        if rest.starts_with("<!--") {
            rest = rest.find("-->").map_or("", |end| &rest[end + 3..]);
            continue;
        }
        if !rest.starts_with('<') {
            let end = rest.find('<').unwrap_or(rest.len());
            let context = stack.last().unwrap();
            if !context.blocked {
                let text = decode(&rest[..end]);
                let text = if context.pre {
                    text.replace("\r\n", "\n").replace('\r', "\n")
                } else {
                    collapse(&text, paragraph.as_ref())
                };
                if !text.is_empty() {
                    let mut run = context.run.clone();
                    run.text = text;
                    paragraph
                        .get_or_insert_with(NoteParagraph::default)
                        .push(run);
                }
            }
            rest = &rest[end..];
            continue;
        }
        let Some(end) = tag_end(rest) else {
            if !stack.last().unwrap().blocked {
                let mut run = stack.last().unwrap().run.clone();
                run.text = decode(rest);
                paragraph
                    .get_or_insert_with(NoteParagraph::default)
                    .push(run);
            }
            break;
        };
        let raw = &rest[1..end];
        rest = &rest[end + 1..];
        let closing = raw.trim_start().starts_with('/');
        let raw = raw.trim().trim_start_matches('/').trim();
        let name_end = raw
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(raw.len());
        let tag = raw[..name_end].to_ascii_lowercase();
        if tag.is_empty() {
            continue;
        }
        let blocked_before = stack.last().unwrap().blocked;
        if closing {
            if let Some(index) = stack.iter().rposition(|c| c.tag == tag) {
                stack.truncate(index.max(1));
            }
            if blocked_before {
                continue;
            }
            if is_block(&tag) {
                finish(&mut note, &mut paragraph);
            }
            if tag == "ul" || tag == "ol" {
                if let Some(index) = lists.iter().rposition(|(name, _)| name == &tag) {
                    lists.truncate(index);
                }
            }
            continue;
        }
        let attrs = attributes(&raw[name_end..]);
        let attr = |key: &str| {
            attrs
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.as_str())
        };
        let mut context = stack.last().unwrap().clone();
        context.tag = tag.clone();
        context.blocked |= matches!(
            tag.as_str(),
            "script"
                | "style"
                | "head"
                | "iframe"
                | "object"
                | "svg"
                | "math"
                | "template"
                | "noscript"
        );
        if !context.blocked {
            if is_block(&tag) {
                finish(&mut note, &mut paragraph);
                if tag != "ul" && tag != "ol" {
                    let list = if tag == "li" {
                        lists.last_mut().map(|(kind, number)| {
                            if kind == "ul" {
                                0
                            } else {
                                let n = attr("value")
                                    .and_then(|v| v.parse().ok())
                                    .filter(|n| *n > 0)
                                    .unwrap_or(*number);
                                *number = n.saturating_add(1);
                                n
                            }
                        })
                    } else {
                        None
                    };
                    paragraph = Some(NoteParagraph {
                        list,
                        ..Default::default()
                    });
                }
            }
            if tag == "ul" || tag == "ol" {
                if lists.len() >= 64 {
                    return Err("A rich note has too many nested lists");
                }
                lists.push((
                    tag.clone(),
                    attr("start")
                        .and_then(|v| v.parse().ok())
                        .filter(|n| *n > 0)
                        .unwrap_or(1),
                ));
            }
            if tag == "br" {
                let mut run = context.run.clone();
                run.text = "\n".into();
                paragraph
                    .get_or_insert_with(NoteParagraph::default)
                    .push(run);
            }
            match tag.as_str() {
                "b" | "strong" => context.run.bold = true,
                "i" | "em" => context.run.italic = true,
                "u" => context.run.underline = true,
                "s" | "strike" | "del" => context.run.strike = true,
                "a" => context.run.link = attr("href").and_then(safe_link),
                "pre" => context.pre = true,
                _ => {}
            }
            if let Some(style) = attr("style") {
                for declaration in style.split(';') {
                    let Some((name, value)) = declaration.split_once(':') else {
                        continue;
                    };
                    let value = value.trim().to_ascii_lowercase();
                    match name.trim().to_ascii_lowercase().as_str() {
                        "font-weight" => {
                            context.run.bold =
                                value == "bold" || value.parse::<u16>().is_ok_and(|n| n >= 600)
                        }
                        "font-style" => {
                            context.run.italic = matches!(value.as_str(), "italic" | "oblique")
                        }
                        "text-decoration" | "text-decoration-line" => {
                            context.run.underline =
                                value.split_whitespace().any(|v| v == "underline");
                            context.run.strike =
                                value.split_whitespace().any(|v| v == "line-through");
                        }
                        "white-space" => {
                            context.pre =
                                matches!(value.as_str(), "pre" | "pre-wrap" | "break-spaces")
                        }
                        _ => {}
                    }
                }
            }
        }
        if !matches!(
            tag.as_str(),
            "br" | "img"
                | "hr"
                | "meta"
                | "link"
                | "input"
                | "wbr"
                | "source"
                | "area"
                | "embed"
                | "base"
        ) && !raw.ends_with('/')
        {
            if stack.len() >= 128 {
                return Err("A rich note has too many nested elements");
            }
            stack.push(context);
        }
    }
    finish(&mut note, &mut paragraph);
    if note.paragraphs.is_empty() {
        note.paragraphs.push(NoteParagraph::default());
    }
    Ok(note)
}

fn finish(note: &mut RichNote, paragraph: &mut Option<NoteParagraph>) {
    if let Some(paragraph) = paragraph.take() {
        note.paragraphs.push(paragraph);
    }
}
fn is_block(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "div"
            | "li"
            | "ul"
            | "ol"
            | "pre"
            | "blockquote"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "tr"
    )
}
fn tag_end(value: &str) -> Option<usize> {
    let mut quote = None;
    for (i, c) in value.char_indices().skip(1) {
        match (quote, c) {
            (Some(q), c) if q == c => quote = None,
            (None, '\'' | '"') => quote = Some(c),
            (None, '>') => return Some(i),
            _ => {}
        }
    }
    None
}
fn attributes(mut raw: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    while !raw.trim_start().is_empty() {
        raw = raw.trim_start();
        let end = raw
            .find(|c: char| c.is_whitespace() || matches!(c, '=' | '/' | '>'))
            .unwrap_or(raw.len());
        if end == 0 {
            raw = &raw[1..];
            continue;
        }
        let name = raw[..end].to_ascii_lowercase();
        raw = raw[end..].trim_start();
        let mut value = "";
        if let Some(next) = raw.strip_prefix('=') {
            raw = next.trim_start();
            if raw.starts_with(['\'', '"']) {
                let quote = raw.as_bytes()[0] as char;
                raw = &raw[1..];
                let end = raw.find(quote).unwrap_or(raw.len());
                value = &raw[..end];
                raw = &raw[(end + usize::from(end < raw.len()))..];
            } else {
                let end = raw.find(char::is_whitespace).unwrap_or(raw.len());
                value = &raw[..end];
                raw = &raw[end..];
            }
        }
        if result.len() < 128 {
            result.push((name, decode(value)));
        }
    }
    result
}
fn collapse(text: &str, paragraph: Option<&NoteParagraph>) -> String {
    let mut result = String::new();
    let mut space = paragraph
        .and_then(|p| p.runs.last())
        .is_none_or(|r| r.text.ends_with(char::is_whitespace));
    for c in text.chars() {
        if matches!(c, ' ' | '\r' | '\n' | '\t' | '\u{000c}') {
            if !space {
                result.push(' ');
                space = true;
            }
        } else {
            result.push(c);
            space = false;
        }
    }
    // Whitespace between block tags must not create a new blank paragraph.
    if paragraph.is_none() && result.trim().is_empty() {
        result.clear();
    }
    result
}
pub(super) fn decode(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        result.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        let Some(end) = rest.find(';').filter(|end| *end <= 32) else {
            result.push('&');
            continue;
        };
        let entity = &rest[..end];
        let value = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some('\u{00a0}'),
            "bull" => Some('•'),
            "ndash" => Some('–'),
            "mdash" => Some('—'),
            "hellip" => Some('…'),
            "copy" => Some('©'),
            "reg" => Some('®'),
            "trade" => Some('™'),
            "lsquo" => Some('‘'),
            "rsquo" => Some('’'),
            "ldquo" => Some('“'),
            "rdquo" => Some('”'),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|v| u32::from_str_radix(v, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|v| v.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(value) = value {
            result.push(if value == '\0' { '\u{fffd}' } else { value });
            rest = &rest[end + 1..];
        } else {
            result.push('&');
        }
    }
    result.push_str(rest);
    result
}

pub(super) fn write(note: &RichNote) -> String {
    let mut html = String::new();
    let mut list: Option<bool> = None;
    for paragraph in &note.paragraphs {
        let kind = paragraph.list.map(|n| n != 0);
        if kind != list {
            if let Some(numbered) = list {
                html.push_str(if numbered { "</ol>" } else { "</ul>" });
            }
            if let Some(numbered) = kind {
                html.push_str(if numbered { "<ol>" } else { "<ul>" });
            }
            list = kind;
        }
        match paragraph.list {
            Some(n) if n > 0 => html.push_str(&format!(
                "<li value=\"{n}\" style=\"white-space:pre-wrap\">"
            )),
            Some(_) => html.push_str("<li style=\"white-space:pre-wrap\">"),
            None => html.push_str("<p style=\"white-space:pre-wrap\">"),
        }
        for run in &paragraph.runs {
            if run.bold {
                html.push_str("<strong>");
            }
            if run.italic {
                html.push_str("<em>");
            }
            if run.underline {
                html.push_str("<u>");
            }
            if run.strike {
                html.push_str("<s>");
            }
            let link = run.link.as_deref().and_then(safe_link);
            if let Some(link) = &link {
                html.push_str(&format!("<a href=\"{}\">", escape_html(link)));
            }
            html.push_str(&escape_html(&run.text).replace('\n', "<br>"));
            if link.is_some() {
                html.push_str("</a>");
            }
            if run.strike {
                html.push_str("</s>");
            }
            if run.underline {
                html.push_str("</u>");
            }
            if run.italic {
                html.push_str("</em>");
            }
            if run.bold {
                html.push_str("</strong>");
            }
        }
        html.push_str(if paragraph.list.is_some() {
            "</li>"
        } else {
            "</p>"
        });
    }
    if let Some(numbered) = list {
        html.push_str(if numbered { "</ol>" } else { "</ul>" });
    }
    html
}

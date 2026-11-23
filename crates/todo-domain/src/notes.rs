//! A small, portable rich-note format. HTML is input/output data, never a view.
//! Only text, character emphasis, lists and validated web links enter the model.
mod html;

pub const MAX_NOTE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RichNote {
    pub paragraphs: Vec<NoteParagraph>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoteParagraph {
    /// `Some(0)` is a bullet; positive values are explicit ordered-list numbers.
    pub list: Option<u32>,
    pub runs: Vec<NoteRun>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoteRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub link: Option<String>,
}

impl RichNote {
    pub fn plain(text: &str) -> Self {
        Self {
            paragraphs: text
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .split('\n')
                .map(|text| NoteParagraph {
                    runs: vec![NoteRun {
                        text: text.into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .collect(),
        }
    }

    pub fn from_html(html: &str) -> Result<Self, &'static str> {
        html::parse(html)
    }

    pub fn text(&self) -> String {
        self.paragraphs
            .iter()
            .map(|paragraph| {
                let mut text = paragraph.prefix();
                for run in &paragraph.runs {
                    text.push_str(&run.text);
                }
                text
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn to_html(&self) -> String {
        html::write(self)
    }

    pub fn has_formatting(&self) -> bool {
        self.paragraphs.iter().any(|p| {
            p.list.is_some()
                || p.runs
                    .iter()
                    .any(|r| r.bold || r.italic || r.underline || r.strike || r.link.is_some())
        })
    }
}

impl NoteParagraph {
    pub fn prefix(&self) -> String {
        match self.list {
            None => String::new(),
            Some(0) => "• ".into(),
            Some(n) => format!("{n}. "),
        }
    }

    pub fn push(&mut self, run: NoteRun) {
        if run.text.is_empty() {
            return;
        }
        if let Some(last) = self.runs.last_mut() {
            if last.bold == run.bold
                && last.italic == run.italic
                && last.underline == run.underline
                && last.strike == run.strike
                && last.link == run.link
            {
                last.text.push_str(&run.text);
                return;
            }
        }
        self.runs.push(run);
    }
}

/// User-visible web links require an explicit scheme/host and no controls or
/// credentials. The same rule applies to toolbar entry, HTML and automatic links.
pub fn safe_link(value: &str) -> Option<String> {
    let value = value.trim();
    let scheme = value
        .get(..8)
        .filter(|s| s.eq_ignore_ascii_case("https://"))
        .map(|_| 8)
        .or_else(|| {
            value
                .get(..7)
                .filter(|s| s.eq_ignore_ascii_case("http://"))
                .map(|_| 7)
        })?;
    let host = value[scheme..].split(['/', '?', '#']).next()?;
    if host.is_empty()
        || host.contains('@')
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
        || value.contains(['\\', '<', '>', '"'])
    {
        return None;
    }
    Some(value.to_owned())
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests;

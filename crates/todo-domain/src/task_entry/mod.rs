//! Conservative English date/time recognition at the end of a new task title.
//! Original text is retained by default; task editing never invokes this parser.

mod date;
mod date_phrase;
mod grammar;
mod repeat;
mod time;

use crate::{Date, Recurrence, Reminder};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEntry {
    pub title: String,
    #[serde(default)]
    pub due_date: Option<Date>,
    #[serde(default)]
    pub start_date: Option<Date>,
    #[serde(default)]
    pub reminder: Option<Reminder>,
    #[serde(default)]
    pub recurrence: Option<Recurrence>,
}

impl TaskEntry {
    pub fn plain(title: impl Into<String>) -> Self {
        Self {
            title: title.into().trim().to_owned(),
            due_date: None,
            start_date: None,
            reminder: None,
            recurrence: None,
        }
    }

    /// Recognize a trailing English date or reminder expression. Date-only input
    /// sets a due date; a clock time sets a reminder. A time without a date means
    /// the supplied local day, even when that time has passed. Only explicit
    /// relative times ("in 90 minutes") advance through the local day boundary.
    pub fn parse_english(
        input: &str,
        today: Date,
        current_minutes: u16,
        remove_recognized: bool,
    ) -> Self {
        let mut entry = Self::plain(input);
        if entry.title.len() > 8192 {
            return entry;
        }
        let words = words(&entry.title);
        if words.len() > 512 {
            return entry;
        }
        if let Some(found) = grammar::extract(&words, today) {
            entry.due_date = found.due;
            entry.start_date = found.start;
            entry.reminder = found.reminder;
            entry.recurrence = found.repeat;
            if remove_recognized {
                let left = entry.title[..words[found.from].start].trim_end();
                let right = if found.to < words.len() {
                    entry.title[words[found.to].start..].trim_start()
                } else {
                    ""
                };
                let title = match (left.is_empty(), right.is_empty()) {
                    (false, false) => format!("{left} {right}"),
                    (false, true) => left.to_owned(),
                    (true, false) => right.to_owned(),
                    _ => entry.title.clone(),
                };
                entry.title = title;
            }
            return entry;
        }
        if grammar::contains_repeat_marker(&words) {
            return entry;
        }
        let matched =
            if let Some((start, reminder)) = time::relative(&words, today, current_minutes) {
                entry.reminder = Some(reminder);
                Some(start)
            } else if let Some(clock) = time::suffix(&words) {
                if let Some((start, date)) = date::suffix(&words[..clock.start], today) {
                    entry.reminder = Some(Reminder {
                        date,
                        minutes_after_midnight: clock.minutes,
                        notified: false,
                    });
                    Some(start)
                } else if clock.has_at
                    && !date::looks_like_date_tail(&words[..clock.start])
                    && !date::ambiguous_prefix(&words, clock.start)
                {
                    entry.reminder = Some(Reminder {
                        date: today,
                        minutes_after_midnight: clock.minutes,
                        notified: false,
                    });
                    Some(clock.start)
                } else {
                    None
                }
            } else if let Some((start, date)) = date::suffix(&words, today) {
                entry.due_date = Some(date);
                Some(start)
            } else {
                None
            };
        if remove_recognized {
            if let Some(start) = matched {
                let title = entry.title[..words[start].start].trim_end();
                if !title.is_empty() {
                    entry.title = title.to_owned();
                }
            }
        }
        entry
    }
}

pub(super) struct Word<'a> {
    text: &'a str,
    start: usize,
}

impl Word<'_> {
    fn is(&self, value: &str) -> bool {
        self.text.eq_ignore_ascii_case(value)
    }
}

fn words(input: &str) -> Vec<Word<'_>> {
    let mut offset = 0;
    input
        .split_whitespace()
        .map(|text| {
            // Search forward so repeated words and non-ASCII titles retain their
            // original byte boundaries and internal whitespace when shortened.
            let start = offset + input[offset..].find(text).expect("word in source");
            offset = start + text.len();
            Word { text, start }
        })
        .collect()
}

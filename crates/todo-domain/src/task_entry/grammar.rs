use super::{Word, date, date_phrase, repeat, time};
use crate::{Date, Recurrence, Reminder};

pub(super) struct Found {
    pub from: usize,
    pub to: usize,
    pub due: Option<Date>,
    pub start: Option<Date>,
    pub reminder: Option<Reminder>,
    pub repeat: Option<Recurrence>,
}
pub(super) fn contains_repeat_marker(words: &[Word<'_>]) -> bool {
    words
        .iter()
        .any(|word| repeat::marker(word) || word.is("remind"))
}

/// Explicit clauses can be embedded. Unintroduced date words remain suffix-only
/// so prose such as “keep tomorrow in the title” is never rewritten.
pub(super) fn extract(words: &[Word<'_>], today: Date) -> Option<Found> {
    for from in 0..words.len() {
        if date::ambiguous_prefix(words, from) || from > 0 && words[from - 1].is("repeat") {
            continue;
        }
        let token = &words[from];
        if repeat::marker(token) {
            if let Some(parsed) = repeat::at(words, from, today) {
                let embedded = token.is("every") || token.is("each") || token.is("repeat");
                if !boundary(words, parsed.end) || (!embedded && parsed.end != words.len()) {
                    continue;
                }
                let due = parsed.rule.first_occurrence(today)?;
                return Some(Found {
                    from,
                    to: parsed.end,
                    due: Some(due),
                    start: None,
                    reminder: parsed.minutes.map(|minutes| reminder(due, minutes)),
                    repeat: Some(parsed.rule),
                });
            }
            continue;
        }
        let ranged = token.is("from") || token.is("between");
        let due_prefix = token.is("due") || token.is("by");
        let remind = token.is("remind");
        let on = token.is("on");
        if !(ranged || due_prefix || remind || on) {
            continue;
        }
        let mut cursor = from + 1;
        if remind && words.get(cursor).is_some_and(|word| word.is("me")) {
            cursor += 1;
        }
        if (due_prefix || remind) && words.get(cursor).is_some_and(|word| word.is("on")) {
            cursor += 1;
        }
        let Some((end, first)) = date_phrase::at(words, cursor, today) else {
            continue;
        };
        cursor = end;
        let (start, due) = if ranged {
            let separator = words.get(cursor).is_some_and(|word| {
                if token.is("between") {
                    word.is("and")
                } else {
                    word.is("to") || word.is("through") || word.is("until")
                }
            });
            if !separator {
                continue;
            }
            let Some((end, last)) = date_phrase::at(words, cursor + 1, first) else {
                continue;
            };
            if last < first {
                continue;
            }
            cursor = end;
            (Some(first), last)
        } else {
            (None, first)
        };
        let clock = time::at(words, cursor);
        if words.get(cursor).is_some_and(|word| word.is("at")) && clock.is_none() {
            continue;
        }
        let mut minutes = None;
        if let Some((end, value)) = clock {
            cursor = end;
            minutes = Some(value);
        }
        let mut recurrence = None;
        if words.get(cursor).is_some_and(repeat::marker) {
            let Some(parsed) = repeat::at(words, cursor, due) else {
                continue;
            };
            if minutes.is_some() && parsed.minutes.is_some() {
                continue;
            }
            minutes = minutes.or(parsed.minutes);
            recurrence = Some(parsed.rule);
            cursor = parsed.end;
        }
        if !boundary(words, cursor)
            || (on && cursor != words.len())
            || (remind && minutes.is_none())
        {
            continue;
        }
        let due = recurrence
            .as_ref()
            .and_then(|rule| rule.first_occurrence(due))
            .unwrap_or(due);
        return Some(Found {
            from,
            to: cursor,
            start,
            due: (ranged || due_prefix || recurrence.is_some() || minutes.is_none()).then_some(due),
            reminder: minutes.map(|minutes| reminder(due, minutes)),
            repeat: recurrence,
        });
    }
    None
}
fn reminder(date: Date, minutes_after_midnight: u16) -> Reminder {
    Reminder {
        date,
        minutes_after_midnight,
        notified: false,
    }
}
fn boundary(words: &[Word<'_>], end: usize) -> bool {
    !words.get(end).is_some_and(|word| {
        [
            "or", "and", "to", "through", "until", "before", "after", "except", "at", "on",
            "starting",
        ]
        .iter()
        .any(|value| word.is(value))
            || word
                .text
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_digit())
    })
}

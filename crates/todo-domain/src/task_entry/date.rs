use super::Word;
use crate::Date;
use chrono::{Datelike, Days, Weekday};

pub(super) fn suffix(words: &[Word<'_>], today: Date) -> Option<(usize, Date)> {
    let end = words.len();
    let last = words.last()?;
    let (mut start, date) = if end >= 2 && words[end - 2].is("next") && last.is("week") {
        let days = 7 - today.weekday().num_days_from_monday();
        (end - 2, today.checked_add_days(Days::new(days.into()))?)
    } else if last.is("today") {
        (end - 1, today)
    } else if last.is("tomorrow") {
        (end - 1, today.succ_opt()?)
    } else if let Some(day) = weekday(last.text) {
        let delta = (day.num_days_from_monday() + 7 - today.weekday().num_days_from_monday()) % 7;
        let next = end >= 2 && words[end - 2].is("next");
        let days = if next && delta == 0 { 7 } else { delta };
        (
            end - 1 - usize::from(next),
            today.checked_add_days(Days::new(days.into()))?,
        )
    } else if is_iso(last.text) {
        (end - 1, Date::parse_from_str(last.text, "%Y-%m-%d").ok()?)
    } else {
        return None;
    };
    if start > 0
        && ["on", "by", "due"]
            .iter()
            .any(|word| words[start - 1].is(word))
    {
        start -= 1;
        if start > 0 && words[start - 1].is("due") {
            start -= 1;
        }
    }
    (!ambiguous_prefix(words, start)).then_some((start, date))
}

/// Do not reinterpret the last half of a range, alternative, or negation.
pub(super) fn ambiguous_prefix(words: &[Word<'_>], start: usize) -> bool {
    start > 0
        && [
            "or", "and", "to", "through", "until", "before", "after", "not", "except", "every",
            "each",
        ]
        .iter()
        .any(|word| words[start - 1].is(word))
}

pub(super) fn looks_like_date_tail(words: &[Word<'_>]) -> bool {
    words.last().is_some_and(|word| {
        ["today", "tomorrow", "tonight", "morning", "afternoon", "evening", "week", "weeks", "day", "days", "month", "year", "daily", "weekly", "monthly", "yearly"]
            .iter().any(|value| word.is(value))
            || weekday(word.text).is_some()
            || (word.text.as_bytes().first().is_some_and(u8::is_ascii_digit)
                && (word.text.contains('/') || word.text.split('-').count() == 3))
            // An unsupported month/day phrase must not become an unrelated
            // time-only reminder for today. A bare name such as April can still
            // be part of "Call April at 3pm".
            || words.windows(2).rev().take(2).any(|pair| month(pair[0].text)
                && pair[1].text.as_bytes().first().is_some_and(u8::is_ascii_digit))
            || (month(word.text) && words.len() >= 2
                && ["in", "on", "next", "this"].iter().any(|prefix| words[words.len() - 2].is(prefix)))
    })
}

fn month(value: &str) -> bool {
    [
        "january",
        "jan",
        "february",
        "feb",
        "march",
        "mar",
        "april",
        "apr",
        "may",
        "june",
        "jun",
        "july",
        "jul",
        "august",
        "aug",
        "september",
        "sep",
        "sept",
        "october",
        "oct",
        "november",
        "nov",
        "december",
        "dec",
    ]
    .iter()
    .any(|month| value.eq_ignore_ascii_case(month))
}

fn is_iso(value: &str) -> bool {
    value.len() == 10
        && value.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

fn weekday(value: &str) -> Option<Weekday> {
    Some(match value.to_ascii_lowercase().as_str() {
        "monday" | "mon" => Weekday::Mon,
        "tuesday" | "tue" | "tues" => Weekday::Tue,
        "wednesday" | "wed" => Weekday::Wed,
        "thursday" | "thu" | "thur" | "thurs" => Weekday::Thu,
        "friday" | "fri" => Weekday::Fri,
        "saturday" | "sat" => Weekday::Sat,
        "sunday" | "sun" => Weekday::Sun,
        _ => return None,
    })
}

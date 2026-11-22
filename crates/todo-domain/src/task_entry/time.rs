use super::{Word, date};
use crate::{Date, Reminder};
use chrono::Days;

pub(super) struct Clock {
    pub start: usize,
    pub minutes: u16,
    pub has_at: bool,
}

pub(super) fn suffix(words: &[Word<'_>]) -> Option<Clock> {
    let end = words.len();
    let last = words.last()?;
    let (mut start, minutes) = if end >= 2 && (last.is("am") || last.is("pm")) {
        (
            end - 2,
            clock(&format!("{}{}", words[end - 2].text, last.text))?,
        )
    } else {
        (end - 1, clock(last.text)?)
    };
    let has_at = start > 0 && words[start - 1].is("at");
    if has_at {
        start -= 1;
    }
    Some(Clock {
        start,
        minutes,
        has_at,
    })
}

pub(super) fn relative(words: &[Word<'_>], today: Date, current: u16) -> Option<(usize, Reminder)> {
    let start = words.len().checked_sub(3)?;
    if !words[start].is("in") || current >= 1440 || date::ambiguous_prefix(words, start) {
        return None;
    }
    let amount = words[start + 1].text.parse::<u16>().ok()?;
    if amount == 0 {
        return None;
    }
    let unit = &words[start + 2];
    let multiplier = if unit.is("minute") || unit.is("minutes") || unit.is("min") {
        1u32
    } else if unit.is("hour") || unit.is("hours") {
        60
    } else {
        return None;
    };
    let minutes = u32::from(current) + u32::from(amount) * multiplier;
    Some((
        start,
        Reminder {
            date: today.checked_add_days(Days::new(u64::from(minutes / 1440)))?,
            minutes_after_midnight: (minutes % 1440) as u16,
            notified: false,
        },
    ))
}

fn clock(value: &str) -> Option<u16> {
    let value = value.to_ascii_lowercase();
    if value == "noon" {
        return Some(720);
    }
    if value == "midnight" {
        return Some(0);
    }
    let (clock, period) = if let Some(clock) = value.strip_suffix("am") {
        (clock, Some(false))
    } else if let Some(clock) = value.strip_suffix("pm") {
        (clock, Some(true))
    } else {
        (value.as_str(), None)
    };
    let (hour, minute) = if let Some((hour, minute)) = clock.split_once(':') {
        if minute.len() != 2 || !minute.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        (hour, minute.parse::<u16>().ok()?)
    } else if period.is_some() {
        (clock, 0)
    } else {
        // "at 3" does not tell us whether 03:00 or 15:00 was intended.
        return None;
    };
    if hour.is_empty()
        || hour.len() > 2
        || !hour.bytes().all(|byte| byte.is_ascii_digit())
        || minute >= 60
    {
        return None;
    }
    let hour = hour.parse::<u16>().ok()?;
    let hour = match period {
        Some(pm) if (1..=12).contains(&hour) => hour % 12 + if pm { 12 } else { 0 },
        None if hour < 24 => hour,
        _ => return None,
    };
    Some(hour * 60 + minute)
}

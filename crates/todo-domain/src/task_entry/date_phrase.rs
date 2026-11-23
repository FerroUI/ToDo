use super::Word;
use crate::{Date, RepeatDay};
use chrono::{Datelike, Days, Months};

/// Parse a complete unambiguous calendar phrase at a token boundary. Numeric
/// slash dates intentionally stay literal because their locale is unknown.
pub(super) fn at(words: &[Word<'_>], start: usize, today: Date) -> Option<(usize, Date)> {
    let token = words.get(start)?;
    if token.is("today") {
        return Some((start + 1, today));
    }
    if token.is("tomorrow") {
        return Some((start + 1, today.succ_opt()?));
    }
    if token.is("next") && words.get(start + 1).is_some_and(|word| word.is("week")) {
        return Some((
            start + 2,
            today.checked_add_days(Days::new(u64::from(
                7 - today.weekday().num_days_from_monday(),
            )))?,
        ));
    }
    let next = token.is("next");
    if let Some(day) = weekday(words.get(start + usize::from(next))?.text) {
        let delta = (day.number() + 7 - today.weekday().num_days_from_sunday()) % 7;
        let days = if next && delta == 0 { 7 } else { delta };
        return Some((
            start + 1 + usize::from(next),
            today.checked_add_days(Days::new(u64::from(days)))?,
        ));
    }
    if token.is("in") {
        let count = positive(words.get(start + 1)?.text)?;
        let unit = words.get(start + 2)?;
        let date = if unit.is("day") || unit.is("days") {
            today.checked_add_days(Days::new(u64::from(count)))
        } else if unit.is("week") || unit.is("weeks") {
            today.checked_add_days(Days::new(u64::from(count) * 7))
        } else if unit.is("month") || unit.is("months") {
            today.checked_add_months(Months::new(u32::from(count)))
        } else if unit.is("year") || unit.is("years") {
            today.checked_add_months(Months::new(u32::from(count) * 12))
        } else {
            None
        }?;
        return Some((start + 3, date));
    }
    if token.text.len() == 10
        && token.text.bytes().enumerate().all(|(i, byte)| {
            if i == 4 || i == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
    {
        return Some((start + 1, token.text.parse().ok()?));
    }
    let (mut end, month, day) = if let Some(month) = month(token.text) {
        (start + 2, month, day(words.get(start + 1)?.text)?)
    } else {
        let day = day(token.text)?;
        let of = words.get(start + 1).is_some_and(|word| word.is("of"));
        (
            start + 2 + usize::from(of),
            month(words.get(start + 1 + usize::from(of))?.text)?,
            day,
        )
    };
    let year = words
        .get(end)
        .filter(|word| word.text.len() == 4 && word.text.bytes().all(|byte| byte.is_ascii_digit()));
    let date = if let Some(year) = year {
        end += 1;
        let year = year
            .text
            .parse::<i32>()
            .ok()
            .filter(|value| (1..=9999).contains(value))?;
        Date::from_ymd_opt(year, month, day)?
    } else {
        // Leap-day phrases find their next valid occurrence without rolling an
        // invalid day such as February 30 into a different month.
        (today.year()..=today.year().checked_add(8)?)
            .filter_map(|year| Date::from_ymd_opt(year, month, day))
            .find(|date| *date >= today)?
    };
    Some((end, date))
}

pub(super) fn positive(value: &str) -> Option<u16> {
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse::<u16>().ok().filter(|number| *number > 0))
        .flatten()
}
pub(super) fn day(value: &str) -> Option<u32> {
    let value = value.trim_end_matches(',');
    let number = value.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    if number.is_empty() || number.len() > 2 || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let day = number
        .parse::<u32>()
        .ok()
        .filter(|day| (1..=31).contains(day))?;
    let suffix = &value[number.len()..];
    let expected = if (11..=13).contains(&(day % 100)) {
        "th"
    } else {
        match day % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    (suffix.is_empty() || suffix.eq_ignore_ascii_case(expected)).then_some(day)
}
pub(super) fn month(value: &str) -> Option<u32> {
    Some(
        match value
            .trim_end_matches([',', '.'])
            .to_ascii_lowercase()
            .as_str()
        {
            "january" | "jan" => 1,
            "february" | "feb" => 2,
            "march" | "mar" => 3,
            "april" | "apr" => 4,
            "may" => 5,
            "june" | "jun" => 6,
            "july" | "jul" => 7,
            "august" | "aug" => 8,
            "september" | "sep" | "sept" => 9,
            "october" | "oct" => 10,
            "november" | "nov" => 11,
            "december" | "dec" => 12,
            _ => return None,
        },
    )
}
pub(super) fn weekday(value: &str) -> Option<RepeatDay> {
    Some(
        match value.trim_end_matches(',').to_ascii_lowercase().as_str() {
            "sunday" | "sun" => RepeatDay::Sunday,
            "monday" | "mon" => RepeatDay::Monday,
            "tuesday" | "tue" | "tues" => RepeatDay::Tuesday,
            "wednesday" | "wed" => RepeatDay::Wednesday,
            "thursday" | "thu" | "thur" | "thurs" => RepeatDay::Thursday,
            "friday" | "fri" => RepeatDay::Friday,
            "saturday" | "sat" => RepeatDay::Saturday,
            _ => return None,
        },
    )
}

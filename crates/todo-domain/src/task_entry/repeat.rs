use super::{Word, date_phrase as date, time};
use crate::{Date, Recurrence, RelativeMode, RepeatDay, RepeatEnd, RepeatUnit, WeekIndex};
use chrono::Datelike;

pub(super) struct Parsed {
    pub end: usize,
    pub rule: Recurrence,
    pub minutes: Option<u16>,
}
pub(super) fn marker(word: &Word<'_>) -> bool {
    [
        "every", "each", "repeat", "daily", "weekly", "monthly", "yearly",
    ]
    .iter()
    .any(|value| word.is(value))
}
pub(super) fn at(words: &[Word<'_>], start: usize, today: Date) -> Option<Parsed> {
    let mut cursor = start;
    eat(words, &mut cursor, "repeat");
    let token = words.get(cursor)?;
    let mut rule = if let Some(unit) = ["daily", "weekly", "monthly", "yearly"]
        .iter()
        .position(|value| token.is(value))
    {
        cursor += 1;
        Recurrence::new(
            [
                RepeatUnit::Day,
                RepeatUnit::Week,
                RepeatUnit::Month,
                RepeatUnit::Year,
            ][unit],
            1,
        )
    } else {
        if !(eat(words, &mut cursor, "every") || eat(words, &mut cursor, "each")) {
            return None;
        }
        let every = if eat(words, &mut cursor, "other") {
            2
        } else if let Some(count) = words.get(cursor).and_then(|word| date::positive(word.text)) {
            cursor += 1;
            count
        } else {
            1
        };
        if let Some((end, days)) = days(words, cursor) {
            cursor = end;
            let mut rule = Recurrence::new(RepeatUnit::Week, every);
            rule.days_of_week = days;
            rule
        } else {
            let token = words.get(cursor)?;
            let unit = if token.is("day") || token.is("days") {
                RepeatUnit::Day
            } else if token.is("weekday") || token.is("weekdays") {
                RepeatUnit::Weekday
            } else if token.is("week") || token.is("weeks") {
                RepeatUnit::Week
            } else if token.is("month") || token.is("months") {
                RepeatUnit::Month
            } else if token.is("year") || token.is("years") {
                RepeatUnit::Year
            } else {
                return None;
            };
            cursor += 1;
            Recurrence::new(unit, every)
        }
    };
    rule.anchor = Some(today);
    if eat(words, &mut cursor, "on") {
        eat(words, &mut cursor, "the");
        if rule.unit == RepeatUnit::Week {
            let (end, chosen) = days(words, cursor)?;
            cursor = end;
            rule.days_of_week = chosen;
        } else if matches!(rule.unit, RepeatUnit::Month | RepeatUnit::Year) {
            if let Some(index) = words.get(cursor).and_then(ordinal) {
                cursor += 1;
                if eat(words, &mut cursor, "day") {
                    rule.day_of_month = Some(match index {
                        WeekIndex::First => 1,
                        WeekIndex::Second => 2,
                        WeekIndex::Third => 3,
                        WeekIndex::Fourth => 4,
                        WeekIndex::Last => 31,
                    });
                } else {
                    rule.ordinal = Some(index);
                    if eat(words, &mut cursor, "weekday") || eat(words, &mut cursor, "weekdays") {
                        rule.days_of_week = RepeatDay::ALL[1..6].to_vec();
                        rule.relative_mode = RelativeMode::NthMatchingDay;
                    } else if eat(words, &mut cursor, "weekend") {
                        rule.days_of_week = vec![RepeatDay::Sunday, RepeatDay::Saturday];
                        rule.relative_mode = RelativeMode::NthMatchingDay;
                    } else {
                        let (end, chosen) = days(words, cursor)?;
                        cursor = end;
                        rule.days_of_week = chosen;
                    }
                }
                if rule.unit == RepeatUnit::Year {
                    if !(eat(words, &mut cursor, "of") || eat(words, &mut cursor, "in")) {
                        return None;
                    }
                    rule.month = Some(date::month(words.get(cursor)?.text)? as u8);
                    cursor += 1;
                }
            } else if rule.unit == RepeatUnit::Year {
                let (end, selected) = date::at(words, cursor, today)?;
                rule.month = Some(selected.month() as u8);
                rule.day_of_month = Some(selected.day() as u8);
                cursor = end;
            } else {
                rule.day_of_month = Some(date::day(words.get(cursor)?.text)? as u8);
                cursor += 1;
            }
        } else {
            return None;
        }
    }
    let mut minutes = None;
    let mut has_start = false;
    let mut has_end = false;
    loop {
        if words.get(cursor).is_some_and(|word| word.is("at")) {
            if minutes.is_some() {
                return None;
            }
            let (end, clock) = time::at(words, cursor)?;
            minutes = Some(clock);
            cursor = end;
        } else if eat(words, &mut cursor, "starting") || eat(words, &mut cursor, "from") {
            if has_start {
                return None;
            }
            has_start = true;
            eat(words, &mut cursor, "on");
            let (end, date) = date::at(words, cursor, today)?;
            rule.anchor = Some(date);
            cursor = end;
        } else if eat(words, &mut cursor, "until") || eat(words, &mut cursor, "through") {
            if has_end {
                return None;
            }
            has_end = true;
            let (end, date) = date::at(words, cursor, rule.anchor.unwrap())?;
            rule.end = RepeatEnd::On(date);
            cursor = end;
        } else if eat(words, &mut cursor, "for") {
            if has_end {
                return None;
            }
            has_end = true;
            let count = words
                .get(cursor)?
                .text
                .parse::<u32>()
                .ok()
                .filter(|count| *count > 0 && *count <= i32::MAX as u32)?;
            cursor += 1;
            if !(eat(words, &mut cursor, "occurrences")
                || eat(words, &mut cursor, "occurrence")
                || eat(words, &mut cursor, "times"))
            {
                return None;
            }
            rule.end = RepeatEnd::After(count);
        } else {
            break;
        }
    }
    rule.normalize();
    rule.validate().ok()?;
    rule.first_occurrence(today)?;
    Some(Parsed {
        end: cursor,
        rule,
        minutes,
    })
}

fn eat(words: &[Word<'_>], cursor: &mut usize, value: &str) -> bool {
    if words.get(*cursor).is_some_and(|word| word.is(value)) {
        *cursor += 1;
        true
    } else {
        false
    }
}
fn ordinal(word: &Word<'_>) -> Option<WeekIndex> {
    Some(match word.text.to_ascii_lowercase().as_str() {
        "first" => WeekIndex::First,
        "second" => WeekIndex::Second,
        "third" => WeekIndex::Third,
        "fourth" => WeekIndex::Fourth,
        "last" => WeekIndex::Last,
        _ => return None,
    })
}
fn days(words: &[Word<'_>], start: usize) -> Option<(usize, Vec<RepeatDay>)> {
    let mut cursor = start;
    let mut result = Vec::new();
    loop {
        let word = words.get(cursor)?;
        let mut chosen = Vec::new();
        for part in word.text.trim_end_matches(',').split(',') {
            if let Some((first, last)) = part.split_once('-') {
                let first = date::weekday(first)?;
                let last = date::weekday(last)?;
                for offset in 0..=((last.number() + 7 - first.number()) % 7) {
                    chosen.push(RepeatDay::ALL[((first.number() + offset) % 7) as usize]);
                }
            } else {
                chosen.push(date::weekday(part)?);
            }
        }
        result.extend(chosen);
        cursor += 1;
        if words
            .get(cursor)
            .is_some_and(|word| word.is("through") || word.is("to"))
            && words
                .get(cursor + 1)
                .and_then(|word| date::weekday(word.text))
                .is_some()
        {
            let first = *result.last()?;
            let last = date::weekday(words[cursor + 1].text)?;
            for offset in 1..=((last.number() + 7 - first.number()) % 7) {
                result.push(RepeatDay::ALL[((first.number() + offset) % 7) as usize]);
            }
            cursor += 2;
        }
        if eat(words, &mut cursor, "and") {
            continue;
        }
        if words
            .get(cursor)
            .and_then(|word| date::weekday(word.text))
            .is_some()
            && word.text.ends_with(',')
        {
            continue;
        }
        break;
    }
    result.sort_unstable();
    result.dedup();
    Some((cursor, result))
}

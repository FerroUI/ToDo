//! Bounded Gregorian MS-OXOTASK/MS-OXOCAL RecurrencePattern decoder.
//! Original binary properties remain available when a producer uses another
//! calendar, sliding completion-relative tasks, or future protocol fields.
use chrono::{Datelike, Days};
use todo_domain::{Date, Recurrence, RelativeMode, RepeatDay, RepeatEnd, RepeatUnit, WeekIndex};

pub(crate) fn decode(bytes: &[u8]) -> Result<Recurrence, &'static str> {
    let mut input = Reader { bytes, at: 0 };
    if input.u16()? != 0x3004 || input.u16()? != 0x3004 {
        return Err("an unsupported recurrence version");
    }
    let frequency = input.u16()?;
    let pattern = input.u16()?;
    if ![0, 1, 2, 9, 10, 11, 12].contains(&input.u16()?) {
        return Err("a non-Gregorian recurrence calendar");
    }
    let phase = input.u32()?;
    let period = input.u32()?;
    if period == 0 {
        return Err("a zero recurrence interval");
    }
    if input.u32()? != 0 {
        return Err("a completion-relative Outlook recurrence");
    }
    let selector = match pattern {
        0 => None,
        1..=4 => Some(input.u32()?),
        _ => return Err("an unsupported recurrence pattern"),
    };
    let ordinal = if pattern == 3 {
        Some(input.u32()?)
    } else {
        None
    };
    let end_type = input.u32()?;
    let count = input.u32()?;
    let week_start = input.u32()?;
    if week_start > 6 {
        return Err("an invalid first weekday");
    }
    if input.u32()? != 0 || input.u32()? != 0 {
        return Err("task recurrence exceptions");
    }
    let start_minutes = input.u32()?;
    let end_minutes = input.u32()?;
    if input.at != bytes.len() {
        return Err("extra recurrence protocol fields");
    }
    let anchor = minute_date(start_minutes)?;
    let (unit, every) = match (frequency, pattern) {
        (0x200a, 0) if period % 1440 == 0 && period / 1440 <= 999 => {
            (RepeatUnit::Day, period / 1440)
        }
        (0x200a | 0x200b, 1) if period <= 99 => (RepeatUnit::Week, period),
        (0x200c, 2..=4) if period <= 99 => (RepeatUnit::Month, period),
        (0x200d, 2..=4) if period == 12 => (RepeatUnit::Year, 1),
        _ => return Err("an unsupported recurrence interval or frequency"),
    };
    let mut rule = Recurrence::new(unit, every as u16);
    rule.anchor = Some(anchor);
    if unit == RepeatUnit::Year {
        rule.month = Some(anchor.month() as u8);
    }
    if pattern == 1 || pattern == 3 {
        let mask = selector.unwrap();
        if mask & 0x7f == 0 {
            return Err("an empty recurrence weekday selection");
        }
        rule.days_of_week = RepeatDay::ALL
            .into_iter()
            .filter(|day| mask & (1 << day.number()) != 0)
            .collect();
        if pattern == 1 {
            rule.week_start = RepeatDay::ALL[week_start as usize];
        } else {
            rule.ordinal = Some(match ordinal.unwrap() {
                1 => WeekIndex::First,
                2 => WeekIndex::Second,
                3 => WeekIndex::Third,
                4 => WeekIndex::Fourth,
                5 => WeekIndex::Last,
                _ => return Err("an invalid relative recurrence ordinal"),
            });
            rule.relative_mode = RelativeMode::NthMatchingDay;
        }
    } else if pattern == 2 || pattern == 4 {
        let day = if pattern == 4 { 31 } else { selector.unwrap() };
        if !(1..=31).contains(&day) {
            return Err("an invalid recurrence day of month");
        }
        rule.day_of_month = Some(day as u8);
    }
    rule.end = match end_type {
        0x2023 | 0xffffffff => RepeatEnd::Never,
        0x2021 => RepeatEnd::On(minute_date(end_minutes)?),
        0x2022 if count > 0 && count <= i32::MAX as u32 => RepeatEnd::After(count),
        _ => return Err("an invalid recurrence range"),
    };
    let expected_phase = match unit {
        RepeatUnit::Day => start_minutes % period,
        RepeatUnit::Week => (i64::from(start_minutes)
            - i64::from((anchor.weekday().num_days_from_sunday() + 7 - week_start) % 7) * 1440)
            .rem_euclid(i64::from(period) * 10080) as u32,
        _ => {
            let months =
                ((anchor.year() - 1601) * 12 + anchor.month0() as i32).rem_euclid(period as i32);
            let phase_date = Date::from_ymd_opt(1601 + months / 12, (months % 12) as u32 + 1, 1)
                .ok_or("an invalid recurrence phase")?;
            (phase_date.signed_duration_since(epoch()).num_days() * 1440) as u32
        }
    };
    if phase != expected_phase {
        return Err("a recurrence phase inconsistent with its start date");
    }
    rule.normalize();
    rule.validate()?;
    if rule.first_occurrence(anchor) != Some(anchor) {
        return Err("a recurrence start outside its pattern");
    }
    if !matches!(rule.end, RepeatEnd::Never)
        && rule.last_occurrence(anchor) != Some(minute_date(end_minutes)?)
    {
        return Err("a recurrence end inconsistent with its pattern");
    }
    Ok(rule)
}

fn epoch() -> Date {
    Date::from_ymd_opt(1601, 1, 1).unwrap()
}
fn minute_date(minutes: u32) -> Result<Date, &'static str> {
    if minutes % 1440 != 0 {
        return Err("a recurrence date that is not midnight");
    }
    epoch()
        .checked_add_days(Days::new(u64::from(minutes / 1440)))
        .filter(|date| date.year() <= 9999)
        .ok_or("a recurrence date outside the supported calendar")
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], &'static str> {
        let value = self
            .bytes
            .get(self.at..self.at + N)
            .ok_or("a truncated recurrence pattern")?;
        self.at += N;
        Ok(value.try_into().unwrap())
    }
    fn u16(&mut self) -> Result<u16, &'static str> {
        Ok(u16::from_le_bytes(self.take()?))
    }
    fn u32(&mut self) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(self.take()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn weekly() -> Vec<u8> {
        // MS-OXOCAL example: Mon/Thu/Fri from 2007-04-02, 12 occurrences.
        let hex = "043004300b2001000000c0210000010000000000000032000000222000000c000000000000000000000000000000e047bc0c80d4bc0c";
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn truncated_future_calendar_phase_and_exception_bombs_are_rejected_without_allocation() {
        let bytes = weekly();
        assert_eq!(
            decode(&bytes)
                .unwrap()
                .last_occurrence("2007-04-02".parse().unwrap()),
            Some("2007-04-27".parse().unwrap())
        );
        for length in 0..bytes.len() {
            assert!(decode(&bytes[..length]).is_err(), "truncation {length}");
        }
        for (offset, value) in [
            (0, 0xffff),
            (8, 8),
            (10, 1),
            (14, 0),
            (18, 1),
            (22, 0),
            (30, u32::MAX),
            (34, 7),
            (38, u32::MAX),
        ] {
            let mut malformed = bytes.clone();
            if offset < 10 {
                malformed[offset..offset + 2].copy_from_slice(&(value as u16).to_le_bytes());
            } else {
                malformed[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            }
            assert!(decode(&malformed).is_err(), "field {offset}");
        }
    }
}

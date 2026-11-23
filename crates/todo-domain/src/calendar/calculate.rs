use super::{Recurrence, RelativeMode, RepeatDay, RepeatEnd, RepeatUnit, WeekIndex};
use crate::Date;
use chrono::{Datelike, Days};

impl Recurrence {
    /// Next scheduled civil date strictly after both the current occurrence and
    /// completion. Binary search skips overdue periods without iterating over dates.
    pub fn next_after(&self, current: Date, completed: Date) -> Option<Date> {
        self.validate().ok()?;
        let anchor = self.anchor.unwrap_or(current);
        let index = self.index_after(current.max(completed), anchor)?;
        self.occurrence(index, anchor)
    }
    pub fn first_occurrence(&self, fallback: Date) -> Option<Date> {
        self.validate().ok()?;
        self.occurrence(0, self.anchor.unwrap_or(fallback))
    }
    /// Zero-based scheduled occurrence. Count ranges include the first occurrence.
    pub fn occurrence(&self, index: u32, fallback: Date) -> Option<Date> {
        let date = self.pattern_date(index, self.anchor.unwrap_or(fallback))?;
        match self.end {
            RepeatEnd::After(count) if index >= count => None,
            RepeatEnd::On(end) if date > end => None,
            _ => Some(date),
        }
    }
    pub fn last_occurrence(&self, fallback: Date) -> Option<Date> {
        let anchor = self.anchor.unwrap_or(fallback);
        match self.end {
            RepeatEnd::Never => None,
            RepeatEnd::After(count) => self.occurrence(count.checked_sub(1)?, anchor),
            RepeatEnd::On(end) => self
                .index_after(end, anchor)
                .and_then(|index| index.checked_sub(1))
                .and_then(|index| self.occurrence(index, anchor)),
        }
    }
    fn index_after(&self, boundary: Date, anchor: Date) -> Option<u32> {
        if self.pattern_date(0, anchor)? > boundary {
            return Some(0);
        }
        let mut low = 0;
        let mut high = 1u32;
        while self
            .pattern_date(high, anchor)
            .is_some_and(|date| date <= boundary)
        {
            low = high;
            high = high.checked_mul(2).unwrap_or(u32::MAX);
            if low == high {
                return None;
            }
        }
        while high - low > 1 {
            let middle = low + (high - low) / 2;
            if self
                .pattern_date(middle, anchor)
                .is_some_and(|date| date <= boundary)
            {
                low = middle;
            } else {
                high = middle;
            }
        }
        Some(high)
    }
    fn pattern_date(&self, index: u32, anchor: Date) -> Option<Date> {
        let every = u64::from(self.every.max(1));
        match self.unit {
            RepeatUnit::Day => anchor.checked_add_days(Days::new(u64::from(index) * every)),
            RepeatUnit::Weekday => {
                let anchor = if self.every == 1 && anchor.weekday().num_days_from_monday() >= 5 {
                    anchor.checked_add_days(Days::new(u64::from(
                        7 - anchor.weekday().num_days_from_monday(),
                    )))?
                } else {
                    anchor
                };
                if index == 0 {
                    return Some(anchor);
                }
                let weekday = anchor.weekday().num_days_from_monday();
                let mut date = if weekday >= 5 {
                    anchor.checked_sub_days(Days::new(u64::from(weekday - 4)))?
                } else {
                    anchor
                };
                let days = u64::from(index) * every;
                date = date.checked_add_days(Days::new(days / 5 * 7))?;
                for _ in 0..days % 5 {
                    date = date.succ_opt()?;
                    while date.weekday().num_days_from_monday() >= 5 {
                        date = date.succ_opt()?;
                    }
                }
                Some(date)
            }
            RepeatUnit::Week => self.week_date(index, anchor),
            RepeatUnit::Month | RepeatUnit::Year => {
                let first = self.month_date(
                    anchor.year(),
                    self.month.map(u32::from).unwrap_or(anchor.month()),
                    anchor,
                )?;
                let skip = u64::from(first < anchor);
                let step = (u64::from(index) + skip) * every;
                let (year, month) = if self.unit == RepeatUnit::Year {
                    (
                        i32::try_from(i64::from(anchor.year()) + i64::try_from(step).ok()?).ok()?,
                        self.month.map(u32::from).unwrap_or(anchor.month()),
                    )
                } else {
                    let month = i64::from(anchor.year()) * 12
                        + i64::from(anchor.month0())
                        + i64::try_from(step).ok()?;
                    (
                        i32::try_from(month.div_euclid(12)).ok()?,
                        month.rem_euclid(12) as u32 + 1,
                    )
                };
                self.month_date(year, month, anchor)
            }
        }
    }
    fn week_date(&self, index: u32, anchor: Date) -> Option<Date> {
        let fallback = [RepeatDay::from_date(anchor)];
        let days = if self.days_of_week.is_empty() {
            &fallback[..]
        } else {
            &self.days_of_week
        };
        let start = self.week_start.number();
        let anchor_offset = (anchor.weekday().num_days_from_sunday() + 7 - start) % 7;
        let mut offsets: Vec<_> = days
            .iter()
            .map(|day| (day.number() + 7 - start) % 7)
            .collect();
        offsets.sort_unstable();
        offsets.dedup();
        let first: Vec<_> = offsets
            .iter()
            .copied()
            .filter(|offset| *offset >= anchor_offset)
            .collect();
        let (cycle, offset) = if let Some(offset) = first.get(index as usize) {
            (0, *offset)
        } else {
            let rest = u64::from(index) - first.len() as u64;
            (
                rest / offsets.len() as u64 + 1,
                offsets[rest as usize % offsets.len()],
            )
        };
        let ordinal = i64::from(anchor.num_days_from_ce()) - i64::from(anchor_offset)
            + i64::try_from(cycle * u64::from(self.every.max(1)) * 7).ok()?
            + i64::from(offset);
        Date::from_num_days_from_ce_opt(i32::try_from(ordinal).ok()?)
    }
    fn month_date(&self, year: i32, month: u32, anchor: Date) -> Option<Date> {
        let Some(ordinal) = self.ordinal else {
            return clamped_day(
                year,
                month,
                self.day_of_month.map(u32::from).unwrap_or(anchor.day()),
            );
        };
        let first = Date::from_ymd_opt(year, month, 1)?;
        let last = clamped_day(year, month, 31)?;
        if self.relative_mode == RelativeMode::NthMatchingDay {
            let mut matches = (1..=last.day()).filter_map(|day| {
                let date = Date::from_ymd_opt(year, month, day)?;
                self.days_of_week
                    .contains(&RepeatDay::from_date(date))
                    .then_some(date)
            });
            return match ordinal {
                WeekIndex::First => matches.next(),
                WeekIndex::Second => matches.nth(1),
                WeekIndex::Third => matches.nth(2),
                WeekIndex::Fourth => matches.nth(3),
                WeekIndex::Last => matches.last(),
            };
        }
        self.days_of_week
            .iter()
            .filter_map(|weekday| {
                let number = weekday.number();
                let day = match ordinal {
                    WeekIndex::Last => {
                        last.day() - (last.weekday().num_days_from_sunday() + 7 - number) % 7
                    }
                    other => {
                        1 + (number + 7 - first.weekday().num_days_from_sunday()) % 7
                            + 7 * match other {
                                WeekIndex::First => 0,
                                WeekIndex::Second => 1,
                                WeekIndex::Third => 2,
                                WeekIndex::Fourth => 3,
                                WeekIndex::Last => unreachable!(),
                            }
                    }
                };
                Date::from_ymd_opt(year, month, day)
            })
            .min()
    }
}
fn clamped_day(year: i32, month: u32, day: u32) -> Option<Date> {
    (1..=day.min(31))
        .rev()
        .find_map(|day| Date::from_ymd_opt(year, month, day))
}

mod calculate;
mod labels;
mod validation;

use crate::Date;
use chrono::Datelike;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatUnit {
    Day,
    Weekday,
    Week,
    Month,
    Year,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatDay {
    #[default]
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl RepeatDay {
    pub const ALL: [Self; 7] = [
        Self::Sunday,
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
    ];
    pub fn from_date(date: Date) -> Self {
        Self::ALL[date.weekday().num_days_from_sunday() as usize]
    }
    pub fn number(self) -> u32 {
        Self::ALL.iter().position(|day| *day == self).unwrap() as u32
    }
    pub fn is_sunday(&self) -> bool {
        *self == Self::Sunday
    }
    pub fn short(self) -> &'static str {
        ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][self.number() as usize]
    }
    pub fn label(self) -> &'static str {
        [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ][self.number() as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekIndex {
    First,
    Second,
    Third,
    Fourth,
    Last,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RelativeMode {
    /// Graph: choose the earliest of the nth occurrences of each weekday.
    #[default]
    FirstMatch,
    /// Outlook: choose the nth date from the combined set of matching weekdays.
    NthMatchingDay,
}
impl RelativeMode {
    pub fn is_first_match(&self) -> bool {
        *self == Self::FirstMatch
    }
}
impl WeekIndex {
    pub const ALL: [Self; 5] = [
        Self::First,
        Self::Second,
        Self::Third,
        Self::Fourth,
        Self::Last,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Second => "second",
            Self::Third => "third",
            Self::Fourth => "fourth",
            Self::Last => "last",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum RepeatEnd {
    #[default]
    Never,
    On(Date),
    After(u32),
}
impl RepeatEnd {
    pub fn is_never(&self) -> bool {
        matches!(self, Self::Never)
    }
}

/// A civil-calendar rule. The anchor is the inclusive start of the series;
/// numbered ranges count scheduled occurrences, including skipped overdue dates.
/// Optional selectors default to the anchor's weekday/day/month for old saves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recurrence {
    pub unit: RepeatUnit,
    pub every: u16,
    #[serde(default)]
    pub anchor: Option<Date>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub days_of_week: Vec<RepeatDay>,
    #[serde(default, skip_serializing_if = "RepeatDay::is_sunday")]
    pub week_start: RepeatDay,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_of_month: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<WeekIndex>,
    #[serde(default, skip_serializing_if = "RelativeMode::is_first_match")]
    pub relative_mode: RelativeMode,
    #[serde(default, skip_serializing_if = "RepeatEnd::is_never")]
    pub end: RepeatEnd,
    /// Graph range timezone identity is retained; dates are not converted through UTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
}

impl Recurrence {
    pub fn new(unit: RepeatUnit, every: u16) -> Self {
        Self {
            unit,
            every: every.max(1),
            anchor: None,
            days_of_week: Vec::new(),
            week_start: RepeatDay::Sunday,
            day_of_month: None,
            month: None,
            ordinal: None,
            relative_mode: RelativeMode::FirstMatch,
            end: RepeatEnd::Never,
            time_zone: None,
        }
    }
    pub fn daily() -> Self {
        Self::new(RepeatUnit::Day, 1)
    }
    pub fn weekdays() -> Self {
        Self::new(RepeatUnit::Weekday, 1)
    }
    pub fn weekly() -> Self {
        Self::new(RepeatUnit::Week, 1)
    }
    pub fn monthly() -> Self {
        Self::new(RepeatUnit::Month, 1)
    }
    pub fn yearly() -> Self {
        Self::new(RepeatUnit::Year, 1)
    }
    pub fn same_pattern(&self, other: &Self) -> bool {
        let (mut left, mut right) = (self.clone(), other.clone());
        left.normalize();
        right.normalize();
        left.unit == right.unit
            && left.every == right.every
            && left.days_of_week == right.days_of_week
            && left.week_start == right.week_start
            && left.day_of_month == right.day_of_month
            && left.month == right.month
            && left.ordinal == right.ordinal
            && left.relative_mode == right.relative_mode
    }
    pub fn normalize(&mut self) {
        self.days_of_week.sort_unstable();
        self.days_of_week.dedup();
        if self.days_of_week.len() <= 1 || self.ordinal == Some(WeekIndex::First) {
            self.relative_mode = RelativeMode::FirstMatch;
        }
        if let Some(anchor) = self.anchor {
            if self.unit == RepeatUnit::Week && self.days_of_week == [RepeatDay::from_date(anchor)]
            {
                self.days_of_week.clear();
            }
            if matches!(self.unit, RepeatUnit::Month | RepeatUnit::Year)
                && self.ordinal.is_none()
                && self.day_of_month == Some(anchor.day() as u8)
            {
                self.day_of_month = None;
            }
            if self.unit == RepeatUnit::Year && self.month == Some(anchor.month() as u8) {
                self.month = None;
            }
        }
        self.time_zone = self
            .time_zone
            .take()
            .map(|zone| zone.trim().to_owned())
            .filter(|zone| !zone.is_empty());
    }
    /// An explicit due-date edit starts the schedule from that date. Preserve a
    /// selected multi-day/relative pattern; move an absolute selector with its date.
    pub fn reanchor(&mut self, date: Date) {
        self.anchor = Some(date);
        if self.ordinal.is_none() {
            if self.day_of_month.is_some() {
                self.day_of_month = Some(date.day() as u8);
            }
            if self.month.is_some() {
                self.month = Some(date.month() as u8);
            }
        }
        if self.unit == RepeatUnit::Week && self.days_of_week.len() == 1 {
            self.days_of_week[0] = RepeatDay::from_date(date);
        }
    }
    /// A task copy has an independent series, retaining the original calendar
    /// selectors even if its current occurrence is a clamped month-end date.
    pub fn restart_from(&mut self, date: Date) {
        if let Some(anchor) = self.anchor {
            if matches!(self.unit, RepeatUnit::Month | RepeatUnit::Year) && self.ordinal.is_none() {
                self.day_of_month.get_or_insert(anchor.day() as u8);
            }
            if self.unit == RepeatUnit::Year {
                self.month.get_or_insert(anchor.month() as u8);
            }
        }
        self.anchor = Some(date);
    }
}

use crate::Date;
use chrono::{Datelike, Duration, Months, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatUnit {
    Day,
    Weekday,
    Week,
    Month,
    Year,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recurrence {
    pub unit: RepeatUnit,
    pub every: u16,
    /// Original civil date preserves e.g. January 31 -> February 28 -> March 31.
    #[serde(default)]
    pub anchor: Option<Date>,
}

impl Recurrence {
    pub fn new(unit: RepeatUnit, every: u16) -> Self {
        Self {
            unit,
            every: every.max(1),
            anchor: None,
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

    /// Next occurrence strictly after both the current due date and completion date.
    /// Arithmetic jumps over missed periods rather than looping over years of overdue days.
    pub fn next_after(&self, current: Date, completed: Date) -> Option<Date> {
        let every = u32::from(self.every.max(1));
        let boundary = current.max(completed);
        let anchor = self.anchor.unwrap_or(current);
        match self.unit {
            RepeatUnit::Day | RepeatUnit::Week => {
                let days = i64::from(every) * if self.unit == RepeatUnit::Week { 7 } else { 1 };
                let passed = (boundary - anchor).num_days().max(0);
                anchor.checked_add_signed(Duration::days((passed / days + 1) * days))
            }
            RepeatUnit::Weekday => {
                let mut candidate = current;
                // Advance by every business days from the later date: weekday schedules have
                // no backlog and a Friday occurrence is never regenerated for the weekend.
                candidate = candidate.max(completed);
                candidate = match candidate.weekday() {
                    Weekday::Sat => candidate.checked_sub_signed(Duration::days(1))?,
                    Weekday::Sun => candidate.checked_sub_signed(Duration::days(2))?,
                    _ => candidate,
                };
                let full_weeks = every / 5;
                candidate =
                    candidate.checked_add_signed(Duration::days(i64::from(full_weeks * 7)))?;
                for _ in 0..(every % 5) {
                    candidate = candidate.succ_opt()?;
                    while matches!(candidate.weekday(), Weekday::Sat | Weekday::Sun) {
                        candidate = candidate.succ_opt()?;
                    }
                }
                while matches!(candidate.weekday(), Weekday::Sat | Weekday::Sun) {
                    candidate = candidate.succ_opt()?;
                }
                Some(candidate)
            }
            RepeatUnit::Month | RepeatUnit::Year => {
                let months = every * if self.unit == RepeatUnit::Year { 12 } else { 1 };
                let passed = (boundary.year() - anchor.year()) * 12 + boundary.month() as i32
                    - anchor.month() as i32;
                let mut step = (passed.max(0) as u32 / months).max(1);
                let mut candidate =
                    anchor.checked_add_months(Months::new(step.checked_mul(months)?))?;
                if candidate <= boundary {
                    step += 1;
                    candidate =
                        anchor.checked_add_months(Months::new(step.checked_mul(months)?))?;
                }
                Some(candidate)
            }
        }
    }

    pub fn label(&self) -> String {
        if self.every == 1 {
            return match self.unit {
                RepeatUnit::Day => "Daily",
                RepeatUnit::Weekday => "Weekdays",
                RepeatUnit::Week => "Weekly",
                RepeatUnit::Month => "Monthly",
                RepeatUnit::Year => "Yearly",
            }
            .into();
        }
        let unit = match self.unit {
            RepeatUnit::Day => "days",
            RepeatUnit::Weekday => "weekdays",
            RepeatUnit::Week => "weeks",
            RepeatUnit::Month => "months",
            RepeatUnit::Year => "years",
        };
        format!("Every {} {unit}", self.every)
    }
}

use super::{Recurrence, RelativeMode, RepeatEnd, RepeatUnit};

impl Recurrence {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.every == 0 {
            return Err("Repeat interval must be a positive whole number.");
        }
        if self.days_of_week.len() > 7
            || self
                .days_of_week
                .iter()
                .enumerate()
                .any(|(index, day)| self.days_of_week[..index].contains(day))
        {
            return Err("Choose each weekday at most once.");
        }
        if !self.days_of_week.is_empty()
            && !matches!(
                self.unit,
                RepeatUnit::Week | RepeatUnit::Month | RepeatUnit::Year
            )
        {
            return Err("This repeat unit does not use a weekday selection.");
        }
        if self.ordinal.is_some()
            && (!matches!(self.unit, RepeatUnit::Month | RepeatUnit::Year)
                || self.days_of_week.is_empty())
        {
            return Err("Choose at least one weekday for a relative monthly or yearly schedule.");
        }
        if self.relative_mode != RelativeMode::FirstMatch && self.ordinal.is_none() {
            return Err("A relative weekday mode requires a monthly or yearly weekday pattern.");
        }
        if matches!(self.unit, RepeatUnit::Month | RepeatUnit::Year)
            && self.ordinal.is_none()
            && !self.days_of_week.is_empty()
        {
            return Err("Choose which week of the month the selected weekdays use.");
        }
        if self
            .day_of_month
            .is_some_and(|day| !(1..=31).contains(&day))
            || (self.day_of_month.is_some()
                && (!matches!(self.unit, RepeatUnit::Month | RepeatUnit::Year)
                    || self.ordinal.is_some()))
        {
            return Err("Choose a day of the month from 1 to 31.");
        }
        if self.month.is_some_and(|month| !(1..=12).contains(&month))
            || (self.month.is_some() && self.unit != RepeatUnit::Year)
        {
            return Err("Choose a month from January to December for a yearly schedule.");
        }
        match self.end {
            RepeatEnd::After(count) if count == 0 || count > i32::MAX as u32 => {
                return Err("Enter a positive occurrence count no greater than 2147483647.");
            }
            RepeatEnd::On(date) if self.anchor.is_some_and(|anchor| date < anchor) => {
                return Err("The repeat end date must be on or after the start date.");
            }
            _ => {}
        }
        if self.time_zone.as_ref().is_some_and(|zone| {
            zone.is_empty() || zone.len() > 128 || zone.chars().any(char::is_control)
        }) {
            return Err("Enter a valid time zone name, such as Europe/Warsaw or UTC.");
        }
        Ok(())
    }
}

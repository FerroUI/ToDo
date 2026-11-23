use super::{Recurrence, RelativeMode, RepeatEnd, RepeatUnit};

impl Recurrence {
    pub fn label(&self) -> String {
        let mut label = if self.every == 1 {
            match self.unit {
                RepeatUnit::Day => "Daily",
                RepeatUnit::Weekday => "Weekdays",
                RepeatUnit::Week => "Weekly",
                RepeatUnit::Month => "Monthly",
                RepeatUnit::Year => "Yearly",
            }
            .to_owned()
        } else {
            let unit = match self.unit {
                RepeatUnit::Day => "days",
                RepeatUnit::Weekday => "weekdays",
                RepeatUnit::Week => "weeks",
                RepeatUnit::Month => "months",
                RepeatUnit::Year => "years",
            };
            format!("Every {} {unit}", self.every)
        };
        let days = self
            .days_of_week
            .iter()
            .map(|day| day.short())
            .collect::<Vec<_>>()
            .join(", ");
        if let Some(ordinal) = self.ordinal {
            if self.relative_mode == RelativeMode::NthMatchingDay {
                label.push_str(&format!(
                    " on the {} matching day ({days})",
                    ordinal.label()
                ));
            } else {
                label.push_str(&format!(" on the {} {days}", ordinal.label()));
            }
        } else if !days.is_empty() {
            label.push_str(&format!(" on {days}"));
        } else if let Some(day) = self.day_of_month {
            label.push_str(&format!(" on day {day}"));
        }
        if let Some(month) = self.month {
            if let Some(name) = [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ]
            .get(month.saturating_sub(1) as usize)
            {
                label.push_str(&format!(" of {name}"));
            }
        }
        match self.end {
            RepeatEnd::Never => {}
            RepeatEnd::On(date) => label.push_str(&format!(" · Until {date}")),
            RepeatEnd::After(count) => label.push_str(&format!(
                " · {count} {}",
                if count == 1 {
                    "occurrence"
                } else {
                    "occurrences"
                }
            )),
        }
        label
    }
}

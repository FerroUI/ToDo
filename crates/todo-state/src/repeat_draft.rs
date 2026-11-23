use crate::{AppViewModel, Panel};
use todo_domain::{
    Action, Date, Id, Recurrence, RelativeMode, RepeatDay, RepeatEnd, RepeatUnit, Task, WeekIndex,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatDraft {
    pub unit: RepeatUnit,
    pub interval: String,
    pub start: String,
    pub days: Vec<RepeatDay>,
    pub week_start: RepeatDay,
    pub relative: bool,
    pub ordinal: WeekIndex,
    pub relative_mode: RelativeMode,
    pub day: String,
    pub month: u8,
    /// 0 = never, 1 = on date, 2 = after count.
    pub ending: u8,
    pub until: String,
    pub count: String,
    pub time_zone: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatValidation {
    pub field: &'static str,
    pub message: String,
}
impl RepeatValidation {
    fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl RepeatDraft {
    pub fn from_task(task: Option<&Task>, today: Date, monday: bool) -> Self {
        use chrono::Datelike;
        let rule = task
            .and_then(|task| task.recurrence.clone())
            .unwrap_or_else(Recurrence::daily);
        let anchor = rule
            .anchor
            .or_else(|| task.and_then(|task| task.due_date))
            .unwrap_or(today);
        let has_rule = task.is_some_and(|task| task.recurrence.is_some());
        Self {
            unit: rule.unit,
            interval: rule.every.to_string(),
            start: anchor.to_string(),
            days: if rule.days_of_week.is_empty() {
                vec![RepeatDay::from_date(anchor)]
            } else {
                rule.days_of_week
            },
            week_start: if has_rule {
                rule.week_start
            } else if monday {
                RepeatDay::Monday
            } else {
                RepeatDay::Sunday
            },
            relative: rule.ordinal.is_some(),
            ordinal: rule.ordinal.unwrap_or(WeekIndex::First),
            relative_mode: rule.relative_mode,
            day: rule
                .day_of_month
                .map(u32::from)
                .unwrap_or(anchor.day())
                .to_string(),
            month: rule.month.unwrap_or(anchor.month() as u8),
            ending: match rule.end {
                RepeatEnd::Never => 0,
                RepeatEnd::On(_) => 1,
                RepeatEnd::After(_) => 2,
            },
            until: if let RepeatEnd::On(date) = rule.end {
                date
            } else {
                anchor
            }
            .to_string(),
            count: if let RepeatEnd::After(count) = rule.end {
                count
            } else {
                10
            }
            .to_string(),
            time_zone: rule.time_zone.unwrap_or_default(),
        }
    }

    pub fn rule(&self) -> Result<Recurrence, RepeatValidation> {
        let interval = self
            .interval
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                RepeatValidation::new(
                    "RepeatInterval",
                    "Enter a positive whole number for the repeat interval.",
                )
            })?;
        let anchor = parse_date(&self.start, "RepeatStart")?;
        let mut rule = Recurrence::new(self.unit, interval);
        rule.anchor = Some(anchor);
        if self.unit == RepeatUnit::Week
            || (matches!(self.unit, RepeatUnit::Month | RepeatUnit::Year) && self.relative)
        {
            if self.days.is_empty() {
                return Err(RepeatValidation::new(
                    "RepeatDays",
                    "Choose at least one weekday.",
                ));
            }
            rule.days_of_week = self.days.clone();
        }
        if self.unit == RepeatUnit::Week {
            rule.week_start = self.week_start;
        }
        if matches!(self.unit, RepeatUnit::Month | RepeatUnit::Year) {
            if self.relative {
                rule.ordinal = Some(self.ordinal);
                rule.relative_mode = self.relative_mode;
            } else {
                rule.day_of_month = Some(
                    self.day
                        .trim()
                        .parse::<u8>()
                        .ok()
                        .filter(|day| (1..=31).contains(day))
                        .ok_or_else(|| {
                            RepeatValidation::new(
                                "RepeatMonthDay",
                                "Choose a day of the month from 1 to 31.",
                            )
                        })?,
                );
            }
        }
        if self.unit == RepeatUnit::Year {
            rule.month = Some(self.month);
        }
        rule.end = match self.ending {
            0 => RepeatEnd::Never,
            1 => RepeatEnd::On(parse_date(&self.until, "RepeatUntil")?),
            2 => RepeatEnd::After(
                self.count
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|count| *count > 0 && *count <= i32::MAX as u32)
                    .ok_or_else(|| {
                        RepeatValidation::new(
                            "RepeatCount",
                            "Enter a positive occurrence count no greater than 2147483647.",
                        )
                    })?,
            ),
            _ => {
                return Err(RepeatValidation::new(
                    "RepeatEnding",
                    "Choose when the repeat schedule ends.",
                ));
            }
        };
        rule.time_zone =
            (!self.time_zone.trim().is_empty()).then(|| self.time_zone.trim().to_owned());
        rule.normalize();
        rule.validate().map_err(|message| {
            RepeatValidation::new(
                if message.contains("end date") {
                    "RepeatUntil"
                } else if message.contains("time zone") {
                    "RepeatTimeZone"
                } else {
                    "RepeatInterval"
                },
                message,
            )
        })?;
        if rule.first_occurrence(anchor).is_none() {
            return Err(RepeatValidation::new(
                "RepeatUntil",
                "The range does not contain an occurrence. Choose a later end date.",
            ));
        }
        Ok(rule)
    }

    pub fn preview(&self, today: Date) -> String {
        let Ok(rule) = self.rule() else {
            return "Complete the schedule to see upcoming dates.".into();
        };
        let Some(mut date) = rule.first_occurrence(today) else {
            return "No occurrences in this range.".into();
        };
        if date < today {
            let boundary = today.pred_opt().unwrap_or(today);
            let Some(next) = rule.next_after(boundary, boundary) else {
                return "This schedule has no remaining occurrences.".into();
            };
            date = next;
        }
        let mut dates = vec![date.format("%a, %-d %b %Y").to_string()];
        for _ in 0..2 {
            let Some(next) = rule.next_after(date, date) else {
                break;
            };
            date = next;
            dates.push(date.format("%a, %-d %b %Y").to_string());
        }
        format!("Upcoming: {}", dates.join(" · "))
    }
}

fn parse_date(text: &str, field: &'static str) -> Result<Date, RepeatValidation> {
    text.trim()
        .parse()
        .map_err(|_| RepeatValidation::new(field, "Enter a calendar date in YYYY-MM-DD format."))
}

impl AppViewModel {
    pub fn save_repeat_draft(&self, id: Id, draft: &RepeatDraft) -> Result<(), RepeatValidation> {
        self.save_bulk_repeat_draft(&[id], draft)
    }

    pub fn save_bulk_repeat_draft(
        &self,
        ids: &[Id],
        draft: &RepeatDraft,
    ) -> Result<(), RepeatValidation> {
        let rule = draft.rule()?;
        let state = self.state();
        let mut actions = Vec::new();
        let mut unique = std::collections::BTreeSet::new();
        for &id in ids {
            if !unique.insert(id) {
                continue;
            }
            let task = state.task(id).ok_or_else(|| {
                RepeatValidation::new("RepeatInterval", "A selected task no longer exists.")
            })?;
            if task.is_planner() {
                return Err(RepeatValidation::new(
                    "RepeatInterval",
                    "Open this task in Planner to change its schedule.",
                ));
            }
            let retain_occurrence = task.recurrence.as_ref().is_some_and(|existing| {
                existing.same_pattern(&rule) && existing.anchor == rule.anchor
            });
            let due = if retain_occurrence {
                task.due_date
                    .or_else(|| rule.first_occurrence(self.today()))
            } else {
                rule.first_occurrence(self.today())
            };
            if task
                .start_date
                .zip(due)
                .is_some_and(|(start, due)| start > due)
            {
                return Err(RepeatValidation::new(
                    "RepeatStart",
                    "The first due date must be on or after the task's start date.",
                ));
            }
            if task.recurrence.as_ref() == Some(&rule) && task.due_date == due {
                continue;
            }
            // Clear the previous rule in the same transaction, so an old end
            // cannot reject a new start and the requested range stays exact.
            actions.extend([
                Action::SetRecurrence {
                    id,
                    recurrence: None,
                },
                Action::SetDueDate { id, date: due },
                Action::SetRecurrence {
                    id,
                    recurrence: Some(rule.clone()),
                },
            ]);
        }
        drop(state);
        if actions.is_empty() {
            self.show_panel(Panel::None);
            return Ok(());
        }
        self.dispatch_after(Action::Batch(actions), |vm, _| vm.show_panel(Panel::None));
        Ok(())
    }
}

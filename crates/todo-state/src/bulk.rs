use crate::{parse_reminder_time, AppViewModel, HostRequest, Panel};
use todo_domain::{Action, Date, Id, Reminder};

impl AppViewModel {
    pub fn save_tasks_date(
        &self,
        ids: &[Id],
        date: Date,
        reminder_time: Option<&str>,
    ) -> Result<(), String> {
        if ids.is_empty() {
            return Err("Select at least one task.".into());
        }
        let minutes = reminder_time.map(parse_reminder_time).transpose()?;
        let state = self.state();
        let mut actions = Vec::with_capacity(ids.len());
        for id in ids {
            let task = state.task(*id).ok_or("A selected task no longer exists.")?;
            if minutes.is_some() && task.is_planner() {
                return Err("Open Planner to change reminders for its tasks.".into());
            }
            if minutes.is_none() {
                if task.start_date.is_some_and(|start| start > date) {
                    return Err(
                        "The due date must be on or after every selected task's start date.".into(),
                    );
                }
                if let Some(mut rule) = task.recurrence.clone() {
                    rule.reanchor(date);
                    rule.validate().map_err(str::to_owned)?;
                }
            }
            actions.push(if let Some(minutes_after_midnight) = minutes {
                Action::SetReminder {
                    id: *id,
                    reminder: Some(Reminder {
                        date,
                        minutes_after_midnight,
                        notified: false,
                    }),
                }
            } else {
                Action::SetDueDate {
                    id: *id,
                    date: Some(date),
                }
            });
        }
        drop(state);
        self.dispatch_after(Action::Batch(actions), move |vm, _| {
            if minutes.is_some() {
                vm.request(HostRequest::RequestNotifications);
            }
            vm.show_panel(Panel::None);
        });
        Ok(())
    }

    pub fn set_tasks_category(&self, ids: &[Id], name: &str, add: bool) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a category name.".into());
        }
        if ids.is_empty() {
            return Err("Select at least one task.".into());
        }
        let state = self.state();
        let mut actions = Vec::with_capacity(ids.len());
        for id in ids {
            let task = state.task(*id).ok_or("A selected task no longer exists.")?;
            if task.is_planner() {
                return Err("Open Planner to change categories for its tasks.".into());
            }
            let mut tags = task.tags.clone();
            if add {
                if !tags.iter().any(|tag| tag.eq_ignore_ascii_case(name)) {
                    tags.push(name.to_owned());
                }
            } else {
                tags.retain(|tag| !tag.eq_ignore_ascii_case(name));
            }
            actions.push(Action::SetTags { id: *id, tags });
        }
        drop(state);
        self.dispatch(Action::Batch(actions));
        Ok(())
    }
}

use crate::{AppViewModel, HostRequest, Panel};
use todo_domain::{Action, Date, Id, Recurrence, Reminder, RepeatUnit};

/// Shared by the quick menu and custom date/time editor, including midnight rollover.
pub fn reminder_in_one_hour(today: Date, minutes: u16) -> Option<Reminder> {
    let later = minutes.min(1439) + 60;
    Some(Reminder {
        date: if later >= 1440 {
            today.succ_opt()?
        } else {
            today
        },
        minutes_after_midnight: later % 1440,
        notified: false,
    })
}

pub fn parse_reminder_time(value: &str) -> Result<u16, &'static str> {
    let invalid = "Enter a time in 24-hour format, such as 09:30.";
    let (hour, minute) = value.trim().split_once(':').ok_or(invalid)?;
    if hour.is_empty()
        || hour.len() > 2
        || minute.len() != 2
        || !hour
            .bytes()
            .chain(minute.bytes())
            .all(|c| c.is_ascii_digit())
    {
        return Err(invalid);
    }
    let hour = hour.parse::<u16>().map_err(|_| invalid)?;
    let minute = minute.parse::<u16>().map_err(|_| invalid)?;
    if hour > 23 || minute > 59 {
        return Err(invalid);
    }
    Ok(hour * 60 + minute)
}

impl AppViewModel {
    pub fn save_task_date(
        &self,
        id: Id,
        date: Date,
        reminder_time: Option<&str>,
    ) -> Result<(), String> {
        if self.state().task(id).is_none() {
            return Err("This task no longer exists.".into());
        }
        let action = if let Some(time) = reminder_time {
            Action::SetReminder {
                id,
                reminder: Some(Reminder {
                    date,
                    minutes_after_midnight: parse_reminder_time(time)?,
                    notified: false,
                }),
            }
        } else {
            Action::SetDueDate {
                id,
                date: Some(date),
            }
        };
        let request_notifications = reminder_time.is_some();
        self.dispatch_after(action, move |vm, _| {
            if request_notifications {
                vm.request(HostRequest::RequestNotifications);
            }
            vm.show_panel(Panel::None);
        });
        Ok(())
    }

    pub fn save_custom_repeat(
        &self,
        id: Id,
        interval: &str,
        unit: RepeatUnit,
    ) -> Result<(), String> {
        let every = interval
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or("Enter a positive whole number for the repeat interval.")?;
        if self.state().task(id).is_none() {
            return Err("This task no longer exists.".into());
        }
        self.dispatch_after(
            Action::SetRecurrence {
                id,
                recurrence: Some(Recurrence::new(unit, every)),
            },
            |vm, _| vm.show_panel(Panel::None),
        );
        Ok(())
    }
}

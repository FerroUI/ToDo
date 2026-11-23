mod commands;
mod headless;
mod platform;
mod scheduler;
pub mod templates;

use chrono::{DateTime, Duration, Local, LocalResult, NaiveDateTime, TimeZone, Timelike};
#[cfg(any(target_os = "macos", windows))]
pub(crate) use commands::output;
pub use headless::run as run_headless;
pub use scheduler::{Scheduler, SchedulerStatus};
use std::path::{Path, PathBuf};
use todo_domain::{Action, AppState, Date, Id};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registration {
    pub id: String,
    pub executable: PathBuf,
    pub directory: PathBuf,
    pub next: NaiveDateTime,
}

pub fn identity(directory: &Path) -> String {
    // Stable across process/release versions and independent for custom data directories.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in directory.as_os_str().as_encoded_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
    }
    format!("ferroui-todo-reminders-{hash:016x}")
}

pub fn next_reminder(state: &AppState, now: NaiveDateTime) -> Option<NaiveDateTime> {
    if !state.settings.background_reminders {
        return None;
    }
    let next = state
        .tasks
        .iter()
        .filter(|task| !task.completed)
        .filter_map(|task| task.reminder.as_ref())
        .filter(|reminder| !reminder.notified)
        .filter_map(|reminder| {
            reminder.date.and_hms_opt(
                u32::from(reminder.minutes_after_midnight / 60),
                u32::from(reminder.minutes_after_midnight % 60),
                0,
            )
        })
        .min()?;
    // Registration takes time. Overdue work uses the next minute, then the helper
    // catches up every due reminder in one invocation rather than firing a busy loop.
    Some(if next <= now {
        (now.checked_add_signed(Duration::minutes(1))?).with_second(0)?
    } else {
        next
    })
}

pub fn registration(
    state: &AppState,
    directory: &Path,
    executable: &Path,
    now: DateTime<Local>,
) -> Result<Option<Registration>, String> {
    let Some(mut next) = next_reminder(state, now.naive_local()) else {
        return Ok(None);
    };
    // A civil time in a daylight-saving gap must advance to the first valid minute.
    for _ in 0..=180 {
        if !matches!(Local.from_local_datetime(&next), LocalResult::None) {
            return Ok(Some(Registration {
                id: identity(directory),
                executable: executable.to_owned(),
                directory: directory.to_owned(),
                next,
            }));
        }
        next = next
            .checked_add_signed(Duration::minutes(1))
            .ok_or("Reminder date is out of range")?;
    }
    Err("The next reminder is outside the supported local clock range".into())
}

/// The OS acceptance is followed by a durable acknowledgement. A failed delivery
/// remains pending; if a crash occurs between acceptance and saving, retry can
/// repeat that notification, but never silently discards it.
pub fn deliver_due(
    state: &mut AppState,
    today: Date,
    minutes: u16,
    mut deliver: impl FnMut(Id, &str) -> Result<(), String>,
    mut save: impl FnMut(&AppState) -> Result<(), String>,
) -> Result<usize, String> {
    if !state.settings.background_reminders {
        return Ok(0);
    }
    let due: Vec<_> = state
        .reminders_due(today, minutes)
        .into_iter()
        .map(|task| (task.id, task.title.clone()))
        .collect();
    let mut delivered = 0;
    let mut first_error = None;
    for (id, title) in due {
        if let Err(error) = deliver(id, &title) {
            first_error.get_or_insert(error);
            continue;
        }
        let mut transaction = state.clone();
        let mut reminder = transaction
            .task(id)
            .and_then(|task| task.reminder.clone())
            .ok_or("The reminder disappeared")?;
        reminder.notified = true;
        transaction
            .apply(
                Action::SetReminder {
                    id,
                    reminder: Some(reminder),
                },
                today,
            )
            .map_err(|error| error.to_string())?;
        save(&transaction)?;
        *state = transaction;
        delivered += 1;
    }
    if let Some(error) = first_error {
        Err(error)
    } else {
        Ok(delivered)
    }
}

#[cfg(test)]
mod tests;

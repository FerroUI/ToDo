use super::{deliver_due, platform, registration};
use crate::persistence;
use chrono::{Local, Timelike};
use std::path::{Path, PathBuf};

pub fn run(mode: &str, args: &[String]) -> i32 {
    match execute(mode, args) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn execute(mode: &str, args: &[String]) -> Result<(), String> {
    let directory = match args {
        [] => persistence::data_directory(),
        [flag, path] if flag == "--data-dir" => PathBuf::from(path),
        _ => return Err("Expected --data-dir followed by the To Do data directory".into()),
    };
    if mode == "--disable-background-reminders" && !directory.exists() {
        return Ok(());
    }
    let directory = directory
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let mut instance = persistence::try_lock_directory(&directory)?;
    // The macOS re-registration child waits for its scheduled parent to release
    // the lock. A newly opened UI still owns all mutation and reconciliation.
    if mode == "--reconcile-reminders" {
        for _ in 0..100 {
            if instance.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            instance = persistence::try_lock_directory(&directory)?;
        }
    }
    let Some(_instance) = instance else {
        return if mode == "--disable-background-reminders" {
            Err("Close To Do before removing its background reminder registration".into())
        } else {
            Ok(())
        };
    };
    let result = (|| {
        if mode == "--disable-background-reminders" && !directory.join("tasks.json").exists() {
            return platform::reconcile(&directory, None);
        }
        let mut state = persistence::load_strict(&directory)?;
        if mode == "--disable-background-reminders" {
            state.settings.background_reminders = false;
            persistence::save(
                &directory,
                &state.to_json().map_err(|error| error.to_string())?,
            )?;
            return platform::reconcile(&directory, None);
        }
        let now = Local::now();
        let delivery = if mode == "--deliver-reminders" {
            deliver_due(
                &mut state,
                now.date_naive(),
                (now.hour() * 60 + now.minute()) as u16,
                |_, title| crate::notification_delivery::deliver("Task reminder", title),
                |state| {
                    persistence::save(
                        &directory,
                        &state.to_json().map_err(|error| error.to_string())?,
                    )
                },
            )
        } else {
            Ok(0)
        };
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        #[cfg(target_os = "macos")]
        if mode == "--deliver-reminders" {
            // Replacing a running LaunchAgent with bootout would kill this process.
            // Its child has an independent process group and takes the directory lock
            // after we leave, then loads the next nearest event (or removes the job).
            use std::os::unix::process::CommandExt;
            use std::process::{Command, Stdio};
            Command::new(&executable)
                .arg("--reconcile-reminders")
                .arg("--data-dir")
                .arg(&directory)
                .process_group(0)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| error.to_string())?;
            delivery?;
            return Ok(());
        }
        let plan = registration(&state, &directory, &executable, Local::now())?;
        platform::reconcile(&directory, plan.as_ref())?;
        delivery?;
        Ok(())
    })();
    if let Err(error) = &result {
        let _ = record_error(&directory, error);
    } else if mode != "--reconcile-reminders" {
        let _ = std::fs::remove_file(directory.join("background-reminders-error.json"));
    }
    result
}

pub(crate) fn record_error(directory: &Path, error: &str) -> Result<(), String> {
    platform::write_atomic(
        &directory.join("background-reminders-error.json"),
        &serde_json::json!({"at":Local::now().to_rfc3339(),"message":error}).to_string(),
    )
}

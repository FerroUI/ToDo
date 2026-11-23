use super::*;
use todo_domain::{Reminder, Task, TASKS_LIST};

fn state() -> AppState {
    let mut state = AppState::new();
    state.settings.background_reminders = true;
    let day = "2026-11-23".parse().unwrap();
    for (id, minutes) in [(2, 570), (3, 540), (4, 555)] {
        let mut task = Task::new(id, TASKS_LIST, format!("Task {id}"), day);
        task.reminder = Some(Reminder {
            date: day,
            minutes_after_midnight: minutes,
            notified: false,
        });
        state.tasks.push(task);
    }
    state.next_id = 5;
    state
}
fn now() -> NaiveDateTime {
    "2026-11-23T09:15:00".parse().unwrap()
}
fn plan() -> Registration {
    Registration {
        id: "ferroui-todo-reminders-123".into(),
        executable: "/home/user/To Do $5%/todo\"app".into(),
        directory: "/home/user/A&B <tasks>".into(),
        next: now(),
    }
}

#[test]
fn nearest_event_ignores_completed_and_delivered_tasks_and_catches_up_overdue_work() {
    let mut document = state();
    assert_eq!(
        next_reminder(&document, now()).unwrap(),
        "2026-11-23T09:16:00".parse::<NaiveDateTime>().unwrap()
    );
    document.tasks[1].completed = true;
    document.tasks[2].reminder.as_mut().unwrap().notified = true;
    assert_eq!(
        next_reminder(&document, now()).unwrap(),
        "2026-11-23T09:30:00".parse::<NaiveDateTime>().unwrap()
    );
    document.settings.background_reminders = false;
    assert!(next_reminder(&document, now()).is_none());
}

#[test]
fn accepted_notifications_are_durably_acknowledged_and_restart_does_not_repeat_them() {
    let mut document = state();
    let mut titles = Vec::new();
    let mut persisted = Vec::new();
    assert_eq!(
        deliver_due(
            &mut document,
            now().date(),
            555,
            |id, title| {
                titles.push((id, title.to_owned()));
                Ok(())
            },
            |state| {
                persisted.push(state.to_json().unwrap());
                Ok(())
            }
        )
        .unwrap(),
        2
    );
    assert_eq!(titles.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [3, 4]);
    let mut restarted = AppState::from_json(persisted.last().unwrap()).unwrap();
    assert_eq!(
        deliver_due(
            &mut restarted,
            now().date(),
            555,
            |_, _| panic!("must not repeat"),
            |_| panic!("no new save")
        )
        .unwrap(),
        0
    );
    assert_eq!(next_reminder(&restarted, now()).unwrap().time().hour(), 9);
}

#[test]
fn failed_delivery_or_persistence_never_loses_a_pending_reminder() {
    let mut document = state();
    assert!(deliver_due(
        &mut document,
        now().date(),
        555,
        |id, _| if id == 3 {
            Err("denied".into())
        } else {
            Ok(())
        },
        |_| Ok(())
    )
    .is_err());
    assert!(
        !document
            .task(3)
            .unwrap()
            .reminder
            .as_ref()
            .unwrap()
            .notified
    );
    assert!(
        document
            .task(4)
            .unwrap()
            .reminder
            .as_ref()
            .unwrap()
            .notified
    );
    let before = document.clone();
    assert!(deliver_due(
        &mut document,
        now().date(),
        555,
        |_, _| Ok(()),
        |_| Err("disk full".into())
    )
    .is_err());
    assert_eq!(document, before);
}

#[test]
fn service_templates_quote_paths_as_data_and_never_require_a_password_or_elevation() {
    let linux = templates::linux_service(&plan()).unwrap();
    assert!(linux.contains("$$5%%"));
    assert!(linux.contains("todo\\\"app\" --deliver-reminders"));
    let timer = templates::linux_timer(&plan());
    assert!(timer.contains("Persistent=true"));
    assert!(timer.contains("OnTimezoneChange=true"));
    let mac = templates::macos_agent(&plan()).unwrap();
    assert!(mac.contains("A&amp;B &lt;tasks&gt;"));
    assert!(mac.contains("<key>RunAtLoad</key><true/>"));
    let windows = templates::windows_task(&plan(), "S-1-5-21-123-456-1001").unwrap();
    assert!(windows.contains("<LogonType>InteractiveToken</LogonType>"));
    assert!(windows.contains("<StartWhenAvailable>true</StartWhenAvailable>"));
    assert!(windows.contains("<RunLevel>LeastPrivilege</RunLevel>"));
    assert!(!windows.contains("Password"));
    assert!(templates::windows_task(&plan(), "bad</UserId>").is_err());
    let mut invalid = plan();
    invalid.directory = PathBuf::from("/tmp/injected\nExecStart=evil");
    assert!(templates::linux_service(&invalid).is_err());
}

#[test]
fn task_identity_is_stable_per_directory_and_windows_quotes_trailing_slashes() {
    assert_eq!(identity(Path::new("/one")), identity(Path::new("/one")));
    assert_ne!(identity(Path::new("/one")), identity(Path::new("/two")));
    assert_eq!(
        templates::windows_argument("C:\\Tasks\\"),
        "\"C:\\Tasks\\\\\""
    );
    assert_eq!(templates::windows_argument("a\"b"), "\"a\\\"b\"");
}

#[test]
fn macos_login_catch_up_settles_and_failed_one_shots_rearm_without_run_at_load_loops() {
    let mut future = plan();
    future.next = "2026-11-23T10:30:00".parse().unwrap();
    let initial = templates::macos_installation(&future, false, None)
        .unwrap()
        .unwrap();
    assert!(initial.bootstrap.contains("<key>RunAtLoad</key><true/>"));
    // The RunAtLoad helper finds no due tasks. Its same-plan reconciliation is inert.
    assert!(
        templates::macos_installation(&future, true, Some(&initial.persistent))
            .unwrap()
            .is_none()
    );

    let document = state();
    let mut retry = future.clone();
    retry.next = next_reminder(&document, "2026-11-23T10:31:00".parse().unwrap()).unwrap();
    let rearmed = templates::macos_installation(&retry, true, Some(&initial.persistent))
        .unwrap()
        .unwrap();
    assert_eq!(
        retry.next,
        "2026-11-23T10:32:00".parse::<NaiveDateTime>().unwrap()
    );
    assert!(rearmed.bootstrap.contains("<key>RunAtLoad</key><false/>"));
    assert!(rearmed.persistent.contains("<key>RunAtLoad</key><true/>"));
    assert!(
        templates::macos_installation(&retry, true, Some(&rearmed.persistent))
            .unwrap()
            .is_none()
    );
    // At the next login the persisted plan still catches up any reminders missed offline.
    assert!(
        templates::macos_installation(&retry, false, Some(&rearmed.persistent))
            .unwrap()
            .unwrap()
            .bootstrap
            .contains("<key>RunAtLoad</key><true/>")
    );
}

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "todo-reminder-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn args(&self) -> Vec<String> {
        vec!["--data-dir".into(), self.0.to_str().unwrap().into()]
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_live_application_lock_prevents_scheduled_reads_writes_and_notification_delivery() {
    let directory = Directory::new();
    std::fs::write(
        directory.0.join("tasks.json"),
        "corrupt but owned by the live application",
    )
    .unwrap();
    let _lock = crate::persistence::lock_directory(&directory.0).unwrap();
    assert_eq!(run_headless("--deliver-reminders", &directory.args()), 0);
    assert!(!directory.0.join("background-reminders-error.json").exists());
    assert_eq!(
        std::fs::read_to_string(directory.0.join("tasks.json")).unwrap(),
        "corrupt but owned by the live application"
    );
}

#[test]
fn corrupt_closed_documents_are_reported_without_replacing_them_with_empty_tasks() {
    let directory = Directory::new();
    std::fs::write(directory.0.join("tasks.json"), "{not a task document").unwrap();
    assert_eq!(run_headless("--deliver-reminders", &directory.args()), 1);
    assert_eq!(
        std::fs::read_to_string(directory.0.join("tasks.json")).unwrap(),
        "{not a task document"
    );
    let diagnostic =
        std::fs::read_to_string(directory.0.join("background-reminders-error.json")).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&diagnostic).unwrap()["message"].is_string());
}

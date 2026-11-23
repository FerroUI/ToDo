use todo_domain::{Action, AppState, Date, Reminder, Settings, TaskEntry, View};

fn date(value: &str) -> Date {
    Date::parse_from_str(value, "%Y-%m-%d").unwrap()
}

fn parse(value: &str) -> TaskEntry {
    TaskEntry::parse_english(value, date("2026-12-31"), 23 * 60 + 30, false)
}

#[test]
fn date_suffixes_use_local_calendar_and_validate_leap_days() {
    for (title, due) in [
        ("Send report TODAY", "2026-12-31"),
        ("Buy milk tomorrow", "2027-01-01"),
        ("Plan next week", "2027-01-04"),
        ("Call on Friday", "2027-01-01"),
        ("Call Thursday", "2026-12-31"),
        ("Call next Thursday", "2027-01-07"),
        ("Review due on 2028-02-29", "2028-02-29"),
    ] {
        let entry = parse(title);
        assert_eq!(entry.title, title);
        assert_eq!(entry.due_date, Some(date(due)), "{title}");
        assert_eq!(entry.reminder, None);
    }
    assert_eq!(parse("Review 2027-02-29").due_date, None);
    let monday = date("2027-01-04");
    assert_eq!(
        TaskEntry::parse_english("Plan next week", monday, 0, false).due_date,
        Some(date("2027-01-11"))
    );
}

#[test]
fn reminder_clock_forms_do_not_silently_shift_a_past_time_to_tomorrow() {
    for (title, day, minutes) in [
        ("Call tomorrow at 3pm", "2027-01-01", 15 * 60),
        ("Call Friday 3 PM", "2027-01-01", 15 * 60),
        ("Call at 15:30", "2026-12-31", 15 * 60 + 30),
        ("Call today 12am", "2026-12-31", 0),
        ("Call tomorrow 12:05pm", "2027-01-01", 12 * 60 + 5),
        ("Call at noon", "2026-12-31", 12 * 60),
        ("Call tomorrow midnight", "2027-01-01", 0),
    ] {
        let entry = parse(title);
        assert_eq!(entry.title, title);
        assert_eq!(
            entry.due_date, None,
            "Clock expressions set a reminder, not an extra due date"
        );
        assert_eq!(
            entry.reminder,
            Some(Reminder {
                date: date(day),
                minutes_after_midnight: minutes,
                notified: false,
            }),
            "{title}"
        );
    }
}

#[test]
fn explicit_relative_minutes_cross_midnight_without_using_utc() {
    let entry = parse("Check oven in 90 minutes");
    assert_eq!(
        entry.reminder,
        Some(Reminder {
            date: date("2027-01-01"),
            minutes_after_midnight: 60,
            notified: false,
        })
    );
    assert_eq!(
        parse("Call in 2 hours")
            .reminder
            .unwrap()
            .minutes_after_midnight,
        90
    );
    assert_eq!(
        TaskEntry::parse_english("Call in 10 minutes", date("2026-12-31"), 1440, false).reminder,
        None
    );
}

#[test]
fn ambiguous_and_invalid_expressions_remain_uninterpreted() {
    for title in [
        "Call at 3",
        "Call at 0pm",
        "Call at 13pm",
        "Call at 24:00",
        "Call at 12:60",
        "Call at 3:5pm",
        "Call tomorrow at 25:00",
        "Call on 2027-02-29 at 3pm",
        "Call Monday or Tuesday at 3pm",
        "Call 04/05/2027 at 3pm",
        "Call tomorrow morning at 3pm",
        "Call in April at 3pm",
        "Compare Monday and Tuesday",
        "Call not tomorrow",
        "Call not at 3pm",
        "Review 04/05/2027",
        "Call in 0 minutes",
        "Call in -5 minutes",
        "Read 'Tomorrow'",
    ] {
        let entry = TaskEntry::parse_english(title, date("2026-12-31"), 10 * 60, true);
        assert_eq!(entry, TaskEntry::plain(title), "{title}");
    }
}

#[test]
fn optional_removal_preserves_unicode_whitespace_and_nonempty_titles() {
    let title = "  Przygotuj  café ☕ tomorrow at 3 PM  ";
    let entry = TaskEntry::parse_english(title, date("2026-12-31"), 0, true);
    assert_eq!(entry.title, "Przygotuj  café ☕");
    assert!(entry.reminder.is_some());
    assert_eq!(
        TaskEntry::parse_english("tomorrow", date("2026-12-31"), 0, true).title,
        "tomorrow"
    );
    assert_eq!(
        parse("Keep tomorrow in the title").title,
        "Keep tomorrow in the title"
    );
    assert_eq!(parse("Keep tomorrow in the title").due_date, None);
}

#[test]
fn parsed_creation_is_atomic_and_keeps_smart_view_membership() {
    let today = date("2026-12-31");
    let mut state = AppState::new();
    let entry = parse("Buy milk tomorrow");
    let change = state
        .apply(
            Action::AddTaskEntry {
                entry,
                view: View::MyDay,
            },
            today,
        )
        .unwrap();
    let task = state.task(change.created_task.unwrap()).unwrap();
    assert_eq!(task.my_day, Some(today));
    assert_eq!(task.due_date, Some(date("2027-01-01")));
    let change = state
        .apply(
            Action::AddTaskEntry {
                entry: parse("Call tomorrow at 3pm"),
                view: View::Planned,
            },
            today,
        )
        .unwrap();
    let task = state.task(change.created_task.unwrap()).unwrap();
    assert_eq!(
        task.due_date, None,
        "An explicit reminder must not inherit Planned's today fallback"
    );
    assert_eq!(task.reminder.as_ref().unwrap().date, date("2027-01-01"));

    let unchanged = state.clone();
    let mut invalid = TaskEntry::plain("Invalid");
    invalid.reminder = Some(Reminder {
        date: today,
        minutes_after_midnight: 1440,
        notified: false,
    });
    assert!(
        state
            .apply(
                Action::AddTaskEntry {
                    entry: invalid,
                    view: View::Important
                },
                today
            )
            .is_err()
    );
    assert_eq!(state, unchanged);
    assert!(
        state
            .apply(
                Action::AddTaskEntry {
                    entry: TaskEntry::plain("Missing list"),
                    view: View::List(999)
                },
                today
            )
            .is_err()
    );
    assert_eq!(state, unchanged);
}

#[test]
fn old_settings_enable_recognition_but_retain_title_text() {
    let settings: Settings = serde_json::from_str("{}").unwrap();
    assert!(settings.recognize_dates);
    assert!(!settings.remove_recognized_dates);
}

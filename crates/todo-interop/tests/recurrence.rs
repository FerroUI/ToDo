use todo_domain::{AppState, Date, RelativeMode, RepeatDay, RepeatEnd, RepeatUnit};
use todo_interop::import_pst;
fn date(value: &str) -> Date {
    value.parse().unwrap()
}

#[test]
fn native_outlook_recurrences_decode_without_application_metadata() {
    let report = import_pst(
        include_bytes!("fixtures/recurrence.pst"),
        &AppState::new(),
        date("2026-11-23"),
    )
    .unwrap();
    let tasks = &report.archive.document.tasks;
    let rule = |name: &str| {
        tasks
            .iter()
            .find(|task| task.title == name)
            .unwrap()
            .recurrence
            .as_ref()
            .unwrap()
    };
    let daily = rule("Daily");
    assert_eq!(daily.every, 3);
    assert_eq!(daily.end, RepeatEnd::After(4));
    assert_eq!(
        daily.last_occurrence(date("2026-11-23")),
        Some(date("2026-12-02"))
    );
    let weekly = rule("Weekly set");
    assert_eq!(
        weekly.days_of_week,
        vec![RepeatDay::Monday, RepeatDay::Thursday]
    );
    assert_eq!(weekly.week_start, RepeatDay::Monday);
    assert_eq!(weekly.end, RepeatEnd::On(date("2026-12-24")));
    assert_eq!(
        rule("Month end").occurrence(2, date("2027-01-31")),
        Some(date("2027-03-31"))
    );
    assert_eq!(
        rule("Leap year").last_occurrence(date("2024-02-29")),
        Some(date("2026-02-28"))
    );
    assert_eq!(
        rule("Last selected weekday").relative_mode,
        RelativeMode::NthMatchingDay
    );
    assert_eq!(
        rule("Last selected weekday").occurrence(2, date("2026-11-30")),
        Some(date("2027-01-29"))
    );
    assert_eq!(rule("Relative yearly").unit, RepeatUnit::Year);
    let remaining = rule("Remaining count");
    assert_eq!(remaining.anchor, Some(date("2027-02-28")));
    assert_eq!(remaining.day_of_month, Some(31));
    assert_eq!(remaining.end, RepeatEnd::After(3));
    assert!(
        tasks
            .iter()
            .find(|task| task.title == "Completed occurrence")
            .unwrap()
            .recurrence_generated
    );
    assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
    assert!(report.warnings[0].contains("completion-relative"));
    let sliding = tasks
        .iter()
        .find(|task| task.title == "Sliding preserved")
        .unwrap();
    assert!(sliding.recurrence.is_none());
    assert!(sliding.microsoft_data.as_ref().unwrap()["pst"]["properties"].is_object());
    report.archive.document.validate().unwrap();
}

#[test]
fn recurrence_metadata_restores_original_range_but_outlook_edits_and_disable_win() {
    let report = import_pst(
        include_bytes!("fixtures/recurrence.pst"),
        &AppState::new(),
        date("2026-11-23"),
    )
    .unwrap();
    let task = |name: &str| {
        report
            .archive
            .document
            .tasks
            .iter()
            .find(|task| task.title == name)
            .unwrap()
    };
    let original = task("Original range restored").recurrence.as_ref().unwrap();
    assert_eq!(original.unit, RepeatUnit::Year);
    assert_eq!(original.every, 3);
    assert_eq!(original.anchor, Some(date("2024-01-01")));
    assert_eq!(original.time_zone.as_deref(), Some("Pacific Standard Time"));
    assert_eq!(original.end, RepeatEnd::On(date("2033-12-31")));
    let changed = task("Outlook changed repeat").recurrence.as_ref().unwrap();
    assert_eq!(changed.unit, RepeatUnit::Day);
    assert_eq!(changed.every, 5);
    assert_eq!(changed.end, RepeatEnd::Never);
    assert!(changed.time_zone.is_none());
    assert!(task("Outlook disabled repeat").recurrence.is_none());
    assert_eq!(
        task("Outlook added repeat")
            .recurrence
            .as_ref()
            .unwrap()
            .every,
        5
    );
    assert!(task("App-only historical").recurrence_generated);
    assert_eq!(
        task("Start without due").start_date,
        Some(date("2026-12-01"))
    );
    assert!(task("Start without due").due_date.is_none());
}

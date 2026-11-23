use todo_domain::*;
fn date(value: &str) -> Date {
    value.parse().unwrap()
}
fn parse(value: &str) -> TaskEntry {
    TaskEntry::parse_english(value, date("2026-11-23"), 600, true)
}

#[test]
fn named_month_dates_accept_ordinals_explicit_years_and_upcoming_leap_days() {
    for (text, expected) in [
        ("File taxes January 15, 2027", "2027-01-15"),
        ("File taxes 15th January 2027", "2027-01-15"),
        ("Review 1st of December", "2026-12-01"),
        ("Leap review Feb 29", "2028-02-29"),
        ("Review in 2 weeks", "2026-12-07"),
        ("Review in 3 months", "2027-02-23"),
    ] {
        assert_eq!(parse(text).due_date, Some(date(expected)), "{text}");
    }
    let call = parse("Call on January 1 at 3 PM");
    assert_eq!(call.title, "Call");
    assert_eq!(call.due_date, None);
    assert_eq!(call.reminder.unwrap().date, date("2027-01-01"));
}

#[test]
fn weekday_sets_end_ranges_clock_time_and_month_day_rules_compose() {
    let weekly = parse("Submit report every Mon and Thu until Dec 31");
    assert_eq!(weekly.title, "Submit report");
    assert_eq!(weekly.due_date, Some(date("2026-11-23")));
    let rule = weekly.recurrence.unwrap();
    assert_eq!(
        rule.days_of_week,
        vec![RepeatDay::Monday, RepeatDay::Thursday]
    );
    assert_eq!(rule.end, RepeatEnd::On(date("2026-12-31")));
    assert_eq!(
        rule.last_occurrence(date("2026-11-23")),
        Some(date("2026-12-31"))
    );
    let rent = parse("Pay rent every month on the 1st at 09:30 for 4 occurrences");
    assert_eq!(rent.title, "Pay rent");
    assert_eq!(rent.due_date, Some(date("2026-12-01")));
    assert_eq!(rent.reminder.as_ref().unwrap().minutes_after_midnight, 570);
    assert_eq!(rent.reminder.as_ref().unwrap().date, date("2026-12-01"));
    assert_eq!(rent.recurrence.as_ref().unwrap().end, RepeatEnd::After(4));
    assert_eq!(
        rent.recurrence.unwrap().last_occurrence(date("2026-11-23")),
        Some(date("2027-03-01"))
    );
}

#[test]
fn relative_patterns_explicit_anchors_and_weekday_ranges_are_unambiguous() {
    let yearly = parse("Host dinner every year on the fourth Thursday of November");
    assert_eq!(yearly.due_date, Some(date("2026-11-26")));
    let monthly = parse("Close books every 2 months on the last weekday starting Dec 1");
    let rule = monthly.recurrence.unwrap();
    assert_eq!(rule.relative_mode, RelativeMode::NthMatchingDay);
    assert_eq!(rule.anchor, Some(date("2026-12-01")));
    assert_eq!(
        rule.occurrence(1, date("2026-11-23")),
        Some(date("2027-02-26"))
    );
    let weekdays = parse("Standup every Mon-Fri at 9am for 3 occurrences");
    assert_eq!(weekdays.recurrence.as_ref().unwrap().days_of_week.len(), 5);
    assert_eq!(
        weekdays
            .recurrence
            .unwrap()
            .last_occurrence(date("2026-11-23")),
        Some(date("2026-11-25"))
    );
    let combined = parse("Call on December 5 at 3pm every week for 2 occurrences");
    assert_eq!(combined.due_date, Some(date("2026-12-05")));
    assert_eq!(combined.reminder.unwrap().date, date("2026-12-05"));
    assert_eq!(
        combined
            .recurrence
            .unwrap()
            .last_occurrence(date("2026-11-23")),
        Some(date("2026-12-12"))
    );
}

#[test]
fn embedded_explicit_clauses_and_date_ranges_preserve_the_rest_of_the_title() {
    let entry = parse("Review  café ☕ due Dec 5 with Ana");
    assert_eq!(entry.title, "Review  café ☕ with Ana");
    assert_eq!(entry.due_date, Some(date("2026-12-05")));
    let range = parse("Holiday from Dec 30 through Jan 2");
    assert_eq!(range.title, "Holiday");
    assert_eq!(range.start_date, Some(date("2026-12-30")));
    assert_eq!(range.due_date, Some(date("2027-01-02")));
    let between = parse("Work between December 1 and December 5 at noon");
    assert_eq!(between.start_date, Some(date("2026-12-01")));
    assert_eq!(between.due_date, Some(date("2026-12-05")));
    assert_eq!(between.reminder.unwrap().minutes_after_midnight, 720);
    let prefix = parse("Every Monday call Mom");
    assert_eq!(prefix.title, "call Mom");
    assert!(prefix.recurrence.is_some());
    let original = "  Submit report every Mon and Thu until Dec 31  ";
    assert_eq!(
        TaskEntry::parse_english(original, date("2026-11-23"), 0, false).title,
        original.trim()
    );
}

#[test]
fn negations_invalid_ranges_incomplete_rules_and_ordinary_prose_stay_literal() {
    for title in [
        "Read daily news",
        "Keep tomorrow in the title",
        "Discuss every option",
        "Meet every Monday or Tuesday",
        "Meet every Monday and discuss",
        "Meet not every Monday",
        "Do not repeat every Monday",
        "Do not remind me tomorrow at 3pm",
        "Pay every 0 days",
        "Pay every 65536 weeks",
        "Pay every month on the 0th",
        "Pay every month on the fifth Monday",
        "Pay every month until February 30",
        "Vacation from Jan 5 2027 to Jan 1 2027",
        "Call on April 31 at 3pm",
        "Call on February 29 2027 at 3pm",
        "Review 04/05/2027",
        "Call due tomorrow or Friday",
        "Repeat every Monday for 0 occurrences",
    ] {
        assert_eq!(parse(title), TaskEntry::plain(title), "{title}");
    }
    let huge = format!("{} every Monday", "界".repeat(3000));
    assert_eq!(parse(&huge), TaskEntry::plain(huge));
}

#[test]
fn parsed_ranges_repeat_and_reminder_offsets_survive_completion_and_persistence() {
    let today = date("2026-11-23");
    let mut state = AppState::new();
    let entry = TaskEntry {
        title: "Range".into(),
        start_date: Some(date("2026-11-21")),
        due_date: Some(today),
        reminder: Some(Reminder {
            date: date("2026-11-22"),
            minutes_after_midnight: 540,
            notified: false,
        }),
        recurrence: Some(Recurrence {
            anchor: Some(today),
            end: RepeatEnd::After(2),
            ..Recurrence::daily()
        }),
    };
    let id = state
        .apply(
            Action::AddTaskEntry {
                entry,
                view: View::MyDay,
            },
            today,
        )
        .unwrap()
        .created_task
        .unwrap();
    let next = state
        .apply(
            Action::SetCompleted {
                id,
                completed: true,
            },
            today,
        )
        .unwrap()
        .created_task
        .unwrap();
    assert_eq!(
        state.task(next).unwrap().start_date,
        Some(date("2026-11-22"))
    );
    assert_eq!(state.task(next).unwrap().due_date, Some(date("2026-11-24")));
    assert_eq!(
        state.task(next).unwrap().reminder.as_ref().unwrap().date,
        today
    );
    let before = state.clone();
    assert!(
        state
            .apply(
                Action::SetDueDate {
                    id: next,
                    date: Some(date("2026-11-20"))
                },
                today
            )
            .is_err()
    );
    assert_eq!(state, before);
    assert_eq!(
        AppState::from_json(&state.to_json().unwrap()).unwrap(),
        state
    );
}

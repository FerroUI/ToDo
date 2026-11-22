use todo_domain::{Action, AppState, Date, Id, Recurrence, RepeatUnit, TASKS_LIST};

fn date(value: &str) -> Date {
    value.parse().unwrap()
}

fn february_occurrence() -> (AppState, Id) {
    let mut state = AppState::new();
    let january = date("2027-01-31");
    let id = state
        .apply(
            Action::AddTask {
                title: "Month end".into(),
                list_id: TASKS_LIST,
            },
            january,
        )
        .unwrap()
        .created_task
        .unwrap();
    state
        .apply(
            Action::SetDueDate {
                id,
                date: Some(january),
            },
            january,
        )
        .unwrap();
    state
        .apply(
            Action::SetRecurrence {
                id,
                recurrence: Some(Recurrence::monthly()),
            },
            january,
        )
        .unwrap();
    let next = state
        .apply(Action::ToggleCompleted(id), january)
        .unwrap()
        .created_task
        .unwrap();
    assert_eq!(state.task(next).unwrap().due_date, Some(date("2027-02-28")));
    (state, next)
}

#[test]
fn unchanged_date_and_repeat_saves_preserve_the_original_month_end_anchor() {
    let (mut state, id) = february_occurrence();
    let february = date("2027-02-28");
    let before = state.clone();
    let changes = state
        .apply(
            Action::SetDueDate {
                id,
                date: Some(february),
            },
            february,
        )
        .unwrap();
    assert!(!changes.changed, "Saving the same due date is a no-op");
    let changes = state
        .apply(
            Action::SetRecurrence {
                id,
                recurrence: Some(Recurrence::monthly()),
            },
            february,
        )
        .unwrap();
    assert!(!changes.changed, "Saving the same repeat rule is a no-op");
    assert_eq!(state, before);

    let next = state
        .apply(Action::ToggleCompleted(id), february)
        .unwrap()
        .created_task
        .unwrap();
    assert_eq!(state.task(next).unwrap().due_date, Some(date("2027-03-31")));
}

#[test]
fn unchanged_repeat_on_a_completed_original_cannot_generate_a_second_successor() {
    let (mut state, successor) = february_occurrence();
    let original = state.tasks.iter().find(|task| task.completed).unwrap().id;
    let before = state.clone();
    let changes = state
        .apply(
            Action::SetRecurrence {
                id: original,
                recurrence: Some(Recurrence::monthly()),
            },
            date("2027-02-01"),
        )
        .unwrap();
    assert!(!changes.changed);
    assert_eq!(state, before);
    assert!(state.task(original).unwrap().recurrence_generated);

    state
        .apply(Action::ToggleCompleted(original), date("2027-02-01"))
        .unwrap();
    let changes = state
        .apply(Action::ToggleCompleted(original), date("2027-02-01"))
        .unwrap();
    assert_eq!(changes.created_task, None);
    assert_eq!(state.tasks.len(), 2);
    assert_eq!(state.task(successor), before.task(successor));
}

#[test]
fn changed_dates_and_rules_reanchor_but_explicit_anchors_are_honored() {
    let (original, id) = february_occurrence();
    let february = date("2027-02-28");
    for (action, anchor, next_due) in [
        (
            Action::SetDueDate {
                id,
                date: Some(date("2027-02-27")),
            },
            "2027-02-27",
            "2027-03-27",
        ),
        (
            Action::SetRecurrence {
                id,
                recurrence: Some(Recurrence::new(RepeatUnit::Month, 2)),
            },
            "2027-02-28",
            "2027-04-28",
        ),
        (
            Action::SetRecurrence {
                id,
                recurrence: Some(Recurrence {
                    unit: RepeatUnit::Month,
                    every: 1,
                    anchor: Some(date("2027-02-15")),
                }),
            },
            "2027-02-15",
            "2027-03-15",
        ),
    ] {
        let mut state = original.clone();
        assert!(state.apply(action, february).unwrap().changed);
        assert_eq!(
            state.task(id).unwrap().recurrence.as_ref().unwrap().anchor,
            Some(date(anchor))
        );
        let next = state
            .apply(Action::ToggleCompleted(id), february)
            .unwrap()
            .created_task
            .unwrap();
        assert_eq!(state.task(next).unwrap().due_date, Some(date(next_due)));
    }
}

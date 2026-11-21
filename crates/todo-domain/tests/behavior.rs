use todo_domain::*;

fn day(text: &str) -> Date {
    text.parse().unwrap()
}
fn today() -> Date {
    day("2026-11-21")
}
fn add(state: &mut AppState, title: &str) -> Id {
    state
        .apply(
            Action::AddTask {
                title: title.into(),
                list_id: TASKS_LIST,
            },
            today(),
        )
        .unwrap()
        .created_task
        .unwrap()
}

#[test]
fn default_list_is_permanent() {
    let mut state = AppState::new();
    assert_eq!(
        state.apply(Action::DeleteList(TASKS_LIST), today()),
        Err(DomainError::ProtectedList)
    );
    assert_eq!(
        state.apply(
            Action::RenameList {
                id: TASKS_LIST,
                name: "Other".into()
            },
            today()
        ),
        Err(DomainError::ProtectedList)
    );
    state.validate().unwrap();
}

#[test]
fn my_day_rolls_over_and_includes_due_today_with_explicit_removal() {
    let mut state = AppState::new();
    let id = add(&mut state, "Task");
    state
        .apply(
            Action::SetDueDate {
                id,
                date: Some(today()),
            },
            today(),
        )
        .unwrap();
    assert_eq!(state.count(&View::MyDay, today()).active, 1);
    state
        .apply(Action::SetMyDay { id, day: None }, today())
        .unwrap();
    assert_eq!(state.count(&View::MyDay, today()).active, 0);
    state
        .apply(
            Action::SetMyDay {
                id,
                day: Some(today()),
            },
            today(),
        )
        .unwrap();
    assert_eq!(state.count(&View::MyDay, today()).active, 1);
    assert_eq!(
        state
            .count(&View::MyDay, today().succ_opt().unwrap())
            .active,
        0
    );
    assert_eq!(state.count(&View::List(TASKS_LIST), today()).active, 1);
}

#[test]
fn monthly_and_yearly_recurrence_preserve_the_original_day() {
    let recurrence = Recurrence {
        unit: RepeatUnit::Month,
        every: 1,
        anchor: Some(day("2026-01-31")),
    };
    assert_eq!(
        recurrence.next_after(day("2026-01-31"), day("2026-01-31")),
        Some(day("2026-02-28"))
    );
    assert_eq!(
        recurrence.next_after(day("2026-02-28"), day("2026-02-28")),
        Some(day("2026-03-31"))
    );
    let recurrence = Recurrence {
        unit: RepeatUnit::Year,
        every: 1,
        anchor: Some(day("2024-02-29")),
    };
    assert_eq!(
        recurrence.next_after(day("2027-02-28"), day("2027-02-28")),
        Some(day("2028-02-29"))
    );
}

#[test]
fn overdue_repeat_skips_backlog_and_weekdays_skip_weekends() {
    assert_eq!(
        Recurrence::daily().next_after(day("2020-01-01"), today()),
        today().succ_opt()
    );
    assert_eq!(
        Recurrence::weekdays().next_after(day("2026-11-20"), day("2026-11-20")),
        Some(day("2026-11-23"))
    );
    let five = Recurrence::new(RepeatUnit::Weekday, 5);
    assert_eq!(five.next_after(today(), today()), Some(day("2026-11-27")));
}

#[test]
fn completion_preserves_history_and_generates_exactly_one_next_occurrence() {
    let mut state = AppState::new();
    let id = add(&mut state, "Daily walk");
    state
        .apply(
            Action::AddStep {
                task_id: id,
                title: "Put on shoes".into(),
            },
            today(),
        )
        .unwrap();
    let step = state.task(id).unwrap().steps[0].id;
    state
        .apply(
            Action::ToggleStep {
                task_id: id,
                step_id: step,
            },
            today(),
        )
        .unwrap();
    state
        .apply(
            Action::SetRecurrence {
                id,
                recurrence: Some(Recurrence::daily()),
            },
            today(),
        )
        .unwrap();
    let next = state
        .apply(Action::ToggleCompleted(id), today())
        .unwrap()
        .created_task
        .unwrap();
    assert!(state.task(id).unwrap().completed);
    assert_eq!(state.task(next).unwrap().due_date, today().succ_opt());
    assert!(!state.task(next).unwrap().steps[0].completed);
    assert_ne!(state.task(next).unwrap().steps[0].id, step);
    state.apply(Action::ToggleCompleted(id), today()).unwrap();
    state.apply(Action::ToggleCompleted(id), today()).unwrap();
    assert_eq!(state.tasks.len(), 2);
    state.validate().unwrap();
}

#[test]
fn batch_is_atomic_on_invalid_foreign_key() {
    let mut state = AppState::new();
    let before = state.clone();
    assert!(
        state
            .apply(
                Action::Batch(vec![
                    Action::AddTask {
                        title: "Valid".into(),
                        list_id: TASKS_LIST
                    },
                    Action::AddTask {
                        title: "Invalid".into(),
                        list_id: 987
                    },
                ]),
                today()
            )
            .is_err()
    );
    assert_eq!(state, before);
}

#[test]
fn deleting_group_ungroups_lists_and_deleting_list_removes_its_tasks() {
    let mut state = AppState::new();
    let group = state
        .apply(
            Action::AddGroup {
                name: "Work".into(),
            },
            today(),
        )
        .unwrap()
        .created_group
        .unwrap();
    let list = state
        .apply(
            Action::AddList {
                name: "Project".into(),
            },
            today(),
        )
        .unwrap()
        .created_list
        .unwrap();
    state
        .apply(
            Action::MoveList {
                id: list,
                group_id: Some(group),
            },
            today(),
        )
        .unwrap();
    state
        .apply(
            Action::AddTask {
                title: "Ship".into(),
                list_id: list,
            },
            today(),
        )
        .unwrap();
    state.apply(Action::DeleteGroup(group), today()).unwrap();
    assert_eq!(state.list(list).unwrap().group_id, None);
    let changes = state.apply(Action::DeleteList(list), today()).unwrap();
    assert_eq!(changes.deleted_tasks.len(), 1);
    assert!(state.tasks.is_empty());
    state.validate().unwrap();
}

#[test]
fn persistence_round_trip_and_duplicate_id_validation() {
    let state = AppState::sample(today());
    assert_eq!(
        AppState::from_json(&state.to_json().unwrap()).unwrap(),
        state
    );
    let mut invalid = state.clone();
    invalid.tasks[0].id = TASKS_LIST;
    assert!(AppState::from_json(&invalid.to_json().unwrap()).is_err());
    invalid = state.clone();
    invalid.tasks[0].list_id = 9999;
    assert!(AppState::from_json(&invalid.to_json().unwrap()).is_err());
}

#[test]
fn future_backup_is_rejected_and_allocation_cursor_repaired() {
    let mut state = AppState::sample(today());
    state.next_id = 2;
    let parsed = AppState::from_json(&state.to_json().unwrap()).unwrap();
    assert!(parsed.next_id > parsed.tasks.iter().map(|task| task.id).max().unwrap());
    state.schema_version = 999;
    assert_eq!(
        AppState::from_json(&state.to_json().unwrap()),
        Err(DomainError::FutureSchema(999))
    );
}

#[test]
fn search_reads_notes_steps_and_tags_and_smart_views_filter_correctly() {
    let mut state = AppState::new();
    let id = add(&mut state, "Review");
    state
        .apply(
            Action::SetNotes {
                id,
                notes: "FerroUI architecture".into(),
            },
            today(),
        )
        .unwrap();
    state
        .apply(
            Action::SetTags {
                id,
                tags: vec!["#Rust".into()],
            },
            today(),
        )
        .unwrap();
    state
        .apply(
            Action::SetImportant {
                id,
                important: true,
            },
            today(),
        )
        .unwrap();
    assert_eq!(
        state
            .count(&View::Search("ferroui #rust".into()), today())
            .active,
        1
    );
    assert_eq!(state.count(&View::Important, today()).active, 1);
    state.apply(Action::ToggleCompleted(id), today()).unwrap();
    assert!(
        state
            .visible_tasks(&View::Important, SortOrder::Manual, false, today())
            .is_empty()
    );
    assert_eq!(
        state
            .visible_tasks(&View::Completed, SortOrder::Manual, false, today())
            .len(),
        1
    );
}

#[test]
fn manual_reorder_is_stable_and_rejects_cross_list_targets() {
    let mut state = AppState::new();
    let a = add(&mut state, "A");
    let b = add(&mut state, "B");
    let c = add(&mut state, "C");
    state
        .apply(
            Action::ReorderTask {
                id: c,
                before: Some(a),
            },
            today(),
        )
        .unwrap();
    let ids: Vec<_> = state
        .visible_tasks(&View::All, SortOrder::Manual, true, today())
        .iter()
        .map(|task| task.id)
        .collect();
    assert_eq!(ids, vec![c, a, b]);
    let list = state
        .apply(
            Action::AddList {
                name: "Other".into(),
            },
            today(),
        )
        .unwrap()
        .created_list
        .unwrap();
    state
        .apply(
            Action::MoveTask {
                id: b,
                list_id: list,
            },
            today(),
        )
        .unwrap();
    assert!(
        state
            .apply(
                Action::ReorderTask {
                    id: a,
                    before: Some(b)
                },
                today()
            )
            .is_err()
    );
}

#[test]
fn duplicate_list_has_fresh_identifiers_and_valid_attachments() {
    let mut state = AppState::sample(today());
    let id = state.tasks[0].id;
    state
        .apply(
            Action::AddAttachment {
                task_id: id,
                attachment: Attachment {
                    id: 0,
                    name: "brief.pdf".into(),
                    mime_type: "application/pdf".into(),
                    size: 512,
                    storage_key: "blob:brief".into(),
                },
            },
            today(),
        )
        .unwrap();
    state
        .apply(Action::DuplicateList(TASKS_LIST), today())
        .unwrap();
    state.validate().unwrap();
}

use todo_domain::*;

fn today() -> Date {
    "2026-11-23".parse().unwrap()
}
fn add(state: &mut AppState, title: &str, list_id: Id) -> Id {
    state
        .apply(
            Action::AddTask {
                title: title.into(),
                list_id,
            },
            today(),
        )
        .unwrap()
        .created_task
        .unwrap()
}
fn order(state: &AppState, list: Id) -> Vec<Id> {
    state
        .visible_tasks(&View::List(list), SortOrder::Manual, true, today())
        .into_iter()
        .map(|task| task.id)
        .collect()
}
fn list(state: &mut AppState) -> Id {
    state
        .apply(
            Action::AddList {
                name: "Destination".into(),
            },
            today(),
        )
        .unwrap()
        .created_list
        .unwrap()
}

#[test]
fn selected_block_keeps_given_display_order_and_is_an_idempotent_transaction() {
    let mut state = AppState::new();
    let ids: Vec<_> = (0..6)
        .map(|n| add(&mut state, &format!("Task {n}"), TASKS_LIST))
        .collect();
    let action = Action::TransferTasks {
        ids: vec![ids[4], ids[1]],
        list_id: TASKS_LIST,
        before: Some(ids[0]),
    };
    state.apply(action.clone(), today()).unwrap();
    assert_eq!(
        order(&state, TASKS_LIST),
        [ids[4], ids[1], ids[0], ids[2], ids[3], ids[5]]
    );
    let once = state.clone();
    assert!(!state.apply(action, today()).unwrap().changed);
    assert_eq!(state, once);
}

#[test]
fn moving_a_mixed_selection_to_a_saturated_list_preserves_other_sources_and_provider_rows() {
    let mut state = AppState::new();
    let first = add(&mut state, "First", TASKS_LIST);
    let second = add(&mut state, "Second", TASKS_LIST);
    let destination = list(&mut state);
    let existing = add(&mut state, "Existing", destination);
    let protected = add(&mut state, "Flagged", destination);
    state
        .tasks
        .iter_mut()
        .find(|t| t.id == existing)
        .unwrap()
        .position = i64::MAX;
    let source = state.tasks.iter_mut().find(|t| t.id == protected).unwrap();
    source.flagged_email = true;
    source.position = i64::MAX;
    let protected_before = source.clone();
    state
        .apply(
            Action::TransferTasks {
                ids: vec![second, existing, first],
                list_id: destination,
                before: None,
            },
            today(),
        )
        .unwrap();
    let movable: Vec<_> = order(&state, destination)
        .into_iter()
        .filter(|id| *id != protected)
        .collect();
    assert_eq!(movable, [second, existing, first]);
    assert_eq!(state.task(protected).unwrap(), &protected_before);
    assert!(order(&state, TASKS_LIST).is_empty());
    state.validate().unwrap();
}

#[test]
fn invalid_member_or_anchor_rejects_every_change_including_service_boundaries() {
    let mut original = AppState::new();
    let a = add(&mut original, "First", TASKS_LIST);
    let b = add(&mut original, "Flagged", TASKS_LIST);
    original
        .tasks
        .iter_mut()
        .find(|t| t.id == b)
        .unwrap()
        .flagged_email = true;
    let destination = list(&mut original);
    for (ids, before) in [
        (vec![a, b], None),
        (vec![a, 9999], None),
        (vec![a, a], None),
        (vec![a], Some(a)),
        (vec![a], Some(b)),
    ] {
        let mut state = original.clone();
        assert!(
            state
                .apply(
                    Action::TransferTasks {
                        ids,
                        list_id: destination,
                        before
                    },
                    today()
                )
                .is_err()
        );
        assert_eq!(state, original);
    }
}

#[test]
fn clipboard_copies_create_independent_tasks_children_and_smart_view_metadata() {
    let mut state = AppState::new();
    let id = add(&mut state, "Original", TASKS_LIST);
    state
        .apply(
            Action::AddStep {
                task_id: id,
                title: "Step".into(),
            },
            today(),
        )
        .unwrap();
    let mut copied = state.task(id).unwrap().clone();
    copied.completed = true;
    copied.steps[0].completed = true;
    copied.flagged_email = true;
    copied.microsoft_data = Some(serde_json::json!({"id":"remote"}));
    copied.notes = "Preserved note".into();
    let change = state
        .apply(
            Action::PasteTasks {
                tasks: vec![copied.clone(), copied],
                view: View::MyDay,
            },
            today(),
        )
        .unwrap();
    assert_eq!(change.changed_tasks.len(), 2);
    for task in state.tasks.iter().filter(|task| task.id != id) {
        assert_ne!(task.steps[0].id, state.task(id).unwrap().steps[0].id);
        assert!(!task.completed && !task.steps[0].completed && !task.flagged_email);
        assert!(task.microsoft_data.is_none());
        assert!(task.is_my_day(today()));
        assert_eq!(task.notes, "Preserved note");
    }
    state.validate().unwrap();
}

#[test]
fn invalid_clipboard_batch_and_exhausted_ids_never_leave_partial_copies() {
    let mut original = AppState::new();
    let id = add(&mut original, "Original", TASKS_LIST);
    let good = original.task(id).unwrap().clone();
    let mut bad = good.clone();
    bad.reminder = Some(Reminder {
        date: today(),
        minutes_after_midnight: 1440,
        notified: false,
    });
    let mut state = original.clone();
    assert!(
        state
            .apply(
                Action::PasteTasks {
                    tasks: vec![good.clone(), bad],
                    view: View::Important
                },
                today()
            )
            .is_err()
    );
    assert_eq!(state, original);
    state.next_id = i64::MAX as u64 - 1;
    let before = state.clone();
    assert!(
        state
            .apply(
                Action::PasteTasks {
                    tasks: vec![good],
                    view: View::Important
                },
                today()
            )
            .is_err()
    );
    assert_eq!(state, before);
}

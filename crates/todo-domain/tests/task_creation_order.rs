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

fn imported_state(position: i64) -> (AppState, Id, Id) {
    let mut state = AppState::new();
    let source = add(&mut state, "Source", TASKS_LIST);
    state
        .apply(
            Action::AddStep {
                task_id: source,
                title: "Promote this step".into(),
            },
            today(),
        )
        .unwrap();
    state
        .apply(
            Action::SetRecurrence {
                id: source,
                recurrence: Some(Recurrence::daily()),
            },
            today(),
        )
        .unwrap();
    let step = state.task(source).unwrap().steps[0].id;
    let other = state
        .apply(
            Action::AddList {
                name: "Other".into(),
            },
            today(),
        )
        .unwrap()
        .created_list
        .unwrap();
    let unrelated = add(&mut state, "Unrelated order limit", other);
    state
        .tasks
        .iter_mut()
        .find(|task| task.id == source)
        .unwrap()
        .position = position;
    state
        .tasks
        .iter_mut()
        .find(|task| task.id == unrelated)
        .unwrap()
        .position = i64::MAX;
    // Both sparse and extreme order keys are valid existing backups.
    (
        AppState::from_json(&state.to_json().unwrap()).unwrap(),
        source,
        step,
    )
}

fn creation_actions(source: Id, step: Id) -> Vec<Action> {
    vec![
        Action::AddTask {
            title: "Plain".into(),
            list_id: TASKS_LIST,
        },
        Action::AddTaskInView {
            title: "From view".into(),
            view: View::List(TASKS_LIST),
        },
        Action::AddTaskEntry {
            entry: TaskEntry::plain("Scheduled"),
            view: View::MyDay,
        },
        Action::DuplicateTask(source),
        Action::PromoteStep {
            task_id: source,
            step_id: step,
        },
        Action::SetCompleted {
            id: source,
            completed: true,
        },
    ]
}

#[test]
fn every_task_creation_path_appends_after_imported_positions_only_in_its_own_list() {
    let (original, source, step) = imported_state(1000);
    for action in creation_actions(source, step) {
        let mut state = original.clone();
        let changes = state.apply(action, today()).unwrap();
        let created = state.task(changes.created_task.unwrap()).unwrap();
        assert_eq!(created.list_id, TASKS_LIST);
        assert_eq!(created.position, 1001);
        assert_eq!(state.tasks.len(), original.tasks.len() + 1);
        assert_eq!(state.tasks.last().unwrap().id, created.id);
        state.validate().unwrap();
    }
}

#[test]
fn saturated_task_order_is_compacted_before_every_creation_path() {
    let (original, source, step) = imported_state(i64::MAX);
    for action in creation_actions(source, step) {
        let mut state = original.clone();
        let changes = state.apply(action, today()).unwrap();
        let created = changes.created_task.unwrap();
        assert_eq!(state.task(source).unwrap().position, 0);
        assert_eq!(state.task(created).unwrap().position, 1);
        assert!(changes.changed_tasks.contains(&source));
        assert!(changes.changed_tasks.contains(&created));
        assert_eq!(
            state.tasks[1], original.tasks[1],
            "other lists are untouched"
        );
        state.validate().unwrap();
    }
    let mut state = original.clone();
    state
        .apply(
            Action::SetRecurrence {
                id: source,
                recurrence: None,
            },
            today(),
        )
        .unwrap();
    state
        .apply(Action::ToggleCompleted(source), today())
        .unwrap();
    assert!(
        state.task(source).unwrap().completed,
        "completion without a new occurrence needs no order slot"
    );
    assert_eq!(state.task(source).unwrap().position, i64::MAX);
}

#[test]
fn failed_creation_does_not_repair_order_remove_steps_or_allocate_ids() {
    let (original, source, _) = imported_state(i64::MAX);
    for action in [
        Action::AddTask {
            title: " ".into(),
            list_id: TASKS_LIST,
        },
        Action::AddTaskInView {
            title: " ".into(),
            view: View::MyDay,
        },
        Action::AddTaskEntry {
            entry: TaskEntry {
                title: "Invalid time".into(),
                due_date: None,
                reminder: Some(Reminder {
                    date: today(),
                    minutes_after_midnight: 1440,
                    notified: false,
                }),
            },
            view: View::MyDay,
        },
        Action::PromoteStep {
            task_id: source,
            step_id: 9999,
        },
        Action::Batch(vec![
            Action::DuplicateTask(source),
            Action::DeleteTask(9999),
        ]),
    ] {
        let mut state = original.clone();
        assert!(state.apply(action, today()).is_err());
        assert_eq!(state, original);
    }
}

#[test]
fn order_repair_preserves_sparse_ties_and_never_repositions_provider_records() {
    let (mut state, source, _) = imported_state(i64::MAX);
    for (title, position, flagged) in [
        ("First tie", -100, false),
        ("Second tie", -100, false),
        ("Protected source", i64::MAX, true),
    ] {
        let mut task = Task::new(state.next_id, TASKS_LIST, title.into(), today());
        state.next_id += 1;
        task.position = position;
        task.flagged_email = flagged;
        state.tasks.push(task);
    }
    let protected = state.tasks.last().unwrap().clone();
    let created = state
        .apply(Action::DuplicateTask(source), today())
        .unwrap()
        .created_task
        .unwrap();
    let titles = state
        .visible_tasks(&View::List(TASKS_LIST), SortOrder::Manual, true, today())
        .into_iter()
        .map(|task| task.title.as_str())
        .collect::<Vec<_>>();
    assert_eq!(titles, ["First tie", "Second tie", "Source", "Source"]);
    assert_eq!(state.task(created).unwrap().position, 3);
    assert_eq!(state.task(protected.id), Some(&protected));
}

#[test]
fn duplicating_a_list_preserves_manual_order_and_completion_without_copying_sparse_keys() {
    let (mut state, first, _) = imported_state(i64::MAX);
    // In-memory storage order differs from the source list's displayed manual order.
    let mut second = Task::new(state.next_id, TASKS_LIST, "Second".into(), today());
    state.next_id += 1;
    second.position = -10;
    let mut third = Task::new(state.next_id, TASKS_LIST, "Third".into(), today());
    state.next_id += 1;
    third.position = 100;
    third.completed = true;
    third.completed_on = Some(today());
    state.tasks.extend([third, second]);
    state
        .tasks
        .iter_mut()
        .find(|task| task.id == first)
        .unwrap()
        .steps[0]
        .completed = true;
    let source_titles = state
        .visible_tasks(&View::List(TASKS_LIST), SortOrder::Manual, true, today())
        .into_iter()
        .map(|task| (task.title.clone(), task.completed))
        .collect::<Vec<_>>();
    let copy = state
        .apply(Action::DuplicateList(TASKS_LIST), today())
        .unwrap()
        .created_list
        .unwrap();
    let copied = state.visible_tasks(&View::List(copy), SortOrder::Manual, true, today());
    assert_eq!(
        copied
            .iter()
            .map(|task| (task.title.clone(), task.completed))
            .collect::<Vec<_>>(),
        source_titles
    );
    assert!(
        copied
            .iter()
            .find(|task| task.title == "Source")
            .unwrap()
            .steps[0]
            .completed
    );
    let mut positions = copied.iter().map(|task| task.position).collect::<Vec<_>>();
    positions.sort_unstable();
    assert_eq!(positions, [0, 1, 2]);
    state.validate().unwrap();
}

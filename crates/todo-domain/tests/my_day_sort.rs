use todo_domain::*;

#[test]
fn my_day_sort_uses_live_membership_then_position_and_id() {
    let today: Date = "2026-11-22".parse().unwrap();
    let yesterday = today.pred_opt().unwrap();
    let mut state = AppState::new();
    for (id, position) in [(2, 0), (3, 7), (4, 7), (5, 1), (6, -2), (7, 2)] {
        let mut task = Task::new(id, TASKS_LIST, format!("Task {id}"), yesterday);
        task.position = position;
        match id {
            3 | 6 => task.my_day = Some(today),
            4 => task.due_date = Some(today),
            5 => {
                task.due_date = Some(today);
                task.my_day_excluded = Some(today);
            }
            7 => task.my_day = Some(yesterday),
            _ => {}
        }
        state.tasks.push(task);
    }
    state.next_id = 8;
    let ordered = |state: &AppState, day| {
        state
            .visible_tasks(&View::List(TASKS_LIST), SortOrder::MyDay, true, day)
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ordered(&state, today), vec![6, 3, 4, 2, 5, 7]);
    // No addition timestamp is implied: equal membership and position use the stable ID.
    state.tasks.reverse();
    assert_eq!(ordered(&state, today), vec![6, 3, 4, 2, 5, 7]);
    assert_eq!(
        ordered(&state, today.succ_opt().unwrap()),
        vec![6, 2, 5, 7, 3, 4]
    );

    state
        .apply(
            Action::SetListSort {
                id: TASKS_LIST,
                sort: SortOrder::MyDay,
            },
            today,
        )
        .unwrap();
    assert_eq!(SortOrder::MyDay.label(), "Added to My Day");
    assert_eq!(
        AppState::from_json(&state.to_json().unwrap()).unwrap(),
        state
    );
}

#[test]
fn my_day_sort_preserves_the_completed_section() {
    let today: Date = "2026-11-22".parse().unwrap();
    let mut state = AppState::new();
    let active = Task::new(2, TASKS_LIST, "Active".into(), today);
    let mut completed = Task::new(3, TASKS_LIST, "Done today".into(), today);
    completed.completed = true;
    completed.completed_on = Some(today);
    completed.due_date = Some(today);
    state.tasks = vec![completed, active];
    let ordered = state.visible_tasks(&View::All, SortOrder::MyDay, true, today);
    assert_eq!(
        ordered.iter().map(|task| task.id).collect::<Vec<_>>(),
        vec![2, 3]
    );
}

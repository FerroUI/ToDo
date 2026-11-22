use todo_domain::*;

fn today() -> Date {
    "2026-11-22".parse().unwrap()
}

#[test]
fn automatic_my_day_does_not_add_tasks_completed_before_today() {
    let today = today();
    let yesterday = today.pred_opt().unwrap();
    let mut task = Task::new(2, TASKS_LIST, "Prepare early".into(), yesterday);
    task.due_date = Some(today);
    assert!(task.is_my_day(today));
    task.completed = true;
    task.completed_on = Some(yesterday);
    assert!(!task.is_my_day(today));
    task.completed_on = None;
    assert!(!task.is_my_day(today));

    // Microsoft documents auto-population for unfinished due-today tasks.
    // Retaining tasks completed today in My Day's completed section is our
    // continuity policy, inferred from that section rather than a published rule.
    task.completed_on = Some(today);
    assert!(task.is_my_day(today));
    task.my_day_excluded = Some(today);
    assert!(!task.is_my_day(today));
    task.my_day = Some(today);
    assert!(task.is_my_day(today));
    assert!(!task.is_my_day(today.succ_opt().unwrap()));
}

#[test]
fn completing_a_due_today_task_keeps_it_in_my_day_until_midnight() {
    let today = today();
    let mut state = AppState::new();
    let id = state
        .apply(
            Action::AddTask {
                title: "Due today".into(),
                list_id: TASKS_LIST,
            },
            today,
        )
        .unwrap()
        .created_task
        .unwrap();
    state
        .apply(
            Action::SetDueDate {
                id,
                date: Some(today),
            },
            today,
        )
        .unwrap();
    state
        .apply(
            Action::SetCompleted {
                id,
                completed: true,
            },
            today,
        )
        .unwrap();
    assert_eq!(
        state.count(&View::MyDay, today),
        TaskCount {
            active: 0,
            completed: 1
        }
    );
    assert_eq!(
        state
            .visible_tasks(&View::MyDay, SortOrder::Manual, true, today)
            .len(),
        1
    );
    assert!(
        state
            .visible_tasks(&View::MyDay, SortOrder::Manual, false, today)
            .is_empty()
    );
    assert_eq!(
        state.count(&View::MyDay, today.succ_opt().unwrap()).total(),
        0
    );
    state
        .apply(
            Action::SetCompleted {
                id,
                completed: false,
            },
            today,
        )
        .unwrap();
    assert_eq!(state.count(&View::MyDay, today).active, 1);
}

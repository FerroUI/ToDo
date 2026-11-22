use todo_domain::{Action, AppState, CloudList, Date, ExternalTask, TASKS_LIST, Task, View};

fn today() -> Date {
    Date::from_ymd_opt(2026, 11, 22).unwrap()
}
fn planner() -> AppState {
    let mut state = AppState::default();
    let mut task = Task::new(2, TASKS_LIST, "Assigned work".into(), today());
    task.external = Some(ExternalTask {
        provider: "Planner".into(),
        id: "service-task".into(),
        url: "https://planner.cloud.microsoft/".into(),
    });
    task.assigned_to = Some("me".into());
    state.tasks.push(task);
    state.next_id = 3;
    state.settings.planner_enabled = true;
    state
}

#[test]
fn planner_capabilities_are_enforced_in_atomic_batches() {
    let mut state = planner();
    let before = state.clone();
    assert!(
        state
            .apply(
                Action::Batch(vec![
                    Action::RenameTask {
                        id: 2,
                        title: "Changed".into()
                    },
                    Action::DeleteTask(2)
                ]),
                today()
            )
            .is_err()
    );
    assert_eq!(state, before);
    state
        .apply(
            Action::SetCompleted {
                id: 2,
                completed: true,
            },
            today(),
        )
        .unwrap();
    state
        .apply(Action::DeleteCompleted { list_id: None }, today())
        .unwrap();
    assert_eq!(
        state.tasks.len(),
        1,
        "clearing completed personal tasks preserves Planner assignments"
    );
}

#[test]
fn planner_projection_stays_out_of_personal_tasks_and_honors_connection_toggle() {
    let mut state = planner();
    assert_eq!(state.count(&View::AssignedToMe, today()).active, 1);
    assert_eq!(state.count(&View::List(TASKS_LIST), today()).active, 0);
    state.settings.planner_enabled = false;
    assert_eq!(state.count(&View::All, today()).active, 0);
    assert_eq!(state.count(&View::AssignedToMe, today()).active, 0);
}

#[test]
fn shared_list_permissions_apply_to_every_command_and_copies_are_independent() {
    let mut state = AppState::default();
    let id = state
        .apply(
            Action::AddList {
                name: "Team".into(),
            },
            today(),
        )
        .unwrap()
        .created_list
        .unwrap();
    state
        .lists
        .iter_mut()
        .find(|list| list.id == id)
        .unwrap()
        .cloud = Some(CloudList {
        is_owner: false,
        is_shared: true,
        well_known_name: "none".into(),
    });
    assert!(
        state
            .apply(
                Action::RenameList {
                    id,
                    name: "Renamed".into()
                },
                today()
            )
            .is_err()
    );
    assert!(state.apply(Action::DeleteList(id), today()).is_err());
    let copy = state
        .apply(Action::DuplicateList(id), today())
        .unwrap()
        .created_list
        .unwrap();
    assert!(state.list(copy).unwrap().cloud.is_none());
    state.apply(Action::DeleteList(copy), today()).unwrap();
}

#[test]
fn duplicate_planner_task_becomes_an_independent_personal_copy() {
    let mut state = planner();
    let copy = state
        .apply(Action::DuplicateTask(2), today())
        .unwrap()
        .created_task
        .unwrap();
    assert!(state.task(copy).unwrap().external.is_none());
    assert!(state.task(copy).unwrap().assigned_to.is_none());
    state.apply(Action::DeleteTask(copy), today()).unwrap();
    assert!(state.task(2).unwrap().is_planner());
}

#[test]
fn outlook_category_names_keep_hashes_and_commas() {
    let mut state = AppState::default();
    let id = state
        .apply(
            Action::AddTask {
                title: "Categories".into(),
                list_id: TASKS_LIST,
            },
            today(),
        )
        .unwrap()
        .created_task
        .unwrap();
    state
        .apply(
            Action::SetTags {
                id,
                tags: vec!["#Release".into(), "Work, planning".into()],
            },
            today(),
        )
        .unwrap();
    assert_eq!(
        state.task(id).unwrap().tags,
        vec!["#Release", "Work, planning"]
    );
    assert!(AppState::from_json(&state.to_json().unwrap()).is_ok());
}

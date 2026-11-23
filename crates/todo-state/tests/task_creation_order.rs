mod common;
use common::*;
use ferroui_dynamic_data::IObservableCache;
use todo_domain::*;

#[test]
fn duplicating_a_reordered_list_keeps_the_source_unchanged_through_undo_and_redo() {
    let vm = vm();
    vm.navigate(View::List(TASKS_LIST));
    for title in ["First", "Second", "Third"] {
        vm.add_task(title.into());
    }
    let ids = vm.visible_task_ids();
    vm.dispatch(Action::ReorderTask {
        id: ids[2],
        before: Some(ids[0]),
    });
    let source = vm.state().tasks.clone();
    let titles = |state: &AppState, list| {
        state
            .visible_tasks(&View::List(list), SortOrder::Manual, true, today())
            .into_iter()
            .map(|task| task.title.clone())
            .collect::<Vec<_>>()
    };
    vm.dispatch(Action::DuplicateList(TASKS_LIST));
    let copy = vm.view().destination_list();
    assert_ne!(copy, TASKS_LIST);
    assert_eq!(titles(&vm.state(), copy), ["Third", "First", "Second"]);
    assert_eq!(&vm.state().tasks[..source.len()], source.as_slice());
    vm.undo();
    assert_eq!(vm.state().tasks, source);
    assert!(vm.state().list(copy).is_none());
    vm.redo();
    assert_eq!(titles(&vm.state(), copy), ["Third", "First", "Second"]);
    assert_eq!(&vm.state().tasks[..source.len()], source.as_slice());
    vm.state().validate().unwrap();
}

#[test]
fn recovered_order_and_new_tasks_share_one_undo_entry_and_reactive_cache_update() {
    let mut original = AppState::new();
    let mut repeating = Task::new(2, TASKS_LIST, "Last imported".into(), today());
    repeating.position = i64::MAX;
    repeating.recurrence = Some(Recurrence::daily());
    repeating.steps.push(Step {
        id: 4,
        title: "Promote me".into(),
        completed: false,
    });
    let mut earlier = Task::new(3, TASKS_LIST, "First imported".into(), today());
    earlier.position = -100;
    original.lists.push(TaskList::new(5, "Other".into()));
    let other = Task::new(6, 5, "Move me".into(), today());
    original.tasks = vec![repeating, earlier, other];
    original.next_id = 7;
    original.validate().unwrap();
    for action in [
        Action::AddTask {
            title: "New".into(),
            list_id: TASKS_LIST,
        },
        Action::DuplicateTask(2),
        Action::PromoteStep {
            task_id: 2,
            step_id: 4,
        },
        Action::ToggleCompleted(2),
        Action::MoveTask {
            id: 6,
            list_id: TASKS_LIST,
        },
        Action::Batch(vec![
            Action::MoveTask {
                id: 6,
                list_id: TASKS_LIST,
            },
            Action::DuplicateTask(2),
        ]),
    ] {
        let vm =
            todo_state::AppViewModel::new(original.clone(), today(), std::rc::Rc::new(|_| Ok(())));
        vm.navigate(View::List(TASKS_LIST));
        vm.dispatch(action);
        let changed = vm.state().clone();
        assert_eq!(changed.task(3).unwrap().position, 0);
        assert_eq!(changed.task(2).unwrap().position, 1);
        assert_eq!(vm.tasks.lookup(&2).unwrap().position, 1);
        assert!(vm.can_undo());
        vm.undo();
        assert_eq!(vm.state().tasks, original.tasks);
        assert!(!vm.can_undo(), "repair and append must be one edit");
        for task in &original.tasks {
            assert_eq!(vm.tasks.lookup(&task.id).as_ref(), Some(task));
        }
        vm.redo();
        for task in &changed.tasks {
            assert_eq!(vm.state().task(task.id), Some(task));
            assert_eq!(vm.tasks.lookup(&task.id).as_ref(), Some(task));
        }
        assert_eq!(vm.tasks.count(), changed.tasks.len());
        vm.state().validate().unwrap();
    }
}

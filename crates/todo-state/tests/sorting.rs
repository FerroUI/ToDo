mod common;
use common::*;
use todo_domain::*;

#[test]
fn reverse_sort_is_durable_undoable_and_preserves_manual_order_and_completion_sections() {
    let vm = vm();
    vm.navigate(View::List(TASKS_LIST));
    for title in ["Bravo", "Alpha", "Done bravo", "Done alpha"] {
        vm.add_task(title.into());
    }
    let original = vm.visible_task_ids();
    for id in &original[2..] {
        vm.dispatch(Action::SetCompleted {
            id: *id,
            completed: true,
        });
    }
    let titles = || {
        vm.visible_tasks()
            .into_iter()
            .map(|task| task.title)
            .collect::<Vec<_>>()
    };
    vm.set_sort(SortOrder::Alphabetical);
    assert_eq!(titles(), ["Alpha", "Bravo", "Done alpha", "Done bravo"]);
    vm.reverse_sort();
    assert_eq!(titles(), ["Bravo", "Alpha", "Done bravo", "Done alpha"]);
    assert_eq!(
        vm.visible_task_ids(),
        original,
        "selection order follows the actual rows"
    );
    assert!(vm.state().list(TASKS_LIST).unwrap().sort_reversed);
    let restored = AppState::from_json(&vm.state().to_json().unwrap()).unwrap();
    assert!(restored.list(TASKS_LIST).unwrap().sort_reversed);
    vm.navigate(View::Important);
    assert!(!vm.sort_reversed());
    vm.navigate(View::List(TASKS_LIST));
    assert!(vm.sort_reversed());
    vm.undo();
    assert!(!vm.sort_reversed());
    assert_eq!(titles()[0], "Alpha");
    vm.redo();
    assert!(vm.sort_reversed());
    vm.set_sort(SortOrder::Manual);
    assert_eq!(vm.visible_task_ids(), original);
    assert!(!vm.sort_reversed());
    vm.reverse_sort();
    assert_eq!(
        vm.visible_task_ids(),
        original,
        "manual positions cannot be reversed by a stale action"
    );
}

#[test]
fn legacy_lists_default_to_normal_direction_and_search_counts_the_visible_completed_matches() {
    let vm = vm();
    vm.navigate(View::List(TASKS_LIST));
    vm.add_task("Search result one".into());
    vm.add_task("Search result two".into());
    let second = vm.state().tasks[1].id;
    vm.dispatch(Action::ToggleCompleted(second));
    vm.navigate(View::Search("Search result".into()));
    assert_eq!(vm.subtitle(), "2 tasks");
    let mut settings = vm.state().settings.clone();
    settings.show_completed = false;
    vm.dispatch(Action::SetSettings(settings));
    assert_eq!(vm.subtitle(), "1 task");
    vm.navigate(View::Search("  ".into()));
    assert!(vm.visible_tasks().is_empty());
    let legacy = vm
        .state()
        .to_json()
        .unwrap()
        .replace(",\"sort_reversed\":false", "");
    assert!(
        !AppState::from_json(&legacy)
            .unwrap()
            .list(TASKS_LIST)
            .unwrap()
            .sort_reversed
    );
}

#[test]
fn creation_sort_uses_imported_creation_dates_and_reverses_completion_sections_independently() {
    let mut state = AppState::new();
    for (id, created, completed) in [
        (2, "2026-11-20", false),
        (3, "2020-01-01", false),
        (4, "2026-11-20", false),
        (5, "2026-11-19", true),
        (6, "2018-01-01", true),
    ] {
        let mut task = Task::new(
            id,
            TASKS_LIST,
            format!("Task {id}"),
            created.parse().unwrap(),
        );
        task.completed = completed;
        task.completed_on = completed.then_some(today());
        state.tasks.push(task);
    }
    state.next_id = 7;
    let imported = AppState::from_json(&state.to_json().unwrap()).unwrap();
    let vm = todo_state::AppViewModel::new(imported, today(), std::rc::Rc::new(|_| Ok(())));
    vm.navigate(View::List(TASKS_LIST));
    vm.set_sort(SortOrder::Created);
    assert_eq!(vm.visible_task_ids(), [4, 2, 3, 5, 6]);
    vm.reverse_sort();
    assert_eq!(vm.visible_task_ids(), [3, 2, 4, 6, 5]);
    vm.undo();
    assert_eq!(vm.visible_task_ids(), [4, 2, 3, 5, 6]);
}

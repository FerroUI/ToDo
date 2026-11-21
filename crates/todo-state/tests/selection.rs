mod common;
use common::*;
use ferroui_dynamic_data::IObservableCache;
use todo_domain::{Action, Id, Recurrence, View, TASKS_LIST};
use todo_state::{AppViewModel, Panel};

fn add_tasks(vm: &AppViewModel, count: usize) -> Vec<Id> {
    vm.navigate(View::List(TASKS_LIST));
    for index in 0..count {
        vm.add_task(format!("Task {index}"));
    }
    vm.visible_task_ids()
}

#[test]
fn range_completion_and_recurring_successor_are_one_undo_step() {
    let vm = vm();
    let ids = add_tasks(&vm, 4);
    vm.dispatch(Action::SetRecurrence {
        id: ids[0],
        recurrence: Some(Recurrence::daily()),
    });
    vm.select(Some(ids[0]));
    vm.select_task_range(&ids, ids[2]);
    assert_eq!(vm.selection().into_iter().collect::<Vec<_>>(), ids[..3]);
    assert!(vm.selection_mode());
    assert_eq!(vm.selected(), None);
    assert_eq!(vm.task_cursor(), Some(ids[2]));

    vm.complete_selected_tasks();
    assert_eq!(vm.count(&View::Completed), 3);
    assert_eq!(vm.state().tasks.len(), 5);
    assert!(!vm.selection_mode());
    assert_eq!(vm.selection_count(), 0);
    vm.undo();
    assert_eq!(vm.count(&View::Completed), 0);
    assert_eq!(vm.state().tasks.len(), 4);
    assert_eq!(vm.tasks.count(), 4);
    vm.redo();
    assert_eq!(vm.count(&View::Completed), 3);
    assert_eq!(vm.state().tasks.len(), 5);
    vm.state().validate().unwrap();
}

#[test]
fn bulk_move_and_confirmed_delete_each_undo_as_one_transaction() {
    let vm = vm();
    let ids = add_tasks(&vm, 3);
    vm.dispatch(Action::AddList {
        name: "Destination".into(),
    });
    let destination = vm.view().destination_list();
    vm.navigate(View::List(TASKS_LIST));
    vm.select_all_tasks(&ids[..2]);
    vm.move_selection(destination);
    assert_eq!(vm.count(&View::List(destination)), 2);
    assert_eq!(vm.count(&View::List(TASKS_LIST)), 1);
    vm.undo();
    assert_eq!(vm.count(&View::List(destination)), 0);
    assert_eq!(vm.count(&View::List(TASKS_LIST)), 3);

    vm.select_all_tasks(&ids[..2]);
    vm.delete_selection();
    assert_eq!(vm.panel(), Panel::DeleteSelection);
    assert_eq!(vm.state().tasks.len(), 3);
    vm.confirm_delete_selection();
    assert_eq!(vm.state().tasks.len(), 1);
    assert_eq!(vm.tasks.count(), 1);
    assert!(!vm.selection_mode());
    vm.undo();
    assert_eq!(vm.state().tasks.len(), 3);
    assert_eq!(vm.tasks.count(), 3);
    vm.state().validate().unwrap();
}

#[test]
fn navigation_clears_selection_cursor_and_range_anchor() {
    let vm = vm();
    let ids = add_tasks(&vm, 4);
    vm.toggle_task_selection(ids[0]);
    vm.select_task_range(&ids, ids[3]);
    assert_eq!(vm.selection_count(), 4);
    vm.navigate(View::Important);
    assert!(!vm.selection_mode());
    assert_eq!(vm.selection_count(), 0);
    assert_eq!(vm.task_cursor(), None);
    assert_eq!(vm.selected(), None);
    vm.navigate(View::List(TASKS_LIST));
    vm.set_task_cursor(Some(ids[2]));
    vm.select_task_range(&ids, ids[1]);
    assert_eq!(vm.selection().into_iter().collect::<Vec<_>>(), ids[1..=2]);
    vm.select(Some(ids[3]));
    assert!(!vm.selection_mode());
    assert_eq!(vm.selection_count(), 0);
}

#[test]
fn select_all_matches_collapsed_completed_rows_and_completed_view() {
    let vm = vm();
    let ids = add_tasks(&vm, 3);
    vm.dispatch(Action::ToggleCompleted(ids[1]));
    vm.toggle_completed();
    let visible = vm.visible_task_ids();
    assert_eq!(visible, vec![ids[0], ids[2]]);
    vm.select_all_tasks(&visible);
    assert_eq!(vm.selection_count(), 2);
    assert!(!vm.selection().contains(&ids[1]));
    vm.navigate(View::Completed);
    assert_eq!(vm.visible_task_ids(), vec![ids[1]]);
    vm.select_all_tasks(&vm.visible_task_ids());
    assert_eq!(vm.selection_count(), 1);
}

#[test]
fn invalid_move_preserves_selection_and_stale_range_ids_are_ignored() {
    let vm = vm();
    let ids = add_tasks(&vm, 3);
    vm.select_all_tasks(&ids[..2]);
    let selected = vm.selection();
    vm.move_selection(999_999);
    assert_eq!(vm.selection(), selected);
    assert!(vm.selection_mode());
    assert!(vm
        .state()
        .tasks
        .iter()
        .all(|task| task.list_id == TASKS_LIST));
    vm.end_selection();
    vm.select(Some(ids[0]));
    vm.select_task_range(&[ids[0], 999_999, ids[2]], ids[2]);
    assert_eq!(
        vm.selection().into_iter().collect::<Vec<_>>(),
        vec![ids[0], ids[2]]
    );
}

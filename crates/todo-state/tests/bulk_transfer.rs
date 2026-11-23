mod common;
use common::vm;
use todo_domain::{Action, SortOrder, View, TASKS_LIST};
use todo_state::gestures::drag::{DragInput, DragItem, DragOrigin};

#[test]
fn alphabetically_selected_tasks_move_in_display_order_and_undo_as_one_operation() {
    let vm = vm();
    for title in ["Zebra", "Apple", "Lemon"] {
        vm.add_task(title.into());
    }
    let ids: Vec<_> = vm.state().tasks.iter().map(|task| task.id).collect();
    vm.dispatch(Action::AddList {
        name: "Destination".into(),
    });
    let destination = vm.state().lists.last().unwrap().id;
    vm.navigate(View::List(TASKS_LIST));
    vm.set_sort(SortOrder::Alphabetical);
    vm.select_all_tasks(&ids);
    assert_eq!(vm.ordered_selection(), [ids[1], ids[2], ids[0]]);
    let before = vm.state().clone();
    vm.move_selection(destination);
    let moved: Vec<_> = vm
        .state()
        .visible_tasks(
            &View::List(destination),
            SortOrder::Manual,
            true,
            vm.today(),
        )
        .into_iter()
        .map(|task| task.id)
        .collect();
    assert_eq!(moved, [ids[1], ids[2], ids[0]]);
    assert!(!vm.selection_mode());
    vm.undo();
    assert_eq!(*vm.state(), before);
    vm.redo();
    assert!(vm
        .state()
        .tasks
        .iter()
        .all(|task| task.list_id == destination));
}

#[test]
fn mouse_drag_snapshots_every_selected_origin_and_does_not_revive_after_a_member_changes() {
    let vm = vm();
    for title in ["A", "B", "C"] {
        vm.add_task(title.into());
    }
    let ids = vm.visible_task_ids();
    vm.select_all_tasks(&[ids[0], ids[2]]);
    let origin = DragOrigin::capture(&vm, DragItem::Task(ids[0]), DragInput::Mouse).unwrap();
    assert_eq!(origin.task_ids(), [ids[0], ids[2]]);
    vm.dispatch(Action::RenameTask {
        id: ids[1],
        title: "Unrelated".into(),
    });
    assert!(origin.valid(&vm));
    vm.dispatch(Action::SetCompleted {
        id: ids[2],
        completed: true,
    });
    assert!(!origin.valid(&vm));
    vm.undo();
    assert!(!origin.valid(&vm));
}

#[test]
fn touch_hold_adds_its_source_in_display_order_then_cancels_on_later_selection_changes() {
    let vm = vm();
    for title in ["A", "B", "C"] {
        vm.add_task(title.into());
    }
    vm.navigate(View::List(TASKS_LIST));
    let ids = vm.visible_task_ids();
    vm.select_all_tasks(&[ids[0], ids[2]]);
    let origin = DragOrigin::capture(&vm, DragItem::Task(ids[1]), DragInput::Touch).unwrap();
    assert_eq!(origin.task_ids(), ids);
    vm.toggle_task_selection(ids[1]);
    assert!(origin.valid(&vm));
    vm.toggle_task_selection(ids[0]);
    assert!(!origin.valid(&vm));
    vm.toggle_task_selection(ids[0]);
    assert!(!origin.valid(&vm));
}

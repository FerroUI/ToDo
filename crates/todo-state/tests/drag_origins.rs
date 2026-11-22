mod common;
use common::vm;
use todo_domain::{Action, SidebarItem, SortOrder, View, TASKS_LIST};
use todo_state::gestures::drag::{DragInput, DragItem, DragOrigin};
use todo_state::Panel;

#[test]
fn interruption_is_terminal_even_when_the_original_route_returns() {
    let vm = vm();
    vm.add_task("Source".into());
    let id = vm.state().tasks[0].id;
    let origin = DragOrigin::capture(&vm, DragItem::Task(id), DragInput::Mouse).unwrap();
    vm.navigate(View::Important);
    assert!(!origin.valid(&vm));
    vm.navigate(View::MyDay);
    assert!(!origin.valid(&vm));
    assert!(!origin.finish());
}

#[test]
fn only_touch_hold_requires_a_manual_real_list_and_survives_selection() {
    let vm = vm();
    vm.add_task("Source".into());
    let id = vm.state().tasks[0].id;
    let item = DragItem::Task(id);
    assert!(DragOrigin::capture(&vm, item, DragInput::Touch).is_none());
    assert!(DragOrigin::capture(&vm, item, DragInput::Mouse).is_some());
    vm.navigate(View::List(TASKS_LIST));
    vm.set_sort(SortOrder::Alphabetical);
    assert!(DragOrigin::capture(&vm, item, DragInput::Touch).is_none());
    assert!(DragOrigin::capture(&vm, item, DragInput::Mouse).is_some());
    vm.set_sort(SortOrder::Manual);
    vm.select(Some(id));
    let touch = DragOrigin::capture(&vm, item, DragInput::Touch).unwrap();
    let mouse = DragOrigin::capture(&vm, item, DragInput::Mouse).unwrap();
    vm.toggle_task_selection(id);
    assert!(
        touch.valid(&vm),
        "selection may replace and recycle the source row"
    );
    assert!(!mouse.valid(&vm));
    assert!(touch.finish());
    assert!(!touch.finish(), "one physical release commits at most once");
}

#[test]
fn moved_completed_or_deleted_sources_do_not_acquire_new_drop_meaning() {
    let vm = vm();
    vm.add_task("Source".into());
    let id = vm.state().tasks[0].id;
    vm.dispatch(Action::AddList {
        name: "Destination".into(),
    });
    let destination = vm.state().lists.last().unwrap().id;
    vm.navigate(View::List(TASKS_LIST));
    let item = DragItem::Task(id);
    let moved = DragOrigin::capture(&vm, item, DragInput::Mouse).unwrap();
    vm.dispatch(Action::MoveTask {
        id,
        list_id: destination,
    });
    assert!(!moved.valid(&vm));
    vm.undo();
    assert!(!moved.valid(&vm));
    let completed = DragOrigin::capture(&vm, item, DragInput::Touch).unwrap();
    vm.dispatch(Action::SetCompleted {
        id,
        completed: true,
    });
    assert!(!completed.valid(&vm));
    vm.undo();
    let deleted = DragOrigin::capture(&vm, item, DragInput::Touch).unwrap();
    vm.dispatch(Action::DeleteTask(id));
    assert!(!deleted.valid(&vm));
}

#[test]
fn sidebar_membership_change_invalidates_but_unrelated_edits_do_not() {
    let vm = vm();
    vm.dispatch(Action::AddList {
        name: "Source".into(),
    });
    let id = vm.state().lists.last().unwrap().id;
    vm.dispatch(Action::AddGroup {
        name: "Group".into(),
    });
    let group = vm.state().groups[0].id;
    let item = DragItem::Sidebar(SidebarItem::List(id));
    let origin = DragOrigin::capture(&vm, item, DragInput::Touch).unwrap();
    vm.add_task("Unrelated task".into());
    assert!(origin.valid(&vm));
    vm.dispatch(Action::MoveList {
        id,
        group_id: Some(group),
    });
    assert!(!origin.valid(&vm));
    vm.undo();
    assert!(!origin.valid(&vm));
    assert!(DragOrigin::capture(
        &vm,
        DragItem::Sidebar(SidebarItem::List(TASKS_LIST)),
        DragInput::Mouse
    )
    .is_none());
}

#[test]
fn resize_refresh_sort_and_modal_changes_require_a_new_contact() {
    for interruption in 0..4 {
        let vm = vm();
        vm.add_task("Source".into());
        let id = vm.state().tasks[0].id;
        vm.navigate(View::List(TASKS_LIST));
        let origin = DragOrigin::capture(&vm, DragItem::Task(id), DragInput::Touch).unwrap();
        match interruption {
            0 => vm.set_viewport(vm.viewport() - 1.0),
            1 => vm.refresh(),
            2 => vm.set_sort(SortOrder::Alphabetical),
            _ => vm.show_panel(Panel::Settings),
        }
        assert!(!origin.valid(&vm));
    }
}

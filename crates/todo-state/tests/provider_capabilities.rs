mod common;
use common::today;
use ferroui_dynamic_data::IConnectableCache;
use ferroui_reactive::ObservableExt;
use std::{cell::Cell, rc::Rc};
use todo_domain::{Action, AppState, ExternalTask, Task, TaskList, View, TASKS_LIST};
use todo_state::{
    gestures::drag::{DragInput, DragItem, DragOrigin},
    AppViewModel, Panel,
};

fn providers() -> Rc<AppViewModel> {
    let mut state = AppState::default();
    let ordinary = Task::new(2, TASKS_LIST, "Ordinary".into(), today());
    let mut flagged = Task::new(3, TASKS_LIST, "Flagged email".into(), today());
    flagged.flagged_email = true;
    let mut planner = Task::new(4, TASKS_LIST, "Planner assignment".into(), today());
    planner.external = Some(ExternalTask {
        provider: "Planner".into(),
        id: "remote-one".into(),
        url: "https://planner.cloud.microsoft/".into(),
    });
    planner.assigned_to = Some("me".into());
    state.tasks = vec![ordinary, flagged, planner];
    state.lists.push(TaskList::new(5, "Destination".into()));
    state.next_id = 6;
    state.settings.planner_enabled = true;
    state.repair_sidebar_order();
    AppViewModel::new(state, today(), Rc::new(|_| Ok(())))
}

#[test]
fn protected_single_delete_never_opens_confirmation_or_announces_success() {
    let vm = providers();
    vm.set_status("Existing status".into());
    vm.delete_task(4);
    assert_eq!(vm.panel(), Panel::None);
    vm.show_panel(Panel::DeleteTask(4));
    vm.confirm_delete_task(4);
    assert_eq!(vm.panel(), Panel::DeleteTask(4));
    assert_eq!(vm.status(), "Existing status");
    assert!(!vm.status_can_undo());
    assert!(vm.state().task(4).is_some());
}

#[test]
fn mixed_protected_selections_keep_selection_and_document_when_move_or_delete_is_rejected() {
    for protected in [3, 4] {
        let vm = providers();
        vm.select_all_tasks(&[2, protected]);
        let selection = vm.selection();
        let original = vm.state().clone();
        vm.show_panel(Panel::MoveSelection);
        vm.move_selection(5);
        assert_eq!(vm.selection(), selection);
        assert!(vm.selection_mode());
        assert_eq!(vm.panel(), Panel::MoveSelection);
        assert_eq!(*vm.state(), original);
    }
    let vm = providers();
    vm.select_all_tasks(&[2, 4]);
    let selection = vm.selection();
    vm.delete_selection();
    assert_eq!(vm.panel(), Panel::None);
    vm.show_panel(Panel::DeleteSelection);
    vm.confirm_delete_selection();
    assert_eq!(vm.selection(), selection);
    assert!(vm.selection_mode());
    assert_eq!(vm.panel(), Panel::DeleteSelection);
    assert_eq!(vm.state().tasks.len(), 3);
    assert!(!vm.status_can_undo());
}

#[test]
fn flagged_email_deletion_and_ordinary_moves_remain_supported() {
    let vm = providers();
    vm.select_all_tasks(&[2]);
    vm.move_selection(5);
    assert_eq!(vm.state().task(2).unwrap().list_id, 5);
    assert!(!vm.selection_mode());
    vm.undo();
    assert_eq!(vm.state().task(2).unwrap().list_id, TASKS_LIST);
    vm.delete_task(3);
    assert_eq!(vm.panel(), Panel::DeleteTask(3));
    vm.confirm_delete_task(3);
    assert!(vm.state().task(3).is_none());
    assert!(vm.status_can_undo());
    vm.undo();
    assert!(vm.state().task(3).unwrap().flagged_email);
}

#[test]
fn mouse_and_held_drags_reject_protected_sources_and_mixed_selections_before_activation() {
    let vm = providers();
    vm.navigate(View::List(TASKS_LIST));
    for input in [DragInput::Mouse, DragInput::Touch] {
        for id in [3, 4] {
            assert!(DragOrigin::capture(&vm, DragItem::Task(id), input).is_none());
        }
        assert!(DragOrigin::capture(&vm, DragItem::Task(2), input).is_some());
    }
    for protected in [3, 4] {
        vm.select_all_tasks(&[2, protected]);
        assert!(DragOrigin::capture(&vm, DragItem::Task(2), DragInput::Touch).is_none());
    }
    vm.select_all_tasks(&[2]);
    assert!(DragOrigin::capture(&vm, DragItem::Task(2), DragInput::Touch).is_some());
}

#[test]
fn losing_move_capability_interrupts_a_drag_and_restoring_it_cannot_revive_the_contact() {
    let vm = providers();
    let origin = DragOrigin::capture(&vm, DragItem::Task(2), DragInput::Mouse).unwrap();
    let mut updated = vm.state().clone();
    updated
        .tasks
        .iter_mut()
        .find(|task| task.id == 2)
        .unwrap()
        .flagged_email = true;
    vm.replace_from_cloud(&updated.to_json().unwrap()).unwrap();
    assert!(!origin.valid(&vm));
    updated
        .tasks
        .iter_mut()
        .find(|task| task.id == 2)
        .unwrap()
        .flagged_email = false;
    vm.replace_from_cloud(&updated.to_json().unwrap()).unwrap();
    assert!(!origin.valid(&vm));
    assert!(!origin.finish());
}

#[test]
fn queued_delete_announces_success_only_after_its_own_action_applies() {
    let vm = providers();
    vm.set_status("Before deletion".into());
    vm.show_panel(Panel::DeleteTask(2));
    let weak = Rc::downgrade(&vm);
    let called = Rc::new(Cell::new(true));
    let once = called.clone();
    let _subscription = vm.tasks.connect().subscribe_next(move |_| {
        if once.replace(true) {
            return;
        }
        let vm = weak.upgrade().unwrap();
        vm.confirm_delete_task(2);
        assert!(
            vm.state().task(2).is_some(),
            "cache callbacks queue their command"
        );
        assert_eq!(vm.status(), "Before deletion");
    });
    // connect emits its initial cache immediately, so arm the callback afterward.
    called.set(false);
    vm.dispatch(Action::ToggleImportant(3));
    assert!(vm.state().task(2).is_none());
    assert_eq!(vm.status(), "Task deleted. You can undo this change.");
    assert_eq!(vm.panel(), Panel::None);
}

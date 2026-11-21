mod common;
use common::*;
use ferroui_dynamic_data::{ChangeReason, IConnectableCache, IObservableCache};
use ferroui_reactive::ObservableExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use todo_domain::*;

#[test]
fn smart_list_add_is_one_undoable_transaction() {
    for view in [
        View::MyDay,
        View::Important,
        View::Planned,
        View::AssignedToMe,
    ] {
        let vm = vm();
        vm.navigate(view.clone());
        vm.add_task("A new task".into());
        assert_eq!(vm.visible_tasks().len(), 1, "view {view:?}");
        assert_eq!(vm.tasks.count(), 1);
        vm.undo();
        assert!(vm.state().tasks.is_empty());
        assert_eq!(vm.tasks.count(), 0);
        vm.redo();
        assert_eq!(vm.visible_tasks().len(), 1);
        vm.state().validate().unwrap();
    }
}

#[test]
fn reactive_cache_publishes_only_the_affected_entity() {
    let vm = vm();
    vm.add_task("First".into());
    vm.add_task("Second".into());
    let id = vm.state().tasks[0].id;
    let changes = Rc::new(RefCell::new(Vec::new()));
    let output = changes.clone();
    let _subscription = vm.tasks.connect().subscribe_next(move |set| {
        output.borrow_mut().push(
            set.iter()
                .map(|change| (change.reason(), *change.key()))
                .collect::<Vec<_>>(),
        );
    });
    changes.borrow_mut().clear();
    vm.dispatch(Action::ToggleImportant(id));
    assert_eq!(*changes.borrow(), vec![vec![(ChangeReason::Update, id)]]);
    assert!(vm.tasks.lookup(&id).unwrap().important);
    vm.undo();
    assert!(!vm.tasks.lookup(&id).unwrap().important);
}

#[test]
fn cache_callbacks_can_dispatch_without_borrowing_panics() {
    let vm = vm();
    let weak = Rc::downgrade(&vm);
    let called = Rc::new(Cell::new(false));
    let once = called.clone();
    let _subscription = vm.tasks.connect().subscribe_next(move |changes| {
        if once.replace(true) {
            return;
        }
        let id = *changes.iter().next().unwrap().key();
        weak.upgrade()
            .unwrap()
            .dispatch(Action::ToggleImportant(id));
    });
    vm.add_task("Reentrant update".into());
    assert!(called.get());
    let task = vm.state().tasks[0].clone();
    assert!(task.important);
    assert!(vm.tasks.lookup(&task.id).unwrap().important);
}

#[test]
fn queued_smart_list_adds_allocate_membership_for_their_own_task() {
    let vm = vm();
    let weak = Rc::downgrade(&vm);
    let once = Rc::new(Cell::new(false));
    let _subscription = vm.tasks.connect().subscribe_next(move |_| {
        if once.replace(true) {
            return;
        }
        let vm = weak.upgrade().unwrap();
        vm.add_task("Queued first".into());
        vm.add_task("Queued second".into());
    });
    vm.add_task("Trigger".into());
    assert_eq!(vm.visible_tasks().len(), 3);
    assert_eq!(vm.state().tasks.len(), 3);
    vm.state().validate().unwrap();
}

#[test]
fn undo_restores_deleted_lists_and_steps_and_clears_invalid_navigation() {
    let vm = vm();
    vm.dispatch(Action::AddList {
        name: "Work".into(),
    });
    let list = vm.view().destination_list();
    vm.add_task("Ship".into());
    let id = vm.state().tasks[0].id;
    vm.dispatch(Action::AddStep {
        task_id: id,
        title: "Review".into(),
    });
    vm.select(Some(id));
    vm.dispatch(Action::DeleteList(list));
    assert_eq!(vm.selected(), None);
    assert_eq!(vm.view(), View::List(TASKS_LIST));
    vm.undo();
    assert_eq!(vm.state().task(id).unwrap().steps.len(), 1);
    assert_eq!(vm.tasks.lookup(&id).unwrap().list_id, list);
    vm.state().validate().unwrap();
    vm.redo();
    assert_eq!(vm.tasks.count(), 0);
    assert!(vm.state().list(list).is_none());
}

#[test]
fn completed_count_planned_order_and_manual_movement_follow_the_model() {
    let vm = vm();
    vm.navigate(View::List(TASKS_LIST));
    for name in ["A", "B", "C"] {
        vm.add_task(name.into());
    }
    let ids: Vec<_> = vm.state().tasks.iter().map(|task| task.id).collect();
    vm.move_task_by(ids[0], 1);
    assert_eq!(
        vm.visible_tasks()
            .iter()
            .map(|task| task.id)
            .collect::<Vec<_>>(),
        vec![ids[1], ids[0], ids[2]]
    );
    vm.move_task_by(ids[0], 1);
    assert_eq!(
        vm.visible_tasks()
            .iter()
            .map(|task| task.id)
            .collect::<Vec<_>>(),
        vec![ids[1], ids[2], ids[0]]
    );
    vm.dispatch(Action::ToggleCompleted(ids[0]));
    assert_eq!(vm.count(&View::Completed), 1);
    assert_eq!(vm.count(&View::All), 2);
    vm.navigate(View::Planned);
    assert_eq!(vm.sort(), SortOrder::DueDate);
}

#[test]
fn theme_preferences_round_trip_and_system_dark_changes_are_observable() {
    let vm = vm();
    vm.set_smart_color("#735CA9".into());
    assert_eq!(vm.accent(), "#735CA9");
    let saved = AppState::from_json(&vm.export_json().unwrap()).unwrap();
    assert_eq!(saved.settings.smart_colors["My Day"], "#735CA9");
    let revision = vm.revision();
    vm.set_system_dark(true);
    assert!(vm.is_dark());
    assert!(vm.revision() > revision);
    vm.undo();
    assert_eq!(vm.accent(), "#58699D");
}

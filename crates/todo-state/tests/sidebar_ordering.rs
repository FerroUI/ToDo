mod common;

use common::today;
use ferroui_dynamic_data::IConnectableCache;
use ferroui_reactive::ObservableExt;
use std::{cell::Cell, rc::Rc};
use todo_domain::*;
use todo_state::AppViewModel;

fn fixture() -> AppState {
    let mut state = AppState::new();
    for id in [2, 3, 4, 9] {
        let mut list = TaskList::new(id, format!("List {id}"));
        list.group_id = Some(if id == 9 { 6 } else { 5 });
        state.lists.push(list);
    }
    for id in 5..=7 {
        state.groups.push(ListGroup {
            id,
            name: format!("Group {id}"),
            collapsed: false,
            position: id as i64,
        });
    }
    state
        .tasks
        .push(Task::new(8, TASKS_LIST, "Unrelated task".into(), today()));
    state.next_id = 10;
    state.validate().unwrap();
    state
}

fn ordered_lists(vm: &AppViewModel, group_id: Id) -> Vec<Id> {
    let mut values: Vec<_> = vm
        .state()
        .lists
        .iter()
        .filter(|list| list.group_id == Some(group_id))
        .map(|list| (list.position, list.id))
        .collect();
    values.sort_unstable();
    values.into_iter().map(|(_, id)| id).collect()
}

fn ordered_groups(vm: &AppViewModel) -> Vec<Id> {
    let mut values: Vec<_> = vm
        .state()
        .groups
        .iter()
        .map(|group| (group.position, group.id))
        .collect();
    values.sort_unstable();
    values.into_iter().map(|(_, id)| id).collect()
}

#[test]
fn explicit_list_movement_keeps_navigation_and_avoids_task_cache_updates() {
    let initial = fixture();
    let vm = AppViewModel::new(initial.clone(), today(), Rc::new(|_| Ok(())));
    vm.navigate(View::List(TASKS_LIST));
    vm.select(Some(8));
    let notifications = Rc::new(Cell::new(0));
    let count = notifications.clone();
    let _subscription = vm
        .tasks
        .connect()
        .subscribe_next(move |_| count.set(count.get() + 1));
    notifications.set(0);
    let revision = vm.revision();

    vm.move_list_by(4, -2);
    assert_eq!(ordered_lists(&vm, 5), vec![4, 2, 3]);
    assert_eq!(vm.view(), View::List(TASKS_LIST));
    assert_eq!(vm.selected(), Some(8));
    assert!(vm.revision() > revision);
    assert_eq!(notifications.get(), 0);
    assert_eq!(vm.state().tasks, initial.tasks);
    vm.undo();
    assert_eq!(*vm.state(), initial);
    assert!(!vm.can_undo());
    vm.redo();
    assert_eq!(ordered_lists(&vm, 5), vec![4, 2, 3]);
    assert_eq!(notifications.get(), 0);

    let revision = vm.revision();
    for (id, delta) in [(4, -1), (3, 1), (2, 0), (999, 1), (TASKS_LIST, 1)] {
        vm.move_list_by(id, delta);
    }
    assert_eq!(vm.revision(), revision);
    vm.undo();
    assert_eq!(*vm.state(), initial);
    assert!(!vm.can_undo());
}

#[test]
fn group_movement_is_one_undoable_change_and_boundaries_do_not_save() {
    let initial = fixture();
    let saves = Rc::new(Cell::new(0));
    let count = saves.clone();
    let vm = AppViewModel::new(
        initial.clone(),
        today(),
        Rc::new(move |_| {
            count.set(count.get() + 1);
            Ok(())
        }),
    );
    vm.move_group_by(7, -2);
    assert_eq!(ordered_groups(&vm), vec![7, 5, 6]);
    assert_eq!(vm.state().lists, initial.lists);
    assert_eq!(saves.get(), 1);
    vm.move_group_by(7, i32::MIN);
    vm.move_group_by(6, i32::MAX);
    vm.move_group_by(999, 1);
    vm.move_group_by(5, 0);
    assert_eq!(saves.get(), 1);
    vm.undo();
    assert_eq!(*vm.state(), initial);
    assert!(!vm.can_undo());
    vm.redo();
    assert_eq!(ordered_groups(&vm), vec![7, 5, 6]);
    vm.move_group_by(7, i32::MAX);
    assert_eq!(ordered_groups(&vm), vec![5, 6, 7]);
}

#[test]
fn a_cross_group_drop_and_its_undo_restore_membership_and_order_together() {
    let initial = fixture();
    let vm = AppViewModel::new(initial.clone(), today(), Rc::new(|_| Ok(())));
    vm.dispatch(Action::Batch(vec![
        Action::MoveList {
            id: 2,
            group_id: Some(6),
        },
        Action::ReorderList {
            id: 2,
            before: Some(9),
        },
    ]));
    assert_eq!(ordered_lists(&vm, 5), vec![3, 4]);
    assert_eq!(ordered_lists(&vm, 6), vec![2, 9]);
    vm.undo();
    assert_eq!(*vm.state(), initial);
    assert!(!vm.can_undo());
    vm.redo();
    assert_eq!(ordered_lists(&vm, 6), vec![2, 9]);
    vm.undo();

    vm.dispatch(Action::Batch(vec![
        Action::MoveList {
            id: 2,
            group_id: Some(6),
        },
        Action::ReorderList {
            id: 2,
            before: Some(3),
        },
    ]));
    assert_eq!(*vm.state(), initial);
    assert!(!vm.can_undo());
    assert!(vm.can_redo());
    assert!(vm.status().contains("Move the list into this group"));
}

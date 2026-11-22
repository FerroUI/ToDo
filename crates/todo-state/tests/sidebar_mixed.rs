mod common;
use common::today;
use std::{cell::RefCell, rc::Rc};
use todo_domain::*;
use todo_state::AppViewModel;

fn fixture() -> AppState {
    let mut state = AppState::new();
    state
        .apply(
            Action::AddList {
                name: "First".into(),
            },
            today(),
        )
        .unwrap();
    state
        .apply(
            Action::AddGroup {
                name: "Group".into(),
            },
            today(),
        )
        .unwrap();
    state
        .apply(
            Action::AddList {
                name: "Last".into(),
            },
            today(),
        )
        .unwrap();
    state
}

#[test]
fn mixed_keyboard_order_and_membership_restore_as_one_undo_step() {
    let initial = fixture();
    let vm = AppViewModel::new(initial.clone(), today(), Rc::new(|_| Ok(())));
    vm.move_group_by(3, 1);
    assert_eq!(vm.state().sidebar_ids(), vec![2, 4, 3]);
    vm.undo();
    assert_eq!(*vm.state(), initial);
    vm.redo();
    vm.move_list_by(2, 2);
    assert_eq!(vm.state().sidebar_ids(), vec![4, 3, 2]);
    let before = vm.state().clone();
    let action = vm
        .state()
        .sidebar_drop_action(SidebarItem::List(2), SidebarDrop::IntoGroup(3))
        .unwrap();
    vm.dispatch(action);
    assert_eq!(vm.state().sidebar_ids(), vec![4, 3]);
    vm.undo();
    assert_eq!(*vm.state(), before);
    vm.redo();
    vm.dispatch(Action::DeleteGroup(3));
    assert_eq!(vm.state().sidebar_ids(), vec![4, 2]);
    vm.undo();
    assert_eq!(vm.state().sidebar_ids(), vec![4, 3]);
    assert_eq!(vm.state().list(2).unwrap().group_id, Some(3));
}

#[test]
fn import_undo_restores_order_while_cloud_replacement_keeps_local_preference() {
    let initial = fixture();
    let saved = Rc::new(RefCell::new(String::new()));
    let output = saved.clone();
    let vm = AppViewModel::new(
        initial.clone(),
        today(),
        Rc::new(move |json| {
            *output.borrow_mut() = json.into();
            Ok(())
        }),
    );
    vm.move_group_by(3, -1);
    let local = vm.state().clone();
    vm.import_json(&initial.to_json().unwrap()).unwrap();
    assert_eq!(vm.state().sidebar_ids(), vec![2, 3, 4]);
    vm.undo();
    assert_eq!(*vm.state(), local);
    let mut cloud = serde_json::to_value(&initial).unwrap();
    cloud.as_object_mut().unwrap().remove("sidebar_order");
    let before_prepare = vm.state().clone();
    let history = vm.can_undo();
    let canonical = vm.prepare_cloud_json(&cloud.to_string()).unwrap();
    assert_eq!(
        *vm.state(),
        before_prepare,
        "journal preflight must be read-only"
    );
    assert_eq!(vm.can_undo(), history);
    vm.replace_from_cloud(&cloud.to_string()).unwrap();
    assert_eq!(vm.state().sidebar_ids(), vec![3, 2, 4]);
    assert_eq!(
        AppState::from_json(&saved.borrow()).unwrap().sidebar_ids(),
        vec![3, 2, 4]
    );
    assert_eq!(
        *saved.borrow(),
        canonical,
        "journal data must match the actual durable write"
    );
}

mod common;
use common::*;
use std::{cell::Cell, rc::Rc};
use todo_domain::{AppState, View, TASKS_LIST};
use todo_state::{AppViewModel, RenameTarget};

#[test]
fn create_list_starts_a_transient_inline_transaction_and_commits_once() {
    let saves = Rc::new(Cell::new(0));
    let count = saves.clone();
    let vm = AppViewModel::new(
        AppState::new(),
        today(),
        Rc::new(move |_| {
            count.set(count.get() + 1);
            Ok(())
        }),
    );
    vm.create_list_inline();
    let edit = vm.rename_session().unwrap();
    let RenameTarget::List(id) = edit.target else {
        panic!("list editor")
    };
    assert_eq!(vm.view(), View::List(id));
    assert_eq!(vm.state().list(id).unwrap().name, "Untitled list");
    let revision = vm.revision();
    let saved = saves.get();
    vm.set_rename_draft(edit.target, "  Weekend plans  ".into());
    assert_eq!(vm.revision(), revision, "typing must not rebuild the shell");
    assert_eq!(saves.get(), saved, "typing must not persist a document");
    assert!(!vm.export_json().unwrap().contains("Weekend plans"));
    vm.commit_rename(edit.target);
    vm.commit_rename(edit.target); // LostFocus after Enter is idempotent.
    assert_eq!(saves.get(), saved + 1);
    assert_eq!(vm.state().list(id).unwrap().name, "Weekend plans");
    assert!(vm.rename_session().is_none());
    vm.undo();
    assert_eq!(vm.state().list(id).unwrap().name, "Untitled list");
}

#[test]
fn escape_blank_names_and_stale_callbacks_preserve_existing_names() {
    let vm = vm();
    vm.create_list_inline();
    let first = vm.rename_session().unwrap().target;
    vm.set_rename_draft(first, "Discarded".into());
    vm.cancel_rename(first);
    vm.commit_rename(first);
    let RenameTarget::List(id) = first else {
        unreachable!()
    };
    assert_eq!(vm.state().list(id).unwrap().name, "Untitled list");
    vm.create_group_inline();
    let second = vm.rename_session().unwrap().target;
    vm.set_rename_draft(first, "Stale event".into());
    vm.cancel_rename(first);
    vm.commit_rename(first);
    assert_eq!(vm.rename_session().unwrap().target, second);
    vm.set_rename_draft(second, "  ".into());
    vm.commit_rename(second);
    assert!(vm.rename_session().is_none());
    assert_eq!(vm.state().groups[0].name, "Untitled group");
}

#[test]
fn group_creation_reveals_navigation_without_replacing_the_current_task() {
    let vm = vm();
    vm.add_task("Keep editing".into());
    let id = vm.state().tasks[0].id;
    vm.select(Some(id));
    vm.set_viewport(390.0);
    let view = vm.view();
    vm.create_group_inline();
    assert_eq!(vm.view(), view);
    assert_eq!(vm.selected(), Some(id));
    assert!(vm.navigation_overlay_open());
    assert!(matches!(
        vm.rename_session().unwrap().target,
        RenameTarget::Group(_)
    ));
}

#[test]
fn protected_and_removed_targets_cannot_retain_an_editor() {
    let vm = vm();
    vm.begin_rename(RenameTarget::List(TASKS_LIST));
    vm.begin_rename(RenameTarget::Group(u64::MAX));
    assert!(vm.rename_session().is_none());
    vm.create_list_inline();
    vm.undo();
    assert!(vm.rename_session().is_none());
    vm.create_group_inline();
    vm.navigate(View::Important);
    assert!(vm.rename_session().is_none());
}

mod common;
use common::*;
use std::{cell::RefCell, rc::Rc};
use todo_domain::{Action, View};
use todo_state::{HostRequest, HostServices};

#[test]
fn back_skips_deleted_lists_and_search_typing_is_one_visit() {
    let vm = vm();
    vm.dispatch(Action::AddList {
        name: "Temporary".into(),
    });
    let id = vm
        .state()
        .lists
        .iter()
        .find(|list| list.name == "Temporary")
        .unwrap()
        .id;
    vm.navigate(View::List(id));
    vm.navigate(View::Search("a".into()));
    vm.navigate(View::Search("ab".into()));
    vm.return_from_search(View::List(id));
    vm.navigate(View::Important);
    vm.dispatch(Action::DeleteList(id));
    assert!(vm.go_back_view());
    assert_eq!(vm.view(), View::MyDay);
    assert!(!vm.go_back_view());
}

#[test]
fn refresh_is_single_flight_and_never_replaces_local_document_or_navigation() {
    let vm = vm();
    vm.add_task("Keep this draft's saved task".into());
    vm.navigate(View::Important);
    let before = vm.export_json().unwrap();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let record = requests.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| record.borrow_mut().push(request)),
    });
    vm.refresh();
    vm.refresh();
    assert!(vm.refreshing());
    assert_eq!(requests.borrow().len(), 1);
    assert!(matches!(
        requests.borrow()[0],
        HostRequest::Refresh { sync: false }
    ));
    assert_eq!(vm.export_json().unwrap(), before);
    assert_eq!(vm.view(), View::Important);
    vm.finish_refresh();
    vm.set_cloud_state(true, "Connected");
    vm.refresh();
    assert!(matches!(
        requests.borrow()[1],
        HostRequest::Refresh { sync: true }
    ));
    vm.finish_refresh();
    assert!(!vm.refreshing());
}

#[test]
fn status_undo_is_reserved_for_its_original_change_and_never_shown_on_errors() {
    let vm = vm();
    vm.add_task("Delete this task".into());
    let id = vm.state().tasks[0].id;
    vm.confirm_delete_task(id);
    assert!(vm.status_can_undo());
    vm.navigate(View::Important);
    assert!(
        vm.status_can_undo(),
        "navigation does not replace the undoable edit"
    );
    vm.add_task("A later change".into());
    assert!(
        !vm.status_can_undo(),
        "an old notification must not undo a newer edit"
    );
    vm.report_error("Notifications are blocked".into());
    assert!(vm.can_undo());
    assert!(
        !vm.status_can_undo(),
        "an informational error does not expose an unrelated Undo action"
    );
}

#[test]
fn repeated_history_messages_still_notify_navigation_and_non_cache_views() {
    let vm = vm();
    let initial = vm.state().lists.len();
    for name in ["First list", "Second list"] {
        vm.dispatch(Action::AddList { name: name.into() });
    }
    for removed in 1..=2 {
        let revision = vm.revision();
        vm.undo();
        assert_eq!(vm.state().lists.len(), initial + 2 - removed);
        assert!(
            vm.revision() > revision,
            "each undo must reconcile non-cache UI even when its status text repeats"
        );
    }
    for restored in 1..=2 {
        let revision = vm.revision();
        vm.redo();
        assert_eq!(vm.state().lists.len(), initial + restored);
        assert!(
            vm.revision() > revision,
            "each redo must reconcile non-cache UI even when its status text repeats"
        );
    }
}

mod common;
use common::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use todo_domain::{Action, AppState, Reminder, View, TASKS_LIST};
use todo_state::{AppViewModel, HostRequest, HostServices};

#[test]
fn account_switch_clears_entity_scoped_history_selection_clipboard_and_reminder_delivery() {
    let vm = vm();
    vm.add_task("Alice reminder".into());
    let id = vm.state().tasks[0].id;
    vm.dispatch(Action::SetReminder {
        id,
        reminder: Some(Reminder {
            date: today(),
            minutes_after_midnight: 600,
            notified: false,
        }),
    });
    let notifications = Rc::new(RefCell::new(Vec::new()));
    let delivered = notifications.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| {
            if let HostRequest::Notify { body, .. } = request {
                delivered.borrow_mut().push(body);
            }
        }),
    });
    vm.tick(today(), 600);
    assert_eq!(notifications.borrow().len(), 1);
    vm.navigate(View::List(TASKS_LIST));
    vm.select_all_tasks(&[id]);
    let offer = vm.prepare_task_clipboard(&[id], true).unwrap();
    let payload = offer.data.clone();
    vm.accept_task_clipboard(offer).unwrap();

    let mut bob = vm.state().clone();
    bob.tasks[0].title = "Bob reminder".into();
    bob.tasks[0].reminder.as_mut().unwrap().notified = false;
    vm.replace_account_document(&bob.to_json().unwrap())
        .unwrap();
    assert_eq!(*vm.state(), bob);
    assert!(!vm.can_undo());
    assert!(!vm.can_redo());
    assert!(vm.selection().is_empty());
    assert!(!vm.selection_mode());
    assert_eq!(vm.selected(), None);
    assert_eq!(vm.task_cursor(), None);
    assert_eq!(vm.view(), View::MyDay);
    assert!(vm
        .paste_task_clipboard(
            vm.task_paste_target(View::List(TASKS_LIST)),
            Some(payload),
            ""
        )
        .is_err());
    vm.tick(today(), 600);
    assert_eq!(
        notifications.borrow().len(),
        2,
        "same task ID and civil time in another account is an independent reminder"
    );
    vm.undo();
    assert_eq!(vm.state().tasks[0].title, "Bob reminder");
}

#[test]
fn failed_account_save_retains_original_history_and_task_document() {
    let failure = Rc::new(Cell::new(false));
    let fail = failure.clone();
    let vm = AppViewModel::new(
        AppState::new(),
        today(),
        Rc::new(move |_| {
            if fail.get() {
                Err("Disk full".into())
            } else {
                Ok(())
            }
        }),
    );
    vm.add_task("Alice edit".into());
    let before = vm.export_json().unwrap();
    failure.set(true);
    assert!(vm
        .replace_account_document(&AppState::new().to_json().unwrap())
        .is_err());
    assert_eq!(vm.export_json().unwrap(), before);
    assert!(vm.can_undo());
    failure.set(false);
    vm.undo();
    assert!(vm.state().tasks.is_empty());
}

#[test]
fn incoming_sidebar_order_is_preserved_across_account_entity_collisions() {
    let vm = vm();
    vm.dispatch(Action::AddList {
        name: "Alice first".into(),
    });
    vm.dispatch(Action::AddList {
        name: "Alice second".into(),
    });
    let mut incoming = vm.state().clone();
    incoming.sidebar_order.reverse();
    incoming.lists[1].name = "Bob second".into();
    vm.replace_account_document(&incoming.to_json().unwrap())
        .unwrap();
    assert_eq!(vm.state().sidebar_order, incoming.sidebar_order);
    assert_eq!(vm.state().lists[1].name, "Bob second");
}

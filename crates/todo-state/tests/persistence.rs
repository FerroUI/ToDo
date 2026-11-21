mod common;
use common::*;
use ferroui_dynamic_data::IObservableCache;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use todo_domain::*;
use todo_state::{AppViewModel, HostRequest, HostServices};

#[test]
fn save_failure_stays_visible_through_undo_and_successful_save_recovers() {
    let fails = Rc::new(Cell::new(true));
    let fail = fails.clone();
    let vm = AppViewModel::new(
        AppState::new(),
        today(),
        Rc::new(move |_| {
            if fail.get() {
                Err("Quota exceeded".into())
            } else {
                Ok(())
            }
        }),
    );
    vm.add_task("Keep in memory".into());
    assert_eq!(vm.state().tasks.len(), 1);
    assert!(vm.status().contains("Quota exceeded"));
    vm.undo();
    assert!(vm.status().contains("Quota exceeded"));
    fails.set(false);
    vm.redo();
    assert_eq!(vm.status(), "Change restored");
    assert_eq!(vm.state().tasks.len(), 1);
}

#[test]
fn imports_validate_then_save_before_replacing_the_current_document() {
    let vm = AppViewModel::new(
        AppState::new(),
        today(),
        Rc::new(|_| Err("Storage unavailable".into())),
    );
    let before = vm.export_json().unwrap();
    let imported = AppState::sample(today()).to_json().unwrap();
    assert!(vm.import_json("not JSON").is_err());
    assert_eq!(vm.export_json().unwrap(), before);
    assert!(vm.import_json(&imported).is_err());
    assert_eq!(vm.export_json().unwrap(), before);
    assert!(vm.replace_from_cloud(&imported).is_err());
    assert_eq!(vm.export_json().unwrap(), before);
}

#[test]
fn identical_cloud_data_must_save_an_unsaved_local_document() {
    let vm = AppViewModel::new(
        AppState::new(),
        today(),
        Rc::new(|_| Err("Disk full".into())),
    );
    vm.add_task("Unsaved".into());
    assert!(vm.replace_from_cloud(&vm.export_json().unwrap()).is_err());
    assert!(vm.status().contains("Disk full"));
}

#[test]
fn cloud_no_op_does_not_write_or_add_undo_and_import_cache_is_exact() {
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
    vm.replace_from_cloud(&vm.export_json().unwrap()).unwrap();
    assert_eq!(saves.get(), 0);
    assert!(!vm.can_undo());
    let imported = AppState::sample(today());
    vm.import_json(&imported.to_json().unwrap()).unwrap();
    assert_eq!(vm.tasks.count(), imported.tasks.len());
    vm.undo();
    assert_eq!(vm.tasks.count(), 0);
    vm.redo();
    assert_eq!(vm.tasks.count(), imported.tasks.len());
    vm.set_cloud_state(true, "Signed in");
    assert!(vm.cloud_connected());
    assert_eq!(vm.cloud_label(), "Signed in");
}

#[test]
fn reminders_are_marked_before_callbacks_and_undo_cannot_redeliver_them() {
    let vm = vm();
    vm.add_task("Reminder".into());
    let id = vm.state().tasks[0].id;
    vm.dispatch(Action::SetReminder {
        id,
        reminder: Some(Reminder {
            date: today(),
            minutes_after_midnight: 600,
            notified: false,
        }),
    });
    vm.dispatch(Action::ToggleImportant(id));
    let delivered = Rc::new(Cell::new(0));
    let count = delivered.clone();
    let weak = Rc::downgrade(&vm);
    vm.set_services(HostServices {
        request: Rc::new(move |request| {
            if matches!(request, HostRequest::Notify { .. }) {
                count.set(count.get() + 1);
                weak.upgrade().unwrap().tick(today(), 601);
            }
        }),
    });
    vm.tick(today(), 601);
    assert_eq!(delivered.get(), 1);
    assert!(vm.tasks.lookup(&id).unwrap().reminder.unwrap().notified);
    vm.undo();
    assert!(!vm.state().task(id).unwrap().important);
    vm.tick(today(), 602);
    assert_eq!(delivered.get(), 1);
}

#[test]
fn host_callbacks_can_change_services_and_print_escapes_user_markup() {
    let vm = vm();
    vm.add_task("<script>alert('x')</script>".into());
    let output = Rc::new(RefCell::new(String::new()));
    let target = output.clone();
    let weak = Rc::downgrade(&vm);
    vm.set_services(HostServices {
        request: Rc::new(move |request| {
            if let HostRequest::Print { html, .. } = request {
                *target.borrow_mut() = html;
            }
            weak.upgrade()
                .unwrap()
                .set_services(HostServices::default());
        }),
    });
    vm.print_list();
    assert!(output.borrow().contains("&lt;script&gt;"));
    assert!(!output.borrow().contains("<script>"));
}

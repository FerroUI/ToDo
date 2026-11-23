mod common;

use common::today;
use std::{cell::RefCell, rc::Rc};
use todo_domain::{Action, AppState, Reminder};
use todo_state::{AppViewModel, HostRequest, HostServices};

#[test]
fn push_status_is_host_only_and_rejects_accidental_private_fields() {
    let saved = Rc::new(RefCell::new(Vec::<String>::new()));
    let writes = saved.clone();
    let vm = AppViewModel::new(
        AppState::new(),
        today(),
        Rc::new(move |value| {
            writes.borrow_mut().push(value.to_owned());
            Ok(())
        }),
    );
    let original = vm.export_json().unwrap();
    vm.set_push_state(r#"{"available":true,"supported":true,"enabled":true,"service_url":"https://reminders.example.test/","public_key":"public","status":"Scheduled"}"#).unwrap();
    assert!(vm.push_state().enabled);
    assert_eq!(vm.export_json().unwrap(), original);
    assert!(saved.borrow().is_empty());
    assert!(vm
        .set_push_state(r#"{"capability":"private-secret"}"#)
        .is_err());
    assert!(vm
        .set_push_state(r#"{"enrollment_key":"private-secret"}"#)
        .is_err());
    assert_eq!(vm.push_state().status, "Scheduled");
}

#[test]
fn foreground_notifications_identify_the_exact_persisted_reminder_occurrence() {
    let notifications = Rc::new(RefCell::new(Vec::new()));
    let emitted = notifications.clone();
    let vm = AppViewModel::new(AppState::new(), today(), Rc::new(|_| Ok(())));
    vm.set_services(HostServices {
        request: Rc::new(move |request| {
            if let HostRequest::Notify {
                task_id,
                date,
                minutes,
                title,
                ..
            } = request
            {
                emitted.borrow_mut().push((task_id, date, minutes, title));
            }
        }),
    });
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
    vm.tick(today(), 601);
    assert_eq!(
        &*notifications.borrow(),
        &[(id, today(), 600, "Reminder".to_owned())]
    );
    assert!(
        vm.state()
            .task(id)
            .unwrap()
            .reminder
            .as_ref()
            .unwrap()
            .notified
    );
    vm.tick(today(), 602);
    assert_eq!(notifications.borrow().len(), 1);
}

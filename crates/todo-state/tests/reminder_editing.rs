mod common;

use common::today;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use todo_domain::{Action, AppState, Reminder};
use todo_state::{AppViewModel, HostRequest, HostServices};

fn session(
    state: AppState,
    saved: Rc<RefCell<String>>,
    notifications: Rc<Cell<u32>>,
) -> Rc<AppViewModel> {
    let vm = AppViewModel::new(
        state,
        today(),
        Rc::new(move |contents| {
            *saved.borrow_mut() = contents.to_owned();
            Ok(())
        }),
    );
    vm.set_services(HostServices {
        request: Rc::new(move |request| {
            if matches!(request, HostRequest::Notify { .. }) {
                notifications.set(notifications.get() + 1);
            }
        }),
    });
    vm
}

fn reminder(minutes: u16, notified: bool) -> Option<Reminder> {
    Some(Reminder {
        date: today(),
        minutes_after_midnight: minutes,
        notified,
    })
}

#[test]
fn unchanged_reminder_save_survives_reload_without_another_notification() {
    let saved = Rc::new(RefCell::new(String::new()));
    let notifications = Rc::new(Cell::new(0));
    let vm = session(AppState::new(), saved.clone(), notifications.clone());
    vm.add_task("Reminder".into());
    let id = vm.state().tasks[0].id;
    vm.dispatch(Action::SetReminder {
        id,
        reminder: reminder(600, false),
    });
    vm.tick(today(), 600);
    assert_eq!(notifications.get(), 1);

    // A dialog save sends an undelivered value; keeping the same schedule is a no-op.
    vm.dispatch(Action::SetReminder {
        id,
        reminder: reminder(600, false),
    });
    assert!(
        vm.state()
            .task(id)
            .unwrap()
            .reminder
            .as_ref()
            .unwrap()
            .notified
    );
    let restored = AppState::from_json(&saved.borrow()).unwrap();
    assert!(
        restored
            .task(id)
            .unwrap()
            .reminder
            .as_ref()
            .unwrap()
            .notified
    );
    let reloaded = session(restored, saved, notifications.clone());
    reloaded.tick(today(), 601);
    assert_eq!(notifications.get(), 1);

    // A changed schedule remains armed and fires normally.
    reloaded.dispatch(Action::SetReminder {
        id,
        reminder: reminder(602, false),
    });
    reloaded.tick(today(), 602);
    assert_eq!(notifications.get(), 2);

    // The clock's explicit acknowledgement must still be able to mark delivery.
    reloaded.dispatch(Action::SetReminder {
        id,
        reminder: reminder(603, false),
    });
    reloaded.dispatch(Action::SetReminder {
        id,
        reminder: reminder(603, true),
    });
    reloaded.tick(today(), 603);
    assert_eq!(notifications.get(), 2);
}

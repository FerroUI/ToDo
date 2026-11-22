mod common;
use common::*;
use ferroui_dynamic_data::IConnectableCache;
use ferroui_reactive::ObservableExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use todo_domain::{Date, RepeatUnit};
use todo_state::{HostRequest, HostServices, Panel, parse_reminder_time, reminder_in_one_hour};

#[test]
fn reminder_time_validation_and_midnight_rollover_are_explicit() {
    assert_eq!(parse_reminder_time(" 9:05 "), Ok(545));
    assert_eq!(parse_reminder_time("00:00"), Ok(0));
    assert_eq!(parse_reminder_time("23:59"), Ok(1439));
    for invalid in [
        "", "24:00", "09:60", "9:5", "-1:00", "+9:00", "09:00:00", "9am",
    ] {
        assert!(parse_reminder_time(invalid).is_err(), "{invalid}");
    }
    let reminder = reminder_in_one_hour(today(), 23 * 60 + 45).unwrap();
    assert_eq!(reminder.date, today().succ_opt().unwrap());
    assert_eq!(reminder.minutes_after_midnight, 45);
    assert!(!reminder.notified);
    assert!(reminder_in_one_hour(Date::MAX, 1430).is_none());
}

#[test]
fn invalid_schedule_preserves_the_document_dialog_and_host_permissions() {
    let vm = vm();
    vm.add_task("Schedule me".into());
    let id = vm.state().tasks[0].id;
    let requests = Rc::new(RefCell::new(Vec::new()));
    let record = requests.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| record.borrow_mut().push(request)),
    });
    vm.show_panel(Panel::Reminder(id));
    let before = vm.export_json().unwrap();
    assert!(vm.save_task_date(id, today(), Some("24:01")).is_err());
    assert_eq!(vm.export_json().unwrap(), before);
    assert_eq!(vm.panel(), Panel::Reminder(id));
    assert!(requests.borrow().is_empty());
    vm.save_task_date(id, today(), Some("09:30")).unwrap();
    assert_eq!(
        vm.state()
            .task(id)
            .unwrap()
            .reminder
            .as_ref()
            .unwrap()
            .minutes_after_midnight,
        570
    );
    assert_eq!(vm.panel(), Panel::None);
    assert!(matches!(
        requests.borrow().as_slice(),
        [HostRequest::RequestNotifications]
    ));
    assert!(vm.save_task_date(u64::MAX, today(), None).is_err());
}

#[test]
fn custom_repeat_validates_before_mutation_and_is_one_undo_step() {
    let vm = vm();
    vm.add_task("Repeat me".into());
    let id = vm.state().tasks[0].id;
    vm.show_panel(Panel::Repeat(id));
    let before = vm.export_json().unwrap();
    for invalid in ["0", "-1", "", "1.5", "65536"] {
        assert!(
            vm.save_custom_repeat(id, invalid, RepeatUnit::Month)
                .is_err()
        );
        assert_eq!(vm.export_json().unwrap(), before);
        assert_eq!(vm.panel(), Panel::Repeat(id));
    }
    vm.save_custom_repeat(id, " 3 ", RepeatUnit::Month).unwrap();
    let recurrence = vm.state().task(id).unwrap().recurrence.clone().unwrap();
    assert_eq!(recurrence.every, 3);
    assert_eq!(recurrence.unit, RepeatUnit::Month);
    assert_eq!(recurrence.anchor, Some(today()));
    assert_eq!(vm.panel(), Panel::None);
    vm.undo();
    assert!(vm.state().task(id).unwrap().recurrence.is_none());
}

#[test]
fn scheduling_from_a_cache_callback_completes_after_its_queued_actions() {
    let vm = vm();
    let weak = Rc::downgrade(&vm);
    let once = Cell::new(false);
    let _subscription = vm.tasks.connect().subscribe_next(move |_| {
        if once.replace(true) {
            return;
        }
        let vm = weak.upgrade().unwrap();
        let id = vm.state().tasks[0].id;
        vm.show_panel(Panel::Reminder(id));
        vm.save_task_date(id, today(), Some("18:45")).unwrap();
        vm.save_custom_repeat(id, "2", RepeatUnit::Week).unwrap();
    });
    vm.add_task("Reentrant schedule".into());
    assert_eq!(vm.panel(), Panel::None);
    let state = vm.state();
    assert_eq!(
        state.tasks[0]
            .reminder
            .as_ref()
            .unwrap()
            .minutes_after_midnight,
        1125
    );
    assert_eq!(state.tasks[0].recurrence.as_ref().unwrap().every, 2);
    state.validate().unwrap();
}

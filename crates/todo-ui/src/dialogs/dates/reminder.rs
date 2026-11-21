use todo_domain::{Action, Date, Id, Reminder};
use todo_state::{AppViewModel, HostRequest, Panel};

pub(super) fn apply_date(vm: &AppViewModel, id: Id, date: Date, reminder: bool, time: &str) {
    if reminder {
        let pieces = time.split(':').collect::<Vec<_>>();
        let minutes = if pieces.len() == 2 {
            pieces[0]
                .parse::<u16>()
                .ok()
                .zip(pieces[1].parse::<u16>().ok())
                .filter(|(h, m)| *h < 24 && *m < 60)
                .map(|(h, m)| h * 60 + m)
        } else {
            None
        };
        let Some(minutes_after_midnight) = minutes else {
            vm.report_error("Enter a time such as 09:30".into());
            return;
        };
        vm.dispatch(Action::SetReminder {
            id,
            reminder: Some(Reminder {
                date,
                minutes_after_midnight,
                notified: false,
            }),
        });
        vm.request(HostRequest::RequestNotifications);
    } else {
        vm.dispatch(Action::SetDueDate {
            id,
            date: Some(date),
        });
    }
    vm.show_panel(Panel::None);
}

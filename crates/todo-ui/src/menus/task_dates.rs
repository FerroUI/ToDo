use super::item::{command, submenu as nested};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Action, Id};
use todo_state::{AppViewModel, Panel};

pub(super) fn submenu(vm: Rc<AppViewModel>, ids: Rc<[Id]>) -> Element {
    let mut dates = Vec::new();
    for (label, days) in [("Today", 0), ("Tomorrow", 1), ("Next week", 7)] {
        dates.push(super::task_action::batch(
            vm.clone(),
            ids.clone(),
            label,
            Icon::Calendar,
            move |id, vm| Action::SetDueDate {
                id,
                date: vm.today().checked_add_signed(chrono::Duration::days(days)),
            },
        ));
    }
    if ids.len() == 1 {
        let custom = vm.clone();
        let id = ids[0];
        dates.push(command("Pick a date", Icon::Calendar, move || {
            custom.show_panel(Panel::DueDate(id))
        }));
    }
    dates.push(separator().into());
    dates.push(super::task_action::batch(
        vm,
        ids,
        "Remove due date",
        Icon::Close,
        |id, _| Action::SetDueDate { id, date: None },
    ));
    nested("Set due date", Icon::Calendar, dates)
}

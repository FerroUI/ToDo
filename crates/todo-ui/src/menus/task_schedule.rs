use super::item::{command, submenu};
use ferroui_view::Element;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Action, Id};
use todo_state::{reminder_in_one_hour, AppViewModel, Panel};

pub(super) fn items(vm: Rc<AppViewModel>, ids: Rc<[Id]>) -> Vec<Element> {
    let custom = vm.clone();
    let targets = ids.to_vec();
    let reminder = submenu(
        "Set reminder",
        Icon::Bell,
        vec![
            super::task_action::batch(
                vm.clone(),
                ids.clone(),
                "In one hour",
                Icon::Bell,
                |id, vm| Action::SetReminder {
                    id,
                    reminder: reminder_in_one_hour(vm.today(), vm.current_minutes()),
                },
            ),
            command("Pick a date and time", Icon::Calendar, move || {
                custom.show_panel(if targets.len() == 1 {
                    Panel::Reminder(targets[0])
                } else {
                    Panel::BulkReminder(targets.clone())
                })
            }),
            super::task_action::batch(
                vm.clone(),
                ids.clone(),
                "Remove reminder",
                Icon::Close,
                |id, _| Action::SetReminder { id, reminder: None },
            ),
        ],
    );
    let repeat = vm.clone();
    let targets = ids.to_vec();
    let repetition = submenu(
        "Set repeat",
        Icon::Repeat,
        vec![
            command("Choose repeat pattern", Icon::Repeat, move || {
                repeat.show_panel(if targets.len() == 1 {
                    Panel::Repeat(targets[0])
                } else {
                    Panel::BulkRepeat(targets.clone())
                })
            }),
            super::task_action::batch(
                vm.clone(),
                ids.clone(),
                "Remove repeat",
                Icon::Close,
                |id, _| Action::SetRecurrence {
                    id,
                    recurrence: None,
                },
            ),
        ],
    );
    vec![
        reminder,
        repetition,
        command("Categories", Icon::Flag, move || {
            vm.show_panel(Panel::BulkCategories(ids.to_vec()))
        }),
    ]
}

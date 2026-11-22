use crate::menus::{command, command_detail};
use chrono::Duration;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::Button;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Action, Id, Recurrence, Reminder};
use todo_state::{AppViewModel, Panel, reminder_in_one_hour};

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Due,
    Reminder,
    Repeat,
}

pub(super) fn attach(button: &Ref<Button>, vm: Rc<AppViewModel>, id: Id, kind: Kind) {
    let anchor = button.downgrade();
    let model = vm.clone();
    crate::menus::attach_actions(button, vm, move || {
        if model.state().task(id).is_none() {
            return Vec::new();
        }
        match kind {
            Kind::Repeat => repeats(model.clone(), id, anchor.clone()),
            _ => dates(
                model.clone(),
                id,
                matches!(kind, Kind::Reminder),
                anchor.clone(),
            ),
        }
    });
}

fn dates(vm: Rc<AppViewModel>, id: Id, reminder: bool, anchor: WeakRef<Button>) -> Vec<Element> {
    let mut items = Vec::new();
    for (label, days) in [
        (if reminder { "Later today" } else { "Today" }, 0),
        ("Tomorrow", 1),
        ("Next week", 7),
    ] {
        let Some(date) = vm.today().checked_add_signed(Duration::days(days)) else {
            continue;
        };
        let Some(value) = (if reminder && days == 0 {
            reminder_in_one_hour(vm.today(), vm.current_minutes())
        } else {
            Some(Reminder {
                date,
                minutes_after_midnight: 540,
                notified: false,
            })
        }) else {
            continue;
        };
        let label = if reminder && days == 0 && value.date != vm.today() {
            "In one hour"
        } else {
            label
        };
        let time = format!(
            "{:02}:{:02}",
            value.minutes_after_midnight / 60,
            value.minutes_after_midnight % 60
        );
        let description = if reminder {
            time.clone()
        } else {
            date.format("%a").to_string()
        };
        let choose = vm.clone();
        items.push(command_detail(
            label,
            &description,
            if reminder { Icon::Bell } else { Icon::Calendar },
            move || {
                if let Err(error) =
                    choose.save_task_date(id, value.date, reminder.then_some(time.as_str()))
                {
                    choose.report_error(error);
                }
            },
        ));
    }
    items.push(separator().into());
    let custom = vm.clone();
    items.push(command(
        if reminder {
            "Pick a date & time"
        } else {
            "Pick a date"
        },
        Icon::Calendar,
        move || {
            open_custom(
                &anchor,
                &custom,
                if reminder {
                    Panel::Reminder(id)
                } else {
                    Panel::DueDate(id)
                },
            );
        },
    ));
    let has_value = vm.state().task(id).is_some_and(|task| {
        if reminder {
            task.reminder.is_some()
        } else {
            task.due_date.is_some()
        }
    });
    if has_value {
        items.push(command(
            if reminder {
                "Remove reminder"
            } else {
                "Remove due date"
            },
            Icon::Close,
            move || {
                vm.dispatch(if reminder {
                    Action::SetReminder { id, reminder: None }
                } else {
                    Action::SetDueDate { id, date: None }
                });
            },
        ));
    }
    items
}

fn repeats(vm: Rc<AppViewModel>, id: Id, anchor: WeakRef<Button>) -> Vec<Element> {
    let existing = vm.state().task(id).and_then(|task| task.recurrence.clone());
    let mut items = Vec::new();
    for rule in [
        Recurrence::daily(),
        Recurrence::weekdays(),
        Recurrence::weekly(),
        Recurrence::monthly(),
        Recurrence::yearly(),
    ] {
        let choose = vm.clone();
        let selected = existing
            .as_ref()
            .is_some_and(|current| current.unit == rule.unit && current.every == rule.every);
        items.push(command(
            rule.label(),
            if selected { Icon::Check } else { Icon::Repeat },
            move || {
                choose.dispatch(Action::SetRecurrence {
                    id,
                    recurrence: Some(rule.clone()),
                });
            },
        ));
    }
    items.push(separator().into());
    let custom = vm.clone();
    items.push(command("Custom", Icon::Calendar, move || {
        open_custom(&anchor, &custom, Panel::Repeat(id))
    }));
    if existing.is_some() {
        items.push(command("Never repeat", Icon::Close, move || {
            vm.dispatch(Action::SetRecurrence {
                id,
                recurrence: None,
            })
        }));
    }
    items
}

fn open_custom(anchor: &WeakRef<Button>, vm: &AppViewModel, panel: Panel) {
    // Give the modal a stable return target after the native popup closes.
    if let Some(anchor) = anchor.upgrade() {
        anchor.focus();
    }
    vm.show_panel(panel);
}

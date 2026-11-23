use super::{
    action_row::{action, clear_button, row},
    fields::section,
    schedule_menu::{self, Kind},
};
use crate::palette;
use chrono::Datelike;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Action, Task};
use todo_state::{AppViewModel, Panel};

pub(super) fn view(vm: Rc<AppViewModel>, task: &Task) -> Element {
    let p = palette(&vm);
    let id = task.id;
    let reminder = task.reminder.as_ref().map(|value| {
        format!(
            "{} at {:02}:{:02}",
            vm.date_label(value.date),
            value.minutes_after_midnight / 60,
            value.minutes_after_midnight % 60
        )
    });
    let due = task.due_date.map(|date| {
        if date == vm.today() {
            "Due today".into()
        } else if Some(date) == vm.today().succ_opt() {
            "Due tomorrow".into()
        } else if date.year() != vm.today().year() {
            format!("Due {}", date.format("%-d %b %Y"))
        } else {
            format!("Due {}", vm.date_label(date))
        }
    });
    let repeat = task.recurrence.as_ref().map(|rule| rule.label());
    let mut content = stack_panel();
    if let Some(start) = task.start_date.filter(|_| !task.is_planner()) {
        let open = vm.clone();
        let clear = vm.clone();
        content = content.child(row(
            action(
                Icon::Calendar,
                &format!("Starts {}", vm.date_label(start)),
                p.accent,
            )
            .name("TaskStartDate")
            .on_click(move |_| open.show_panel(Panel::DueDate(id))),
            Some(clear_button(
                "Remove start date",
                p.secondary,
                "TaskStartDate",
                move || clear.dispatch(Action::SetStartDate { id, date: None }),
            )),
        ));
    }
    for (kind, symbol, label, active, clear_label, clear_action, name) in [
        (
            Kind::Reminder,
            Icon::Bell,
            reminder.unwrap_or_else(|| "Remind me".into()),
            task.reminder.is_some(),
            "Remove reminder",
            Action::SetReminder { id, reminder: None },
            "TaskReminder",
        ),
        (
            Kind::Due,
            Icon::Calendar,
            due.unwrap_or_else(|| "Add due date".into()),
            task.due_date.is_some(),
            "Remove due date",
            Action::SetDueDate { id, date: None },
            "TaskDueDate",
        ),
        (
            Kind::Repeat,
            Icon::Repeat,
            repeat.unwrap_or_else(|| "Repeat".into()),
            task.recurrence.is_some(),
            "Remove repeat",
            Action::SetRecurrence {
                id,
                recurrence: None,
            },
            "TaskRepeat",
        ),
    ] {
        if task.is_planner() && !matches!(kind, Kind::Due) {
            continue;
        }
        let menu = vm.clone();
        let clear = vm.clone();
        let overdue = matches!(kind, Kind::Due)
            && !task.completed
            && task.due_date.is_some_and(|date| date < vm.today());
        let color = if overdue {
            if vm.is_dark() {
                "#F1707B"
            } else {
                "#D13438"
            }
        } else if active {
            p.accent
        } else {
            p.secondary
        };
        content = content.child(row(
            action(symbol, &label, color)
                .name(name)
                .with(move |button| schedule_menu::attach(button, menu.clone(), id, kind)),
            active.then(|| {
                clear_button(clear_label, p.secondary, name, move || {
                    clear.dispatch(clear_action.clone())
                })
            }),
        ));
        if !task.is_planner() && !matches!(kind, Kind::Repeat) {
            content = content.child(
                border()
                    .height(1.0)
                    .background(p.border)
                    .margin((16.0, 0.0)),
            );
        }
    }
    section(vm, content)
}

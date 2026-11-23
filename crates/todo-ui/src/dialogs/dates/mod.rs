mod calendar;
mod calendar_accessibility;
mod date_field;
mod recurrence;
mod validation;

use crate::palette;
use ferroui_base::input::Key;
use ferroui_base::layout::VerticalAlignment;
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
pub use recurrence::{repeat, repeat_bulk};
use std::rc::Rc;
use todo_controls::primary_button;
use todo_domain::Id;
use todo_state::{reminder_in_one_hour, AppViewModel};

pub fn date_picker(vm: Rc<AppViewModel>, id: Id, reminder: bool) -> Element {
    date_picker_bulk(vm, vec![id], reminder)
}

pub fn date_picker_bulk(vm: Rc<AppViewModel>, ids: Vec<Id>, reminder: bool) -> Element {
    let Some(id) = ids.first().copied() else {
        return Element::Empty;
    };
    rerender(move || {
        let p = palette(&vm);
        let later = reminder_in_one_hour(vm.today(), vm.current_minutes());
        let selected = use_signal(|| {
            vm.state()
                .task(id)
                .and_then(|task| {
                    if reminder {
                        task.reminder.as_ref().map(|value| value.date)
                    } else {
                        task.due_date
                    }
                })
                .unwrap_or_else(|| {
                    if reminder {
                        later.as_ref().map_or(vm.today(), |value| value.date)
                    } else {
                        vm.today()
                    }
                })
        });
        let time = use_signal(|| {
            let minutes = vm
                .state()
                .task(id)
                .and_then(|task| task.reminder.as_ref())
                .map(|value| value.minutes_after_midnight)
                .or_else(|| later.as_ref().map(|value| value.minutes_after_midnight))
                .unwrap_or(540);
            format!("{:02}:{:02}", minutes / 60, minutes % 60)
        });
        let error = use_signal(String::new);
        let start = use_signal(|| {
            vm.state()
                .task(id)
                .and_then(|task| task.start_date)
                .map(|date| date.to_string())
                .unwrap_or_default()
        });
        let edit_range = !reminder
            && ids.len() == 1
            && !vm.state().task(id).is_some_and(|task| task.is_planner());
        let field = use_signal(NodeRef::<TextBox>::new).get_untracked();
        let save = vm.clone();
        let targets = ids.clone();
        let submit: Rc<dyn Fn()> = Rc::new(move || {
            let text = time.get();
            let result = if edit_range {
                save.save_task_range(id, selected.get(), &start.get())
            } else {
                save.save_tasks_date(&targets, selected.get(), reminder.then_some(text.as_str()))
            };
            if let Err(message) = result {
                error.set(message);
                if let Some(field) = field.get_untracked() {
                    field.focus();
                    field.select_all();
                }
            }
        });
        let mut content = stack_panel()
            .spacing(12.0)
            .child(calendar::view(&vm, selected));
        if edit_range {
            content = content.child(recurrence::fields::labelled(
                &vm,
                "Start date (optional)",
                recurrence::fields::date(&vm, "TaskRangeStart", "Task start date", start, error),
            ));
        }
        if reminder {
            let enter = submit.clone();
            content = content.child(
                grid()
                    .column_definitions("*,116")
                    .column_spacing(12.0)
                    .child(
                        text_block()
                            .text("Time")
                            .font_size(14.0)
                            .foreground(p.text)
                            .vertical_alignment(VerticalAlignment::Center),
                    )
                    .child(
                        text_box()
                            .name("ReminderTime")
                            .class("todo-no-clear")
                            .node_ref(field)
                            .grid_column(1)
                            .automation_properties_name("Reminder time")
                            .automation_properties_help_text("24-hour time, for example 09:30")
                            .text(time.two_way())
                            .placeholder_text("HH:MM")
                            .max_length(5)
                            .on_changed(TextBox::text_property(), move |_| error.set(String::new()))
                            .on_key_down(move |event| {
                                if event.key == Key::Return && !crate::editing::is_composing(event)
                                {
                                    enter();
                                    event.set_handled(true);
                                }
                            }),
                    ),
            );
        }
        content
            .child(validation::message(error.get(), vm.is_dark()))
            .child(primary_button("Save", move || submit()))
            .into_element()
    })
}

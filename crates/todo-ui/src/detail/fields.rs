use crate::palette;
use ferroui_base::input::Key;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, Icon};
use todo_domain::Task;
use todo_state::{AppViewModel, Panel};

pub(super) fn scheduling(vm: Rc<AppViewModel>, task: &Task) -> Element {
    let id = task.id;
    let reminder_vm = vm.clone();
    let due_vm = vm.clone();
    let repeat_vm = vm.clone();
    let reminder = task.reminder.as_ref().map(|r| {
        format!(
            "{} at {:02}:{:02}",
            vm.date_label(r.date),
            r.minutes_after_midnight / 60,
            r.minutes_after_midnight % 60
        )
    });
    let due = task
        .due_date
        .map(|date| format!("Due {}", vm.date_label(date).to_lowercase()));
    let repeat = task.recurrence.as_ref().map(|r| r.label());
    section(
        vm.clone(),
        stack_panel()
            .child(detail_action(
                vm.clone(),
                Icon::Bell,
                reminder.as_deref().unwrap_or("Remind me"),
                move || reminder_vm.show_panel(Panel::Reminder(id)),
            ))
            .child(separator(vm.clone()))
            .child(detail_action(
                vm.clone(),
                Icon::Calendar,
                due.as_deref().unwrap_or("Add due date"),
                move || due_vm.show_panel(Panel::DueDate(id)),
            ))
            .child(separator(vm.clone()))
            .child(detail_action(
                vm.clone(),
                Icon::Repeat,
                repeat.as_deref().unwrap_or("Repeat"),
                move || repeat_vm.show_panel(Panel::Repeat(id)),
            )),
    )
}

pub(crate) fn edit_field(
    name: &str,
    value: String,
    placeholder: &str,
    multiline: bool,
    save: impl Fn(String) + 'static,
) -> Element {
    let enter_commits = !multiline || name == "TaskTitle";
    let commit: Rc<dyn Fn(String)> = Rc::new(save);
    let enter = commit.clone();
    text_box()
        .name(name)
        .class("todo-input")
        .text(value)
        .placeholder_text(placeholder)
        .font_size(if name == "TaskTitle" { 18.0 } else { 13.0 })
        .font_weight(if name == "TaskTitle" {
            FontWeight::SemiBold
        } else {
            FontWeight::Normal
        })
        .background("Transparent")
        .border_thickness(0.0)
        .padding((6.0, 8.0))
        .accepts_return(multiline && name != "TaskTitle")
        .text_wrapping(TextWrapping::Wrap)
        .on_lost_focus(move |event| {
            if let Some(field) = event.source().and_then(|source| source.cast::<TextBox>()) {
                commit(field.text().unwrap_or_default());
            }
        })
        .on_key_down(move |event| {
            if event.key == Key::Return && enter_commits {
                if let Some(field) = event.source().and_then(|source| source.cast::<TextBox>()) {
                    enter(field.text().unwrap_or_default());
                }
                event.set_handled(true);
            }
        })
        .into()
}

fn detail_action(
    vm: Rc<AppViewModel>,
    symbol: Icon,
    label: &str,
    click: impl Fn() + 'static,
) -> Element {
    let p = palette(&vm);
    border()
        .min_height(48.0)
        .background(p.surface)
        .child(action_button(symbol, label, click))
        .into()
}
pub(super) fn section(vm: Rc<AppViewModel>, child: impl IntoChild) -> Element {
    border()
        .background(palette(&vm).surface)
        .corner_radius(4.0)
        .child(child)
        .into()
}
fn separator(vm: Rc<AppViewModel>) -> Element {
    border()
        .height(1.0)
        .background(palette(&vm).border)
        .margin((16.0, 0.0))
        .into()
}

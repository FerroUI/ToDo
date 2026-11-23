use super::{edit_field, note_links};
use crate::palette;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::{Button, TextBox};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_domain::{notes::RichNote, Action, Id, Step};
use todo_state::AppViewModel;

pub(super) fn view(vm: Rc<AppViewModel>, task_id: Id, step: &Step) -> Element {
    let id = step.id;
    let step = step.clone();
    border()
        .key(format!("step-text-{id}"))
        .child(rerender(move || -> Element {
            let p = palette(&vm);
            let editing = use_signal(|| false);
            let input = use_hook(NodeRef::<TextBox>::new);
            let edit = use_hook(NodeRef::<Button>::new);
            let linked = note_links::has_links(&step.title);
            let save = vm.clone();
            use_effect_with(editing.get(), move |editing| {
                if *editing {
                    if let Some(input) = input.get_untracked() {
                        input.focus();
                        input.select_all();
                    }
                }
            });
            grid()
                .column_definitions("*,Auto")
                .child(
                    border()
                        .is_visible(linked && !editing.get())
                        .child(note_links::view(
                            vm.clone(),
                            RichNote::plain(&step.title),
                            &format!("StepRead{id}"),
                            if step.completed { p.secondary } else { p.text },
                            p.accent,
                        )),
                )
                .child(
                    edit_field(
                        &format!("Step{id}"),
                        step.title.clone(),
                        "Step",
                        false,
                        move |title| {
                            let entered = input
                                .get_untracked()
                                .is_some_and(|field| field.is_focused());
                            save.dispatch(Action::RenameStep {
                                task_id,
                                step_id: id,
                                title,
                            });
                            editing.set(false);
                            if entered {
                                Dispatcher::ui_thread().post_local(
                                    move || {
                                        if let Some(button) = edit.get_untracked() {
                                            if button.is_attached_to_visual_tree()
                                                && button.is_effectively_visible()
                                            {
                                                button.focus();
                                            } else if let Some(input) =
                                                input.get_untracked().filter(|input| {
                                                    input.is_attached_to_visual_tree()
                                                        && input.is_effectively_visible()
                                                })
                                            {
                                                input.focus();
                                            }
                                        }
                                    },
                                    DispatcherPriority::BACKGROUND,
                                );
                            }
                        },
                    )
                    .node_ref(input)
                    .is_visible(!linked || editing.get())
                    .foreground(if step.completed { p.secondary } else { p.text })
                    .automation_properties_name(format!("Step: {}", step.title)),
                )
                .child(
                    button()
                        .grid_column(1)
                        .node_ref(edit)
                        .name(format!("EditStep{id}"))
                        .is_visible(linked && !editing.get())
                        .class("todo-toolbar")
                        .padding((6.0, 4.0))
                        .content("Edit")
                        .automation_properties_name(format!("Edit step: {}", step.title))
                        .on_click(move |event| {
                            editing.set(true);
                            event.set_handled(true);
                        }),
                )
                .into()
        }))
        .into()
}

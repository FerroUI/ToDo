use super::edit_field;
use crate::palette;
use ferroui_base::input::Key;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{task_check, Icon};
use todo_domain::{Action, Task};
use todo_state::{AppViewModel, Panel};

pub(super) fn steps(vm: Rc<AppViewModel>, task: &Task) -> Element {
    let p = palette(&vm);
    let id = task.id;
    let mut items = stack_panel().spacing(1.0);
    for step in &task.steps {
        let step_id = step.id;
        let toggle = vm.clone();
        let options = vm.clone();
        let context = vm.clone();
        let rename = vm.clone();
        items = items.child(
            grid()
                .key(step_id.to_string())
                .column_definitions("40,*,32")
                .min_height(36.0)
                .child(border().vertical_alignment(V::Center).child(task_check(
                    step.completed,
                    p.accent,
                    move || {
                        toggle.dispatch(Action::ToggleStep {
                            task_id: id,
                            step_id,
                        })
                    },
                )))
                .child(border().grid_column(1).child(edit_field(
                    &format!("Step{step_id}"),
                    step.title.clone(),
                    "Step",
                    false,
                    move |title| {
                        if !title.trim().is_empty() {
                            rename.dispatch(Action::RenameStep {
                                task_id: id,
                                step_id,
                                title,
                            });
                        }
                    },
                )))
                .child(
                    button()
                        .grid_column(2)
                        .vertical_alignment(V::Center)
                        .name(format!("StepOptions{step_id}"))
                        .class("todo-toolbar")
                        .width(32.0)
                        .height(32.0)
                        .padding(6.0)
                        .automation_properties_name(format!("Options for {}", step.title))
                        .tool_tip_tip("Step options")
                        .on_click(move |event| {
                            event.set_handled(true);
                            options.show_panel(Panel::StepMenu {
                                task_id: id,
                                step_id,
                            });
                        })
                        .on_context_requested(move |event| {
                            event.set_handled(true);
                            context.show_panel(Panel::StepMenu {
                                task_id: id,
                                step_id,
                            });
                        })
                        .child(todo_controls::icon(Icon::More, 18.0, p.secondary)),
                ),
        );
    }
    let add = vm.clone();
    items
        .child(
            grid()
                .column_definitions("40,*")
                .margin((0.0, 2.0))
                .child(
                    border()
                        .horizontal_alignment(H::Center)
                        .vertical_alignment(V::Center)
                        .child(todo_controls::icon(Icon::Add, 16.0, p.accent)),
                )
                .child(
                    text_box()
                        .grid_column(1)
                        .name("NewStep")
                        .class("todo-input")
                        .placeholder_text("Add step")
                        .font_size(13.0)
                        .foreground(p.accent)
                        .background("Transparent")
                        .border_thickness(0.0)
                        .on_key_down(move |event| {
                            if event.key == Key::Return {
                                if let Some(field) =
                                    event.source().and_then(|source| source.cast::<TextBox>())
                                {
                                    let title = field.text().unwrap_or_default();
                                    if !title.trim().is_empty() {
                                        let before =
                                            add.state().task(id).map_or(0, |task| task.steps.len());
                                        add.dispatch(Action::AddStep { task_id: id, title });
                                        if add
                                            .state()
                                            .task(id)
                                            .is_some_and(|task| task.steps.len() > before)
                                        {
                                            field.set_text(Some(""));
                                        }
                                    }
                                }
                                event.set_handled(true);
                            }
                        }),
                ),
        )
        .into()
}

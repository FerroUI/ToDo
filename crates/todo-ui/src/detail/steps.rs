use super::edit_field;
use crate::palette;
use ferroui_base::layout::VerticalAlignment;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{Icon, inherited_icon};
use todo_domain::{Action, Task};
use todo_state::AppViewModel;

pub(super) fn steps(vm: Rc<AppViewModel>, task: &Task) -> Element {
    let p = palette(&vm);
    let id = task.id;
    let mut items = stack_panel()
        .spacing(1.0)
        .automation_properties_name("Steps")
        .automation_properties_help_text(format!(
            "{} of {} steps completed",
            task.completed_steps(),
            task.steps.len()
        ));
    for step in &task.steps {
        let step_id = step.id;
        let toggle = vm.clone();
        let options = vm.clone();
        let rename = vm.clone();
        let label = format!(
            "Mark {} as {}",
            step.title,
            if step.completed {
                "not completed"
            } else {
                "completed"
            }
        );
        items = items.child(
            grid()
                .key(step_id.to_string())
                .column_definitions("40,*,32")
                .min_height(40.0)
                .child(
                    check_box()
                        .class("todo-completion")
                        .is_checked(Some(step.completed))
                        .width(40.0)
                        .height(40.0)
                        .padding(9.0)
                        .foreground(p.accent)
                        .automation_properties_name(label.clone())
                        .tool_tip_tip(label)
                        .on_click(move |event| {
                            toggle.dispatch(Action::ToggleStep {
                                task_id: id,
                                step_id,
                            });
                            event.set_handled(true);
                        })
                        .child(inherited_icon(
                            if step.completed {
                                Icon::CheckCircle
                            } else {
                                Icon::Circle
                            },
                            20.0,
                        )),
                )
                .child(
                    border().grid_column(1).child(
                        edit_field(
                            &format!("Step{step_id}"),
                            step.title.clone(),
                            "Step",
                            false,
                            move |title| {
                                rename.dispatch(Action::RenameStep {
                                    task_id: id,
                                    step_id,
                                    title,
                                })
                            },
                        )
                        .foreground(if step.completed { p.secondary } else { p.text })
                        .automation_properties_name(format!("Step: {}", step.title)),
                    ),
                )
                .child(
                    todo_controls::toolbar_button(
                        Icon::More,
                        &format!("Options for {}", step.title),
                        p.secondary,
                    )
                    .grid_column(2)
                    .vertical_alignment(VerticalAlignment::Center)
                    .name(format!("StepOptions{step_id}"))
                    .tool_tip_tip("Step options")
                    .with(move |button| {
                        super::step_menu::attach(button, options.clone(), id, step_id)
                    }),
                ),
        );
    }
    items.child(super::new_step::view(vm, id)).into()
}

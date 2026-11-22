use crate::palette;
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, Icon};
use todo_domain::{Action, Id};
use todo_state::{AppViewModel, Panel};

pub(super) fn view(vm: Rc<AppViewModel>, task_id: Id, step_id: Id) -> Element {
    let Some(step) = vm
        .state()
        .task(task_id)
        .and_then(|task| task.steps.iter().find(|step| step.id == step_id))
        .cloned()
    else {
        return Element::Empty;
    };
    let promote = vm.clone();
    let remove = vm.clone();
    stack_panel()
        .spacing(2.0)
        .child(
            text_block()
                .text(step.title)
                .font_size(13.0)
                .foreground(palette(&vm).secondary)
                .text_wrapping(TextWrapping::Wrap)
                .margin((8.0, 4.0, 8.0, 10.0)),
        )
        .child(action_button(Icon::ArrowUp, "Promote to task", move || {
            promote.dispatch(Action::PromoteStep { task_id, step_id });
            promote.show_panel(Panel::None);
        }))
        .child(action_button(Icon::Trash, "Delete step", move || {
            remove.dispatch(Action::DeleteStep { task_id, step_id });
            remove.show_panel(Panel::None);
        }))
        .into()
}

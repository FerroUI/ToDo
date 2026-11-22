use ferroui_base::Ref;
use ferroui_controls::Button;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Action, Id};
use todo_state::AppViewModel;

pub(super) fn attach(button: &Ref<Button>, vm: Rc<AppViewModel>, task_id: Id, step_id: Id) {
    let model = vm.clone();
    crate::menus::attach_actions(button, vm, move || {
        let Some(step) = model
            .state()
            .task(task_id)
            .and_then(|task| task.steps.iter().find(|step| step.id == step_id))
            .cloned()
        else {
            return Vec::new();
        };
        let complete = model.clone();
        let promote = model.clone();
        let remove = model.clone();
        vec![
            crate::menus::command(
                if step.completed {
                    "Mark as not completed"
                } else {
                    "Mark as completed"
                },
                Icon::Check,
                move || complete.dispatch(Action::ToggleStep { task_id, step_id }),
            ),
            crate::menus::command("Promote to task", Icon::ArrowUp, move || {
                promote.dispatch(Action::PromoteStep { task_id, step_id })
            }),
            separator().into(),
            crate::menus::destructive("Delete step", Icon::Trash, model.is_dark(), move || {
                remove.dispatch(Action::DeleteStep { task_id, step_id })
            }),
        ]
    });
}

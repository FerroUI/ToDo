use ferroui_controls::{Control, TextBox};
use todo_domain::Action;
use todo_state::AppViewModel;

/// Commits visible document/settings editors before a host hides, closes or exports them.
/// Collect first: dispatch can synchronously reconcile and replace these controls.
pub fn commit_pending_edits(vm: &AppViewModel, root: &Control) {
    let document = vm.state();
    let task = vm.selected().and_then(|id| document.task(id));
    let mut changes = Vec::new();
    let mut rename = None;
    for visual in root.get_visual_descendants() {
        let Some(field) = visual.cast::<TextBox>() else {
            continue;
        };
        if !field.is_effectively_visible() {
            continue;
        }
        let value = field.text().unwrap_or_default();
        match field.name().as_deref() {
            Some("AccountName") if value != document.settings.account_name => {
                let mut settings = document.settings.clone();
                settings.account_name = value;
                changes.push(Action::SetSettings(settings));
            }
            Some("ListName" | "GroupName") => {
                rename = vm.rename_session().map(|edit| (edit.target, value));
            }
            Some("TaskTitle")
                if task
                    .is_some_and(|task| !value.trim().is_empty() && value.trim() != task.title) =>
            {
                changes.push(Action::RenameTask {
                    id: task.unwrap().id,
                    title: value,
                });
            }
            Some("TaskNotes") if task.is_some_and(|task| value != task.notes) => {
                changes.push(Action::SetNotes {
                    id: task.unwrap().id,
                    notes: value,
                });
            }
            Some(name) if name.starts_with("Step") => {
                if let (Some(task), Ok(step_id)) = (task, name[4..].parse::<u64>()) {
                    if task.steps.iter().any(|step| {
                        step.id == step_id && !value.trim().is_empty() && value.trim() != step.title
                    }) {
                        changes.push(Action::RenameStep {
                            task_id: task.id,
                            step_id,
                            title: value,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    drop(document);
    if !changes.is_empty() {
        vm.dispatch(Action::Batch(changes));
    }
    if let Some((target, value)) = rename {
        vm.set_rename_draft(target, value);
        vm.commit_rename(target);
    }
}

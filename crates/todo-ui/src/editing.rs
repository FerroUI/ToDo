use ferroui_controls::{Control, TextBox};
use todo_domain::Action;
use todo_state::AppViewModel;

/// Commits the visible detail editor before a host hides, closes or exports it.
/// Collect first: dispatch can synchronously reconcile and replace these controls.
pub fn commit_pending_edits(vm: &AppViewModel, root: &Control) {
    let Some(id) = vm.selected() else { return };
    let document = vm.state();
    let Some(task) = document.task(id) else {
        return;
    };
    let mut changes = Vec::new();
    for visual in root.get_visual_descendants() {
        let Some(field) = visual.cast::<TextBox>() else {
            continue;
        };
        if !field.is_effectively_visible() {
            continue;
        }
        let value = field.text().unwrap_or_default();
        match field.name().as_deref() {
            Some("TaskTitle") if !value.trim().is_empty() && value.trim() != task.title => {
                changes.push(Action::RenameTask { id, title: value });
            }
            Some("TaskNotes") if value != task.notes => {
                changes.push(Action::SetNotes { id, notes: value });
            }
            Some(name) if name.starts_with("Step") => {
                if let Ok(step_id) = name[4..].parse::<u64>() {
                    if task.steps.iter().any(|step| {
                        step.id == step_id && !value.trim().is_empty() && value.trim() != step.title
                    }) {
                        changes.push(Action::RenameStep {
                            task_id: id,
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
}

//! Undo stores only affected entities; changing one checkbox never retains thirty databases.
use std::collections::{BTreeSet, HashMap};
use todo_domain::{Action, AppState, Changes, Id, ListGroup, Settings, Task, TaskList};

pub(crate) struct HistoryEntry {
    tasks: HashMap<Id, Option<Task>>,
    topology: Option<(Vec<TaskList>, Vec<ListGroup>)>,
    settings: Option<Settings>,
    next_id: Id,
}

impl HistoryEntry {
    pub fn capture(state: &AppState, action: &Action) -> Self {
        let mut ids = BTreeSet::new();
        let mut topology = false;
        let mut settings = false;
        scope(action, state, &mut ids, &mut topology, &mut settings);
        Self {
            tasks: ids
                .into_iter()
                .map(|id| (id, state.task(id).cloned()))
                .collect(),
            topology: topology.then(|| (state.lists.clone(), state.groups.clone())),
            settings: settings.then(|| state.settings.clone()),
            next_id: state.next_id,
        }
    }

    pub fn whole(state: &AppState) -> Self {
        Self {
            tasks: state
                .tasks
                .iter()
                .map(|task| (task.id, Some(task.clone())))
                .collect(),
            topology: Some((state.lists.clone(), state.groups.clone())),
            settings: Some(state.settings.clone()),
            next_id: state.next_id,
        }
    }

    pub fn finish(&mut self, changes: &Changes) {
        let affected: BTreeSet<_> = changes
            .changed_tasks
            .iter()
            .chain(&changes.deleted_tasks)
            .copied()
            .collect();
        self.tasks.retain(|id, _| affected.contains(id));
        for id in affected {
            self.tasks.entry(id).or_insert(None);
        }
    }

    pub fn include_new_ids(&mut self, state: &AppState) {
        for task in &state.tasks {
            self.tasks.entry(task.id).or_insert(None);
        }
    }

    /// Swap the patch with the current document and return its inverse for redo.
    pub fn restore(mut self, state: &mut AppState) -> (Self, Changes) {
        let mut inverse = Self {
            tasks: HashMap::new(),
            topology: None,
            settings: None,
            next_id: state.next_id,
        };
        let mut changes = Changes {
            changed: true,
            ..Changes::default()
        };
        let old = std::mem::take(&mut state.tasks);
        state.tasks.reserve(old.len());
        for task in old {
            if let Some(previous) = self.tasks.remove(&task.id) {
                let id = task.id;
                inverse.tasks.insert(id, Some(task.clone()));
                if let Some(previous) = previous {
                    if previous != task {
                        changes.changed_tasks.push(id);
                    }
                    state.tasks.push(previous);
                } else {
                    changes.deleted_tasks.push(id);
                }
            } else {
                state.tasks.push(task);
            }
        }
        for (id, previous) in self.tasks {
            inverse.tasks.insert(id, None);
            if let Some(task) = previous {
                state.tasks.push(task);
                changes.changed_tasks.push(id);
            }
        }
        if let Some((lists, groups)) = self.topology {
            inverse.topology = Some((
                std::mem::replace(&mut state.lists, lists),
                std::mem::replace(&mut state.groups, groups),
            ));
            changes.lists_changed = true;
        }
        if let Some(settings) = self.settings {
            inverse.settings = Some(std::mem::replace(&mut state.settings, settings));
            changes.settings_changed = true;
        }
        // Never recycle an identifier after undo: attachments and host events may retain it.
        state.next_id = state.next_id.max(self.next_id);
        (inverse, changes)
    }

    pub fn estimated_bytes(&self) -> usize {
        let tasks = self
            .tasks
            .values()
            .filter_map(Option::as_ref)
            .map(|task| {
                std::mem::size_of::<Task>()
                    + task.title.len()
                    + task.notes.len()
                    + task
                        .steps
                        .iter()
                        .map(|step| step.title.len() + 32)
                        .sum::<usize>()
                    + task
                        .attachments
                        .iter()
                        .map(|file| file.name.len() + file.storage_key.len() + 96)
                        .sum::<usize>()
            })
            .sum::<usize>();
        let topology = self.topology.as_ref().map_or(0, |(lists, groups)| {
            lists
                .iter()
                .map(|list| list.name.len() + 128)
                .sum::<usize>()
                + groups
                    .iter()
                    .map(|group| group.name.len() + 48)
                    .sum::<usize>()
        });
        tasks + topology + self.tasks.len() * 32 + 256
    }
}

fn scope(
    action: &Action,
    state: &AppState,
    ids: &mut BTreeSet<Id>,
    topology: &mut bool,
    settings: &mut bool,
) {
    match action {
        Action::Batch(actions) => {
            for action in actions {
                scope(action, state, ids, topology, settings);
            }
        }
        Action::SetSettings(_) => *settings = true,
        Action::RenameTask { id, .. }
        | Action::SetNotes { id, .. }
        | Action::ToggleCompleted(id)
        | Action::SetCompleted { id, .. }
        | Action::ToggleImportant(id)
        | Action::SetImportant { id, .. }
        | Action::SetMyDay { id, .. }
        | Action::SetDueDate { id, .. }
        | Action::SetReminder { id, .. }
        | Action::SetRecurrence { id, .. }
        | Action::SetTags { id, .. }
        | Action::AssignTask { id, .. }
        | Action::MoveTask { id, .. }
        | Action::DeleteTask(id) => {
            ids.insert(*id);
        }
        Action::AddStep { task_id, .. }
        | Action::RenameStep { task_id, .. }
        | Action::ToggleStep { task_id, .. }
        | Action::DeleteStep { task_id, .. }
        | Action::PromoteStep { task_id, .. }
        | Action::AddAttachment { task_id, .. }
        | Action::DeleteAttachment { task_id, .. } => {
            ids.insert(*task_id);
        }
        // A preceding command in a batch may have moved the task into another list.
        Action::ReorderTask { .. } => ids.extend(state.tasks.iter().map(|task| task.id)),
        Action::DeleteCompleted { list_id } => ids.extend(
            state
                .tasks
                .iter()
                .filter(|task| list_id.is_none_or(|id| task.list_id == id))
                .map(|task| task.id),
        ),
        Action::DeleteList(id) => {
            *topology = true;
            ids.extend(
                state
                    .tasks
                    .iter()
                    .filter(|task| task.list_id == *id)
                    .map(|task| task.id),
            );
        }
        Action::AddTask { .. }
        | Action::AddTaskInView { .. }
        | Action::AddTaskEntry { .. }
        | Action::DuplicateTask(_) => {}
        _ => *topology = true,
    }
}

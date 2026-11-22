mod groups;
mod order;

use crate::{Action, AppState, Changes, Date, DomainError, TASKS_LIST, TaskList, action::nonempty};

impl AppState {
    pub(crate) fn apply_list(
        &mut self,
        action: Action,
        today: Date,
    ) -> Result<Changes, DomainError> {
        match action {
            Action::AddList { name } => {
                let name = nonempty(name)?;
                let position = self.next_list_position(None)?;
                let id = self.allocate_id();
                let mut list = TaskList::new(id, name);
                list.position = position;
                self.lists.push(list);
                self.append_sidebar(id);
                Ok(Changes {
                    created_list: Some(id),
                    ..Changes::lists()
                })
            }
            Action::RenameList { id, name } => {
                if id == TASKS_LIST {
                    return Err(DomainError::ProtectedList);
                }
                let name = nonempty(name)?;
                let index = self.list_index(id)?;
                if self.lists[index].name == name {
                    return Ok(Changes::default());
                }
                self.lists[index].name = name;
                Ok(Changes::lists())
            }
            Action::DeleteList(id) => {
                if id == TASKS_LIST {
                    return Err(DomainError::ProtectedList);
                }
                let index = self.list_index(id)?;
                let deleted_tasks = self
                    .tasks
                    .iter()
                    .filter(|task| task.list_id == id)
                    .map(|task| task.id)
                    .collect();
                self.tasks.retain(|task| task.list_id != id);
                self.lists.remove(index);
                self.sidebar_order.retain(|candidate| *candidate != id);
                Ok(Changes {
                    deleted_tasks,
                    ..Changes::lists()
                })
            }
            Action::DuplicateList(id) => {
                let index = self.list_index(id)?;
                let mut list = self.lists[index].clone();
                list.cloud = None;
                let position = self.next_list_position(list.group_id)?;
                let list_id = self.allocate_id();
                list.id = list_id;
                list.name.push_str(" (copy)");
                list.position = position;
                self.lists.push(list);
                if self
                    .list(list_id)
                    .is_some_and(|list| list.group_id.is_none())
                {
                    self.append_sidebar(list_id);
                }
                let tasks: Vec<_> = self
                    .tasks
                    .iter()
                    .filter(|task| task.list_id == id && !task.is_planner() && !task.flagged_email)
                    .cloned()
                    .collect();
                let mut changes = Changes {
                    created_list: Some(list_id),
                    ..Changes::lists()
                };
                for original in tasks {
                    let mut task = self.duplicate(original.clone(), today);
                    task.list_id = list_id;
                    task.completed = original.completed;
                    task.completed_on = original.completed_on;
                    task.recurrence_generated = original.recurrence_generated;
                    for (step, original_step) in task.steps.iter_mut().zip(&original.steps) {
                        step.completed = original_step.completed;
                    }
                    changes.changed_tasks.push(task.id);
                    self.tasks.push(task);
                }
                Ok(changes)
            }
            Action::SetListTheme {
                id,
                color,
                background,
            } => {
                if !valid_color(&color) {
                    return Err(DomainError::InvalidValue(
                        "Choose a six-digit hexadecimal theme color".into(),
                    ));
                }
                let index = self.list_index(id)?;
                self.lists[index].color = color;
                self.lists[index].background = background;
                Ok(Changes::lists())
            }
            Action::SetListSort { id, sort } => {
                let index = self.list_index(id)?;
                self.lists[index].sort = sort;
                self.lists[index].sort_reversed = false;
                Ok(Changes::lists())
            }
            Action::SetListSortDirection { id, reversed } => {
                let index = self.list_index(id)?;
                let reversed = reversed && self.lists[index].sort != crate::SortOrder::Manual;
                if self.lists[index].sort_reversed == reversed {
                    return Ok(Changes::default());
                }
                self.lists[index].sort_reversed = reversed;
                Ok(Changes::lists())
            }
            Action::SetListShowCompleted { id, show } => {
                let index = self.list_index(id)?;
                self.lists[index].show_completed = show;
                Ok(Changes::lists())
            }
            Action::MoveList { id, group_id } => self.move_list(id, group_id),
            Action::ReorderList { id, before } => self.reorder_list(id, before),
            Action::ReorderGroup { id, before } => self.reorder_group(id, before),
            Action::ReorderSidebar { id, before } => self.reorder_sidebar(id, before),
            Action::AddGroup { .. }
            | Action::RenameGroup { .. }
            | Action::DeleteGroup(_)
            | Action::ToggleGroup(_) => self.apply_group(action),
            _ => Err(DomainError::InvalidValue("Unsupported list command".into())),
        }
    }
}

pub(crate) fn valid_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

use crate::{
    Action, AppState, Changes, Date, DomainError, Id, ListGroup, TASKS_LIST, TaskList,
    action::nonempty,
};

impl AppState {
    pub(crate) fn apply_list(
        &mut self,
        action: Action,
        today: Date,
    ) -> Result<Changes, DomainError> {
        match action {
            Action::AddList { name } => {
                let name = nonempty(name)?;
                let id = self.allocate_id();
                self.lists.push(TaskList::new(id, name));
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
                Ok(Changes {
                    deleted_tasks,
                    ..Changes::lists()
                })
            }
            Action::DuplicateList(id) => {
                let index = self.list_index(id)?;
                let mut list = self.lists[index].clone();
                let list_id = self.allocate_id();
                list.id = list_id;
                list.name.push_str(" (copy)");
                list.position = self
                    .lists
                    .iter()
                    .map(|list| list.position)
                    .max()
                    .unwrap_or(0)
                    + 1;
                self.lists.push(list);
                let tasks: Vec<_> = self
                    .tasks
                    .iter()
                    .filter(|task| task.list_id == id)
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
                Ok(Changes::lists())
            }
            Action::SetListShowCompleted { id, show } => {
                let index = self.list_index(id)?;
                self.lists[index].show_completed = show;
                Ok(Changes::lists())
            }
            Action::MoveList { id, group_id } => {
                if id == TASKS_LIST {
                    return Err(DomainError::ProtectedList);
                }
                let index = self.list_index(id)?;
                if let Some(id) = group_id {
                    self.group(id).ok_or(DomainError::NotFound("Group", id))?;
                }
                self.lists[index].group_id = group_id;
                Ok(Changes::lists())
            }
            Action::ReorderList { id, before } => self.reorder_list(id, before),
            Action::AddGroup { name } => {
                let name = nonempty(name)?;
                let id = self.allocate_id();
                self.groups.push(ListGroup {
                    id,
                    name,
                    collapsed: false,
                    position: id as i64,
                });
                Ok(Changes {
                    created_group: Some(id),
                    ..Changes::lists()
                })
            }
            Action::RenameGroup { id, name } => {
                let name = nonempty(name)?;
                let group = self
                    .groups
                    .iter_mut()
                    .find(|group| group.id == id)
                    .ok_or(DomainError::NotFound("Group", id))?;
                group.name = name;
                Ok(Changes::lists())
            }
            Action::DeleteGroup(id) => {
                let index = self
                    .groups
                    .iter()
                    .position(|group| group.id == id)
                    .ok_or(DomainError::NotFound("Group", id))?;
                self.groups.remove(index);
                for list in &mut self.lists {
                    if list.group_id == Some(id) {
                        list.group_id = None;
                    }
                }
                Ok(Changes::lists())
            }
            Action::ToggleGroup(id) => {
                let group = self
                    .groups
                    .iter_mut()
                    .find(|group| group.id == id)
                    .ok_or(DomainError::NotFound("Group", id))?;
                group.collapsed = !group.collapsed;
                Ok(Changes::lists())
            }
            _ => Err(DomainError::InvalidValue("Unsupported list command".into())),
        }
    }

    fn reorder_list(&mut self, id: Id, before: Option<Id>) -> Result<Changes, DomainError> {
        if id == TASKS_LIST || before == Some(TASKS_LIST) {
            return Err(DomainError::ProtectedList);
        }
        let index = self.list_index(id)?;
        let group = self.lists[index].group_id;
        if before == Some(id) {
            return Ok(Changes::default());
        }
        if let Some(before_id) = before {
            let target = self.list_index(before_id)?;
            if self.lists[target].group_id != group {
                return Err(DomainError::InvalidValue(
                    "Move the list into this group before reordering".into(),
                ));
            }
        }
        let mut ids: Vec<_> = self
            .lists
            .iter()
            .filter(|list| list.group_id == group && list.id != TASKS_LIST)
            .map(|list| (list.position, list.id))
            .collect();
        ids.sort_unstable();
        let mut ids: Vec<_> = ids
            .into_iter()
            .map(|(_, id)| id)
            .filter(|value| *value != id)
            .collect();
        let position = before
            .and_then(|before_id| ids.iter().position(|value| *value == before_id))
            .unwrap_or(ids.len());
        ids.insert(position, id);
        let positions: std::collections::HashMap<_, _> = ids
            .into_iter()
            .enumerate()
            .map(|(position, id)| (id, position as i64))
            .collect();
        for list in &mut self.lists {
            if let Some(position) = positions.get(&list.id) {
                list.position = *position;
            }
        }
        Ok(Changes::lists())
    }
}

pub(crate) fn valid_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

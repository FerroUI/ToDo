use crate::{Action, AppState, Changes, DomainError, ListGroup, action::nonempty};

impl AppState {
    pub(super) fn apply_group(&mut self, action: Action) -> Result<Changes, DomainError> {
        match action {
            Action::AddGroup { name } => {
                let name = nonempty(name)?;
                let position = self
                    .groups
                    .iter()
                    .map(|group| group.position)
                    .max()
                    .unwrap_or(-1)
                    .checked_add(1)
                    .ok_or_else(|| {
                        DomainError::InvalidValue("The group order is out of range".into())
                    })?;
                let id = self.allocate_id();
                self.groups.push(ListGroup {
                    id,
                    name,
                    collapsed: false,
                    position,
                });
                self.append_sidebar(id);
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
                let mut order = self.sidebar_ids();
                let insertion = order
                    .iter()
                    .position(|candidate| *candidate == id)
                    .unwrap_or(order.len());
                order.retain(|candidate| *candidate != id);
                let mut members: Vec<_> = self
                    .lists
                    .iter()
                    .filter(|list| list.group_id == Some(id))
                    .map(|list| (list.position, list.id))
                    .collect();
                members.sort_unstable();
                order.splice(insertion..insertion, members.into_iter().map(|(_, id)| id));
                self.groups.remove(index);
                for list in &mut self.lists {
                    if list.group_id == Some(id) {
                        list.group_id = None;
                    }
                }
                self.sidebar_order = order;
                self.refresh_legacy_sidebar_positions();
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
            _ => Err(DomainError::InvalidValue(
                "Unsupported group command".into(),
            )),
        }
    }
}

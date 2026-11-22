use crate::{AppState, Changes, DomainError, Id, TASKS_LIST};
use std::collections::HashMap;

impl AppState {
    pub(super) fn next_list_position(&self, group_id: Option<Id>) -> Result<i64, DomainError> {
        self.lists
            .iter()
            .filter(|list| list.id != TASKS_LIST && list.group_id == group_id)
            .map(|list| list.position)
            .max()
            .unwrap_or(-1)
            .checked_add(1)
            .ok_or_else(|| {
                DomainError::InvalidValue("The destination list order is out of range".into())
            })
    }

    pub(super) fn move_list(
        &mut self,
        id: Id,
        group_id: Option<Id>,
    ) -> Result<Changes, DomainError> {
        if id == TASKS_LIST {
            return Err(DomainError::ProtectedList);
        }
        let index = self.list_index(id)?;
        if let Some(id) = group_id {
            self.group(id).ok_or(DomainError::NotFound("Group", id))?;
        }
        if self.lists[index].group_id == group_id {
            return Ok(Changes::default());
        }
        let position = self.next_list_position(group_id)?;
        self.lists[index].group_id = group_id;
        self.lists[index].position = position;
        if let Some(group_id) = group_id {
            self.sidebar_order.retain(|candidate| *candidate != id);
            if let Some(group) = self.groups.iter_mut().find(|group| group.id == group_id) {
                group.collapsed = false;
            }
        } else {
            self.append_sidebar(id);
        }
        Ok(Changes::lists())
    }

    pub(super) fn reorder_list(
        &mut self,
        id: Id,
        before: Option<Id>,
    ) -> Result<Changes, DomainError> {
        if id == TASKS_LIST || before == Some(TASKS_LIST) {
            return Err(DomainError::ProtectedList);
        }
        let index = self.list_index(id)?;
        let group_id = self.lists[index].group_id;
        if let Some(before) = before {
            let target = self.list_index(before)?;
            if self.lists[target].group_id != group_id {
                return Err(DomainError::InvalidValue(
                    "Move the list into this group before reordering".into(),
                ));
            }
        }
        let siblings = self
            .lists
            .iter()
            .filter(|list| list.group_id == group_id && list.id != TASKS_LIST)
            .map(|list| (list.position, list.id))
            .collect();
        let Some(positions) = reordered_positions(siblings, id, before) else {
            return Ok(Changes::default());
        };
        for list in &mut self.lists {
            if let Some(position) = positions.get(&list.id) {
                list.position = *position;
            }
        }
        if group_id.is_none() {
            self.refresh_sidebar_slots(false);
        }
        Ok(Changes::lists())
    }

    pub(super) fn reorder_group(
        &mut self,
        id: Id,
        before: Option<Id>,
    ) -> Result<Changes, DomainError> {
        self.group(id).ok_or(DomainError::NotFound("Group", id))?;
        if let Some(before) = before {
            self.group(before)
                .ok_or(DomainError::NotFound("Group", before))?;
        }
        let siblings = self
            .groups
            .iter()
            .map(|group| (group.position, group.id))
            .collect();
        let Some(positions) = reordered_positions(siblings, id, before) else {
            return Ok(Changes::default());
        };
        for group in &mut self.groups {
            if let Some(position) = positions.get(&group.id) {
                group.position = *position;
            }
        }
        self.refresh_sidebar_slots(true);
        Ok(Changes::lists())
    }
}

/// All identifiers are validated by the caller. No-op drops preserve even sparse positions.
fn reordered_positions(
    mut siblings: Vec<(i64, Id)>,
    id: Id,
    before: Option<Id>,
) -> Option<HashMap<Id, i64>> {
    if before == Some(id) {
        return None;
    }
    siblings.sort_unstable();
    let original: Vec<_> = siblings.into_iter().map(|(_, id)| id).collect();
    let mut ids: Vec<_> = original
        .iter()
        .copied()
        .filter(|value| *value != id)
        .collect();
    let position = before
        .and_then(|before| ids.iter().position(|value| *value == before))
        .unwrap_or(ids.len());
    ids.insert(position, id);
    (ids != original).then(|| {
        ids.into_iter()
            .enumerate()
            .map(|(position, id)| (id, position as i64))
            .collect()
    })
}

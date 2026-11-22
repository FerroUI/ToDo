use crate::{AppState, Changes, DomainError, Id, TASKS_LIST};
use std::collections::{HashMap, HashSet};

/// Navigation identities remain typed even though entities share one identifier namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarItem {
    List(Id),
    Group(Id),
}

impl SidebarItem {
    pub fn id(self) -> Id {
        match self {
            Self::List(id) | Self::Group(id) => id,
        }
    }
}

impl AppState {
    /// Old documents retain root-list-then-group order. New or remotely created roots
    /// missing from the preference are appended deterministically; stale IDs are ignored.
    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let mut lists: Vec<_> = self
            .lists
            .iter()
            .filter(|list| list.id != TASKS_LIST && list.group_id.is_none())
            .map(|list| (list.position, list.id))
            .collect();
        let mut groups: Vec<_> = self
            .groups
            .iter()
            .map(|group| (group.position, group.id))
            .collect();
        lists.sort_unstable();
        groups.sort_unstable();
        let legacy: Vec<_> = lists
            .into_iter()
            .map(|(_, id)| SidebarItem::List(id))
            .chain(groups.into_iter().map(|(_, id)| SidebarItem::Group(id)))
            .collect();
        let mut remaining: HashMap<_, _> = legacy.iter().map(|item| (item.id(), *item)).collect();
        let mut ordered: Vec<_> = self
            .sidebar_order
            .iter()
            .filter_map(|id| remaining.remove(id))
            .collect();
        ordered.extend(
            legacy
                .iter()
                .filter_map(|item| remaining.remove(&item.id())),
        );
        ordered
    }

    pub fn sidebar_ids(&self) -> Vec<Id> {
        self.sidebar_items()
            .into_iter()
            .map(SidebarItem::id)
            .collect()
    }

    /// Preference repair does not change tasks or topology. Keep an absent preference absent.
    pub fn repair_sidebar_order(&mut self) {
        if !self.sidebar_order.is_empty() {
            self.sidebar_order = self.sidebar_ids();
        }
    }

    pub(crate) fn validate_sidebar_order(&self) -> Result<(), DomainError> {
        let roots: HashSet<_> = self.sidebar_ids().into_iter().collect();
        let mut seen = HashSet::new();
        if self
            .sidebar_order
            .iter()
            .any(|id| !roots.contains(id) || !seen.insert(*id))
        {
            return Err(DomainError::InvalidData(
                "Navigation order contains an invalid or duplicate root".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn append_sidebar(&mut self, id: Id) {
        let mut order = self.sidebar_ids();
        order.retain(|candidate| *candidate != id);
        order.push(id);
        self.sidebar_order = order;
    }

    pub(crate) fn reorder_sidebar(
        &mut self,
        id: Id,
        before: Option<Id>,
    ) -> Result<Changes, DomainError> {
        if id == TASKS_LIST || before == Some(TASKS_LIST) {
            return Err(DomainError::ProtectedList);
        }
        let original = self.sidebar_ids();
        for candidate in std::iter::once(id).chain(before) {
            if !original.contains(&candidate) {
                return Err(DomainError::InvalidValue(
                    "Only root lists and groups can be reordered here".into(),
                ));
            }
        }
        if before == Some(id) {
            return Ok(Changes::default());
        }
        let mut order: Vec<_> = original
            .iter()
            .copied()
            .filter(|candidate| *candidate != id)
            .collect();
        let index = before
            .and_then(|before| order.iter().position(|candidate| *candidate == before))
            .unwrap_or(order.len());
        order.insert(index, id);
        if order == original {
            return Ok(Changes::default());
        }
        self.sidebar_order = order;
        self.refresh_legacy_sidebar_positions();
        Ok(Changes::lists())
    }

    /// Legacy sibling-order commands keep mixed roots in their existing slots.
    pub(crate) fn refresh_sidebar_slots(&mut self, groups: bool) {
        if self.sidebar_order.is_empty() {
            return;
        }
        self.sidebar_order = self.sidebar_ids();
        let mut siblings: Vec<_> = if groups {
            self.groups
                .iter()
                .map(|group| (group.position, group.id))
                .collect()
        } else {
            self.lists
                .iter()
                .filter(|list| list.id != TASKS_LIST && list.group_id.is_none())
                .map(|list| (list.position, list.id))
                .collect()
        };
        siblings.sort_unstable();
        let ids: HashSet<_> = siblings.iter().map(|(_, id)| *id).collect();
        let mut next = siblings.into_iter().map(|(_, id)| id);
        for id in &mut self.sidebar_order {
            if ids.contains(id) {
                *id = next.next().expect("one root slot per sibling");
            }
        }
    }

    pub(crate) fn refresh_legacy_sidebar_positions(&mut self) {
        let order = self.sidebar_items();
        let mut list_position = 0;
        let mut group_position = 0;
        for item in order {
            match item {
                SidebarItem::List(id) => {
                    if let Some(list) = self.lists.iter_mut().find(|list| list.id == id) {
                        list.position = list_position;
                        list_position += 1;
                    }
                }
                SidebarItem::Group(id) => {
                    if let Some(group) = self.groups.iter_mut().find(|group| group.id == id) {
                        group.position = group_position;
                        group_position += 1;
                    }
                }
            }
        }
    }
}

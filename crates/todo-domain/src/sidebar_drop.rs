use crate::{Action, AppState, Id, SidebarItem, TASKS_LIST};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarDrop {
    Before(SidebarItem),
    After(SidebarItem),
    IntoGroup(Id),
    RootEnd,
}

impl AppState {
    /// Plan against live topology both during hover and at drop. Nothing changes until apply.
    pub fn sidebar_drop_action(&self, source: SidebarItem, target: SidebarDrop) -> Option<Action> {
        let id = source.id();
        let old_group = match source {
            SidebarItem::List(id) if id != TASKS_LIST => self.list(id)?.group_id,
            SidebarItem::Group(id) => {
                self.group(id)?;
                None
            }
            _ => return None,
        };
        let (group_id, before, into) = match target {
            SidebarDrop::IntoGroup(group) => {
                self.group(group)?;
                (Some(group), None, true)
            }
            SidebarDrop::RootEnd => (None, None, false),
            SidebarDrop::Before(item) | SidebarDrop::After(item) => {
                if item == source {
                    return None;
                }
                let group = match item {
                    SidebarItem::List(id) if id != TASKS_LIST => self.list(id)?.group_id,
                    SidebarItem::Group(id) => {
                        self.group(id)?;
                        None
                    }
                    _ => return None,
                };
                let siblings = self.sidebar_siblings(group);
                let before = if matches!(target, SidebarDrop::After(_)) {
                    let index = siblings
                        .iter()
                        .position(|candidate| *candidate == item.id())?;
                    siblings[index + 1..]
                        .iter()
                        .copied()
                        .find(|candidate| *candidate != id)
                } else {
                    Some(item.id())
                };
                (group, before, false)
            }
        };
        if group_id.is_some() && matches!(source, SidebarItem::Group(_)) {
            return None;
        }
        let siblings = self.sidebar_siblings(group_id);
        let mut reordered: Vec<_> = siblings
            .iter()
            .copied()
            .filter(|candidate| *candidate != id)
            .collect();
        let index = before
            .and_then(|before| reordered.iter().position(|candidate| *candidate == before))
            .unwrap_or(reordered.len());
        reordered.insert(index, id);
        let moving = old_group != group_id;
        let expanding = into && self.group(group_id?).is_some_and(|group| group.collapsed);
        if !moving && reordered == siblings && !expanding {
            return None;
        }
        let mut actions = Vec::new();
        if moving {
            actions.push(Action::MoveList { id, group_id });
        }
        if group_id.is_some() {
            actions.push(Action::ReorderList { id, before });
        } else {
            actions.push(Action::ReorderSidebar { id, before });
        }
        if expanding && !moving {
            actions.push(Action::ToggleGroup(group_id.unwrap()));
        }
        Some(if actions.len() == 1 {
            actions.pop().unwrap()
        } else {
            Action::Batch(actions)
        })
    }

    pub fn sidebar_siblings(&self, group_id: Option<Id>) -> Vec<Id> {
        if group_id.is_none() {
            return self.sidebar_ids();
        }
        let mut lists: Vec<_> = self
            .lists
            .iter()
            .filter(|list| list.group_id == group_id)
            .map(|list| (list.position, list.id))
            .collect();
        lists.sort_unstable();
        lists.into_iter().map(|(_, id)| id).collect()
    }
}

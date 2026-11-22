use todo_domain::{Id, View};
use todo_state::{AppViewModel, PaneLayout, Panel, RenameTarget};

/// Compare transitions, not persistent modes: a panel already open when an edge
/// gesture starts must not cancel it on an unrelated clock or storage notification.
pub(super) struct Context {
    view: View,
    selected: Option<Id>,
    panel: Panel,
    rename: Option<RenameTarget>,
    selection: bool,
    layout: PaneLayout,
    refreshing: bool,
}

impl Context {
    pub fn capture(vm: &AppViewModel) -> Self {
        Self {
            view: vm.view(),
            selected: vm.selected(),
            panel: vm.panel(),
            rename: vm.rename_session().map(|session| session.target),
            selection: vm.selection_mode(),
            layout: vm.layout(),
            refreshing: vm.refreshing(),
        }
    }

    pub fn invalidates(&self, next: &Self, held: bool) -> bool {
        // Holding a task intentionally selects it and closes its details. Its
        // shell-owned pointer survives the resulting replacement of that row.
        let held_selection = held
            && !self.selection
            && next.selection
            && self.selected.is_some()
            && next.selected.is_none()
            && self.layout.navigation_overlay == next.layout.navigation_overlay
            && self.layout.compact == next.layout.compact;
        self.view != next.view
            || self.panel != next.panel
            || self.rename != next.rename
            || (self.layout != next.layout && !held_selection)
            || (!held && (self.selected != next.selected || self.selection != next.selection))
            || (!self.refreshing && next.refreshing)
    }
}

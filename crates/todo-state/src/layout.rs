use crate::{AppViewModel, Panel};
use todo_domain::Id;

pub const NAVIGATION_WIDTH: f64 = 280.0;
pub const MINIMUM_LIST_WIDTH: f64 = 360.0;
pub const DETAIL_WIDTH: f64 = 340.0;
pub const COMPACT_WIDTH: f64 = 760.0;

/// Pane topology is independent of individual resize pixels. Grid owns the remaining space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneLayout {
    pub compact: bool,
    pub can_inline_navigation: bool,
    pub navigation_inline: bool,
    pub navigation_overlay: bool,
    pub list_visible: bool,
    pub right_visible: bool,
}

impl PaneLayout {
    pub fn resolve(
        width: f64,
        navigation_preferred: bool,
        navigation_open: bool,
        right: bool,
    ) -> Self {
        let compact = width < COMPACT_WIDTH;
        let required =
            NAVIGATION_WIDTH + MINIMUM_LIST_WIDTH + if right { DETAIL_WIDTH } else { 0.0 };
        let can_inline_navigation = !compact && width >= required;
        Self {
            compact,
            can_inline_navigation,
            navigation_inline: can_inline_navigation && navigation_preferred,
            navigation_overlay: !can_inline_navigation && navigation_open,
            list_visible: !compact || !right,
            right_visible: right,
        }
    }
}

impl AppViewModel {
    pub fn layout(&self) -> PaneLayout {
        let right = self.selected().is_some()
            || matches!(self.panel(), Panel::Settings | Panel::Suggestions);
        PaneLayout::resolve(
            self.viewport(),
            self.sidebar.get(),
            self.navigation_open.get(),
            right,
        )
    }

    pub fn is_narrow(&self) -> bool {
        self.viewport() < COMPACT_WIDTH
    }

    pub fn sidebar_visible(&self) -> bool {
        let layout = self.layout();
        layout.navigation_inline || layout.navigation_overlay
    }

    pub fn sidebar_preferred(&self) -> bool {
        self.sidebar.get()
    }

    pub fn navigation_overlay_open(&self) -> bool {
        self.layout().navigation_overlay
    }

    pub fn toggle_sidebar(&self) {
        if self.layout().can_inline_navigation {
            self.sidebar.set(!self.sidebar.get());
            self.navigation_open.set(false);
        } else {
            self.navigation_open.set(!self.navigation_open.get());
        }
        self.notify();
    }

    pub fn close_navigation(&self) {
        if self.navigation_open.replace(false) {
            self.notify();
        }
    }

    pub fn set_viewport(&self, width: f64) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }
        let previous = self.layout();
        self.viewport.set(width);
        if self.layout().can_inline_navigation {
            self.navigation_open.set(false);
        }
        if previous != self.layout() {
            self.notify();
        }
    }

    /// Unsubmitted steps survive a temporary pane change without entering the task document.
    pub fn step_draft(&self, task_id: Id) -> String {
        self.step_drafts
            .borrow()
            .get(&task_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_step_draft(&self, task_id: Id, text: String) {
        let mut drafts = self.step_drafts.borrow_mut();
        if text.is_empty() {
            drafts.remove(&task_id);
        } else {
            drafts.insert(task_id, text);
        }
    }
}

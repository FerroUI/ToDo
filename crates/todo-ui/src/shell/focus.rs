use super::{focus_target, Drafts};
use crate::{focus_origin::FocusOrigin, rows::Rows};
use ferroui_base::input::KeyboardDevice;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::{Grid, TextBox};
use ferroui_view::NodeRef;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use todo_domain::Id;
use todo_state::{AppViewModel, PaneLayout, Panel};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Pane {
    List,
    Task(Id),
    Settings,
    Suggestions,
}

impl Pane {
    fn current(vm: &AppViewModel) -> Self {
        match vm.panel() {
            Panel::Settings => Self::Settings,
            Panel::Suggestions => Self::Suggestions,
            _ => vm.selected().map(Self::Task).unwrap_or(Self::List),
        }
    }
    fn side_panel(self) -> bool {
        matches!(self, Self::Settings | Self::Suggestions)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Presentation {
    layout: PaneLayout,
    pane: Pane,
}

#[derive(Default)]
pub(super) struct PaneFocus {
    previous: RefCell<Option<Presentation>>,
    detail_return: RefCell<Option<FocusOrigin>>,
    panel_return: RefCell<Option<FocusOrigin>>,
    navigation_return: RefCell<Option<FocusOrigin>>,
    generation: Cell<u64>,
}

impl PaneFocus {
    /// Observe pane transitions before reconciliation removes the currently focused control.
    pub(super) fn synchronize(
        self: &Rc<Self>,
        vm: &Rc<AppViewModel>,
        rows: &Rc<Rows>,
        drafts: &Rc<Drafts>,
        node: NodeRef<Grid>,
        layout: PaneLayout,
    ) {
        let next = Presentation {
            layout,
            pane: Pane::current(vm),
        };
        let Some(previous) = self.previous.replace(Some(next)) else {
            return;
        };
        if previous == next {
            return;
        }
        let focused = KeyboardDevice::instance().and_then(|keyboard| keyboard.focused_element());
        let focus_origin = focused.as_ref().map(|input| FocusOrigin::capture(input));
        let opening_navigation = !previous.layout.navigation_overlay && layout.navigation_overlay;
        let opening_pane = (next.pane.side_panel() && next.pane != previous.pane)
            || (matches!(next.pane, Pane::Task(_)) && previous.pane == Pane::List);
        if opening_navigation {
            *self.navigation_return.borrow_mut() = focus_origin.clone();
        }
        if next.pane.side_panel() && next.pane != previous.pane {
            *self.panel_return.borrow_mut() = focus_origin.clone();
        }
        if matches!(next.pane, Pane::Task(_)) && previous.pane == Pane::List {
            *self.detail_return.borrow_mut() = focus_origin;
        }
        let return_to = if previous.layout.navigation_overlay && !layout.navigation_overlay {
            self.navigation_return.borrow_mut().take()
        } else if previous.pane.side_panel() && !next.pane.side_panel() {
            self.panel_return.borrow_mut().take()
        } else if matches!(previous.pane, Pane::Task(_)) && next.pane == Pane::List {
            self.detail_return.borrow_mut().take()
        } else {
            None
        };
        let search_selection = focused
            .as_ref()
            .and_then(|focused| focused.cast::<TextBox>())
            .filter(|input| input.name().as_deref() == Some("SearchTasks"))
            .map(|input| (input.selection_start(), input.selection_end()));
        let Some(root) = node.get_untracked() else {
            return;
        };
        let root = root.downgrade();
        let vm = Rc::downgrade(vm);
        let rows = Rc::downgrade(rows);
        let drafts = Rc::downgrade(drafts);
        let owner = Rc::downgrade(self);
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        Dispatcher::ui_thread().post_local(
            move || {
                let (Some(owner), Some(root), Some(vm), Some(rows), Some(drafts)) = (
                    owner.upgrade(),
                    root.upgrade(),
                    vm.upgrade(),
                    rows.upgrade(),
                    drafts.upgrade(),
                ) else {
                    return;
                };
                if owner.generation.get() != generation
                    || !root.is_attached_to_visual_tree()
                    || !matches!(
                        vm.panel(),
                        Panel::None | Panel::Settings | Panel::Suggestions
                    )
                {
                    return;
                }
                focus_target::restore(
                    &root,
                    &vm,
                    &rows,
                    &drafts,
                    return_to,
                    opening_navigation,
                    opening_pane,
                    search_selection,
                );
            },
            DispatcherPriority::BACKGROUND,
        );
    }
}

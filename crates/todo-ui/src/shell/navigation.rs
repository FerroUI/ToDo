use super::Drafts;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::primitives::FlyoutBase;
use ferroui_controls::{ContextMenu, Control, Grid, TextBox};
use std::rc::Rc;
use todo_domain::View;
use todo_state::{AppViewModel, HostRequest, Panel};

struct NavigationActions(Rc<NavigationContext>);
impl PartialEq for NavigationActions {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

struct NavigationContext {
    vm: std::rc::Weak<AppViewModel>,
    drafts: Rc<Drafts>,
    root: WeakRef<Grid>,
}

pub(super) fn attach(root: &Ref<Grid>, vm: Rc<AppViewModel>, drafts: Rc<Drafts>) {
    root.set_tag(Some(Rc::new(NavigationActions(Rc::new(
        NavigationContext {
            vm: Rc::downgrade(&vm),
            drafts: drafts.clone(),
            root: root.downgrade(),
        },
    )))));
    let back_root = root.downgrade();
    let available_vm = Rc::downgrade(&vm);
    let available_root = root.downgrade();
    crate::touch::attach_shell(
        root,
        vm,
        Rc::new(move || back_root.upgrade().is_some_and(|root| navigate_back(&root))),
        Rc::new(move || {
            available_vm.upgrade().is_some_and(|vm| available(&vm))
                || available_root
                    .upgrade()
                    .is_some_and(|root| popup_open(&root))
        }),
    );
}

pub(crate) fn available(vm: &AppViewModel) -> bool {
    vm.rename_session().is_some()
        || vm.navigation_overlay_open()
        || vm.panel() != Panel::None
        || vm.selected().is_some()
        || vm.selection_mode()
        || matches!(vm.view(), View::Search(_))
        || vm.can_go_back_view()
}

pub(crate) fn sync_host(vm: &AppViewModel) {
    vm.request(HostRequest::NavigationBack {
        available: available(vm),
    });
}

/// Escape dismisses the current surface. Browser/edge Back may additionally return to a prior list.
pub(super) fn back(vm: &AppViewModel, drafts: &Drafts, include_history: bool) -> bool {
    if let Some(edit) = vm.rename_session() {
        vm.cancel_rename(edit.target);
    } else if vm.navigation_overlay_open() {
        vm.close_navigation();
    } else if vm.panel() != Panel::None {
        vm.show_panel(Panel::None);
    } else if vm.selected().is_some() {
        vm.select(None);
    } else if vm.selection_mode() {
        vm.end_selection();
    } else if matches!(vm.view(), View::Search(_)) {
        drafts.cancel_search(vm);
    } else if include_history {
        return vm.go_back_view();
    } else {
        return false;
    }
    true
}

fn open_menu(root: &Control) -> Option<Ref<ContextMenu>> {
    root.get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find_map(|control| control.context_menu().filter(|menu| menu.is_open()))
}

fn open_editor_flyout(root: &Control) -> Option<Ref<FlyoutBase>> {
    root.get_visual_descendants()
        .filter_map(|visual| visual.cast::<TextBox>())
        .find_map(|editor| {
            let owner: &Control = &editor;
            editor.context_flyout().filter(|flyout| {
                flyout.is_open()
                    && flyout
                        .target()
                        .is_some_and(|target| std::ptr::eq(&*target, owner))
            })
        })
}

fn popup_open(root: &Control) -> bool {
    open_editor_flyout(root).is_some() || open_menu(root).is_some()
}

pub(super) fn dismiss_popup(root: &Control) -> bool {
    // Native touch selection uses a transient flyout: focus stays in the editor,
    // so Escape does not route through the menu. Dismiss it before its containing pane.
    if let Some(flyout) = open_editor_flyout(root) {
        flyout.hide();
        return true;
    }
    open_menu(root).is_some_and(|menu| {
        menu.close();
        true
    })
}

pub fn navigate_back(root: &Control) -> bool {
    let Some(actions) = actions(root) else {
        return false;
    };
    let Some(vm) = actions.vm.upgrade() else {
        return false;
    };
    if let Some(root) = actions.root.upgrade() {
        if dismiss_popup(&root) {
            return true;
        }
        // Back cancels transient naming just like Escape. Flushing first would commit
        // the rename, clear its session, and dismiss another surface in the same action.
        if vm.rename_session().is_some() {
            return back(&vm, &actions.drafts, true);
        }
        crate::commit_pending_edits(&vm, &root);
    }
    back(&vm, &actions.drafts, true)
}

pub fn can_navigate_back(root: &Control) -> bool {
    actions(root).is_some_and(|actions| {
        actions.vm.upgrade().is_some_and(|vm| available(&vm)) || popup_open(root)
    })
}

fn actions(root: &Control) -> Option<Rc<NavigationContext>> {
    let root = root.find_descendant_of_type_where::<Grid>(true, |grid| grid.has_name("TodoApp"))?;
    root.tag()?
        .downcast_ref::<NavigationActions>()
        .map(|actions| actions.0.clone())
}

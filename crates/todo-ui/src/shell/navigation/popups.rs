use ferroui_base::Ref;
use ferroui_controls::primitives::FlyoutBase;
use ferroui_controls::{ContextMenu, Control, Grid, TextBox};
use ferroui_view::on_cleanup;
use std::rc::Rc;
use todo_state::AppViewModel;

/// Native editor flyouts can open without changing the view model. Observe their
/// accepted state so browser Back also works in the initial task composer.
pub(super) fn attach(root: &Ref<Grid>, vm: &Rc<AppViewModel>) {
    let root = root.downgrade();
    let vm = Rc::downgrade(vm);
    let changed = FlyoutBase::is_open_property()
        .changed()
        .add_class_handler::<FlyoutBase>(move |flyout, _| {
            let (Some(root), Some(vm), Some(target)) =
                (root.upgrade(), vm.upgrade(), flyout.target())
            else {
                return;
            };
            if root.is_attached_to_visual_tree() && root.is_visual_ancestor_of(&target) {
                super::sync_host(&vm, Some(&root));
            }
        });
    on_cleanup(move || drop(changed));
}

pub(super) fn is_open(root: &Control) -> bool {
    editor_flyout(root).is_some() || menu(root).is_some()
}

pub(super) fn dismiss(root: &Control) -> bool {
    // Touch selection keeps focus in its editor, so Escape does not route through
    // the transient menu. Dismiss that flyout before its containing pane.
    if let Some(flyout) = editor_flyout(root) {
        flyout.hide();
        return true;
    }
    menu(root).is_some_and(|menu| {
        menu.close();
        true
    })
}

fn menu(root: &Control) -> Option<Ref<ContextMenu>> {
    root.get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find_map(|control| control.context_menu().filter(|menu| menu.is_open()))
}

fn editor_flyout(root: &Control) -> Option<Ref<FlyoutBase>> {
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

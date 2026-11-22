use super::focus::ReturnFocus;
use ferroui_base::Ref;
use ferroui_controls::{ContextMenu, Control, MenuBase};
use ferroui_view::prelude::*;
use std::{cell::RefCell, rc::Rc};
use todo_state::AppViewModel;

/// One owned item tree per open menu. No handler retains its anchor or menu.
pub(super) fn attach(
    anchor: &Ref<Control>,
    vm: Rc<AppViewModel>,
    factory: impl Fn(&Control) -> Vec<Element> + 'static,
) {
    super::keys::attach(anchor);
    let menu = context_menu()
        .min_width(224.0)
        .max_width(360.0)
        .on_key_down(super::keys::isolate)
        .into_element()
        .mount()
        .cast::<ContextMenu>()
        .expect("ContextMenu element");
    let items_owner = Rc::new(RefCell::new(None::<Owner>));
    let restore = Rc::new(RefCell::new(None::<ReturnFocus>));
    let weak_menu = menu.downgrade();
    let weak_anchor = anchor.downgrade();
    let owned_items = items_owner.clone();
    let opening_focus = restore.clone();
    let opening = menu.opening(move |event| {
        let (Some(menu), Some(anchor)) = (weak_menu.upgrade(), weak_anchor.upgrade()) else {
            event.set_cancel(true);
            return;
        };
        if menu.is_open() {
            return;
        }
        let items = factory(&anchor);
        if items.is_empty() {
            event.set_cancel(true);
            return;
        }
        *opening_focus.borrow_mut() = Some(ReturnFocus::capture(&anchor));
        menu.items().clear();
        if let Some(previous) = owned_items.borrow_mut().take() {
            previous.dispose();
        }
        let owner = Owner::new_root();
        owner.with(|| {
            for item in items {
                menu.items().add(Some(Control::boxed(&item.mount())));
            }
        });
        *owned_items.borrow_mut() = Some(owner);
    });
    let closed = menu.add_handler(MenuBase::closed_event(), move |_, _| {
        if let Some(restore) = restore.borrow_mut().take() {
            restore.schedule(vm.clone());
        }
    });
    anchor.set_context_menu(Some(menu.clone()));
    let weak_menu = menu.downgrade();
    let weak_anchor = anchor.downgrade();
    on_cleanup(move || {
        drop(opening);
        if let Some(menu) = weak_menu.upgrade() {
            menu.close();
            menu.remove_handler(MenuBase::closed_event(), closed);
            menu.items().clear();
            if let Some(anchor) = weak_anchor.upgrade() {
                if anchor.context_menu().as_ref() == Some(&menu) {
                    anchor.set_context_menu(None);
                }
            }
        }
        if let Some(owner) = items_owner.borrow_mut().take() {
            owner.dispose();
        }
    });
}

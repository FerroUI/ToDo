use super::{input::Drafts, keyboard};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Grid, TopLevel};
use ferroui_view::on_cleanup;
use std::{cell::RefCell, rc::Rc};
use todo_state::AppViewModel;

/// TopLevel is the keyboard target when recycling/removal clears child focus.
/// One bubbling route covers that case and normal input without stealing keys
/// already handled by a TextBox, ListBox or child button.
pub(super) fn attach(root: &Ref<Grid>, vm: Rc<AppViewModel>, drafts: Rc<Drafts>) {
    let route = Rc::new(TopLevelRoute::default());
    let weak_root = root.downgrade();
    let weak_vm = Rc::downgrade(&vm);
    let weak_drafts = Rc::downgrade(&drafts);
    let connected = route.clone();
    let attached = root.attached_to_visual_tree(move |_| {
        if let (Some(root), Some(vm), Some(drafts)) = (
            weak_root.upgrade(),
            weak_vm.upgrade(),
            weak_drafts.upgrade(),
        ) {
            connected.connect(&root, &vm, &drafts);
        }
    });
    let disconnected = route.clone();
    let detached = root.detached_from_visual_tree(move |_| disconnected.clear());
    // The mount hook normally runs before attachment; this also supports an
    // already attached root without waiting for another lifecycle event.
    route.connect(root, &vm, &drafts);
    on_cleanup(move || {
        attached.dispose();
        detached.dispose();
        route.clear();
    });
}

#[derive(Default)]
struct TopLevelRoute(RefCell<Option<(WeakRef<TopLevel>, RoutedEventHandlerToken)>>);

impl TopLevelRoute {
    fn connect(&self, root: &Ref<Grid>, vm: &Rc<AppViewModel>, drafts: &Rc<Drafts>) {
        self.clear();
        if !root.is_attached_to_visual_tree() {
            return;
        }
        let Some(top_level) = TopLevel::get_top_level(Some(root)) else {
            return;
        };
        let root = root.downgrade();
        let vm = Rc::downgrade(vm);
        let drafts = Rc::downgrade(drafts);
        let token = top_level.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                let Some(root) = root.upgrade().filter(|root| {
                    root.is_attached_to_visual_tree() && root.is_effectively_visible()
                }) else {
                    return;
                };
                if let (Some(vm), Some(drafts)) = (vm.upgrade(), drafts.upgrade()) {
                    keyboard::handle(&vm, &drafts, event);
                }
                drop(root);
            },
            RoutingStrategies::BUBBLE,
            false,
        );
        *self.0.borrow_mut() = Some((top_level.downgrade(), token));
    }

    fn clear(&self) {
        let previous = self.0.borrow_mut().take();
        if let Some((top_level, token)) = previous {
            if let Some(top_level) = top_level.upgrade() {
                top_level.remove_handler(InputElement::key_down_event(), token);
            }
        }
    }
}

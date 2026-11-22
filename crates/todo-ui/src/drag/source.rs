use super::cancellation::Cancellation;
use super::{feedback, format, sidebar_feedback, sidebar_identity, DragItem, Identity, ItemDrag};
use ferroui_base::input::{
    DataTransfer, DataTransferItem, DragDrop, DragDropEffects, FocusManager, InputElement, Key,
    KeyModifiers, PointerPressedEventArgs, PointerType,
};
use ferroui_base::interactivity::{RoutedEventArgs, RoutingStrategies};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Point, Ref};
use ferroui_controls::{Button, TopLevel};
use ferroui_view::on_cleanup;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use todo_state::{AppViewModel, Panel};

const DRAGGED: &str = "todo-suppress-drag-click";
type Press = (DragItem, Point, PointerPressedEventArgs);

#[derive(Clone, Copy)]
enum Source {
    Task(Identity),
    Sidebar,
}

impl Source {
    fn identity(self, button: &Button) -> Option<DragItem> {
        match self {
            Self::Task(identity) => identity(button).map(DragItem::Task),
            Self::Sidebar => sidebar_identity(button).map(DragItem::Sidebar),
        }
    }
}

/// Observe the normal Button press without taking its pointer until the threshold is crossed.
pub(crate) fn attach_source(button: &Ref<Button>, vm: Rc<AppViewModel>, identity: Identity) {
    attach(button, vm, Source::Task(identity));
}

pub(crate) fn attach_sidebar_source(button: &Ref<Button>, vm: Rc<AppViewModel>) {
    crate::touch::attach_sidebar_source(button, vm.clone());
    attach(button, vm, Source::Sidebar);
}

fn attach(button: &Ref<Button>, vm: Rc<AppViewModel>, source: Source) {
    let pending = Rc::new(RefCell::<Option<Press>>::new(None));
    let epoch = Rc::new(Cell::new(0u64));
    let arm = pending.clone();
    let arm_vm = vm.clone();
    let weak = button.downgrade();
    let pressed = button.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |_, event| {
            arm.borrow_mut().take();
            let Some(button) = weak.upgrade() else { return };
            button.classes().set(DRAGGED, false);
            if event.pointer().type_() != PointerType::Mouse
                || !event.properties().is_left_button_pressed
                || event.key_modifiers().intersects(
                    KeyModifiers::CONTROL
                        | KeyModifiers::META
                        | KeyModifiers::SHIFT
                        | KeyModifiers::ALT,
                )
                || arm_vm.selection_mode()
                || arm_vm.panel() != Panel::None
                || arm_vm.rename_session().is_some()
            {
                return;
            }
            if let Some(item) = source.identity(&button).filter(|item| item.exists(&arm_vm)) {
                *arm.borrow_mut() = Some((item, event.get_position(None), event.clone()));
            }
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let moving = pending.clone();
    let weak = button.downgrade();
    let moved = button.add_handler_with(
        InputElement::pointer_moved_event(),
        move |_, event| {
            let Some(button) = weak.upgrade() else { return };
            let mut press = moving.borrow_mut();
            let Some((item, start, _)) = press.as_ref() else {
                return;
            };
            if !event.properties().is_left_button_pressed || source.identity(&button) != Some(*item)
            {
                press.take();
                return;
            }
            let position = event.get_position(None);
            if (position.x - start.x).hypot(position.y - start.y) < 6.0 {
                return;
            }
            let (item, _, trigger) = press.take().expect("the pending press was checked");
            drop(press); // Starting DragDrop releases capture and can re-enter the cleanup handler.
            if vm.selection_mode()
                || vm.panel() != Panel::None
                || vm.rename_session().is_some()
                || !item.exists(&vm)
            {
                return;
            }
            event.set_handled(true);
            button.classes().set(DRAGGED, true);
            let current_epoch = epoch.get().wrapping_add(1);
            epoch.set(current_epoch);
            let active = Rc::new(Cell::new(true));
            let data = DataTransfer::new();
            data.add(DataTransferItem::create(
                &format(),
                Some(ItemDrag {
                    item,
                    owner: Rc::downgrade(&vm),
                    active: active.clone(),
                }),
            ));
            let root = TopLevel::get_top_level(Some(&button)).map(|root| root.downgrade());
            // Clear Button.IsPressed even when no platform drag source is available.
            trigger.pointer().capture(None);
            let cancellation = Cancellation::watch(active.clone());
            let operation = DragDrop::do_drag_drop_async(&trigger, data, DragDropEffects::MOVE);
            let finished_button = button.downgrade();
            let finished_epoch = epoch.clone();
            Dispatcher::ui_thread().spawn_local(async move {
                let _ = operation.await;
                drop(cancellation);
                let root = root.and_then(|root| root.upgrade());
                if let Some(root) = &root {
                    feedback::clear_root(&root);
                    sidebar_feedback::clear_root(&root);
                }
                let root = root.map(|root| root.downgrade());
                // Protect the current release, then let accessibility Invoke work without a new press.
                Dispatcher::ui_thread().post_local(
                    move || {
                        if finished_epoch.get() == current_epoch {
                            if let Some(button) = finished_button.upgrade() {
                                button.classes().set(DRAGGED, false);
                            }
                            if let (DragItem::Sidebar(item), Some(root)) =
                                (item, root.and_then(|root| root.upgrade()))
                            {
                                if let Some(button) =
                                    root.find_descendant_of_type_where::<Button>(false, |button| {
                                        sidebar_identity(button) == Some(item)
                                            && FocusManager::can_focus(button)
                                    })
                                {
                                    button.focus();
                                }
                            }
                        }
                    },
                    DispatcherPriority::BACKGROUND,
                );
            });
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let released_state = pending.clone();
    let released = button.add_handler_with(
        InputElement::pointer_released_event(),
        move |_, _| {
            released_state.borrow_mut().take();
        },
        RoutingStrategies::TUNNEL,
        true,
    );
    let lost_state = pending.clone();
    let lost = button.add_handler(InputElement::pointer_capture_lost_event(), move |_, _| {
        lost_state.borrow_mut().take();
    });
    let weak = button.downgrade();
    let keyboard_state = pending.clone();
    let key = button.add_handler_with(
        InputElement::key_down_event(),
        move |_, event| {
            if event.key == Key::Escape {
                keyboard_state.borrow_mut().take();
            }
            if let Some(button) = weak.upgrade() {
                button.classes().set(DRAGGED, false);
            }
        },
        RoutingStrategies::TUNNEL,
        true,
    );
    let weak = button.downgrade();
    on_cleanup(move || {
        pending.borrow_mut().take();
        if let Some(button) = weak.upgrade() {
            button.remove_handler(InputElement::pointer_pressed_event(), pressed);
            button.remove_handler(InputElement::pointer_moved_event(), moved);
            button.remove_handler(InputElement::pointer_released_event(), released);
            button.remove_handler(InputElement::pointer_capture_lost_event(), lost);
            button.remove_handler(InputElement::key_down_event(), key);
        }
    });
}

pub(crate) fn ignore_click(event: &RoutedEventArgs) -> bool {
    event
        .source()
        .and_then(|source| source.cast::<Button>())
        .is_some_and(|button| button.classes().remove(DRAGGED))
}

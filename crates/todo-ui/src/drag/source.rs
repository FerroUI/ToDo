use super::native::NativeDrag;
use super::{format, sidebar_identity, DragItem, Identity, ItemDrag};
use ferroui_base::input::{
    DataTransfer, DataTransferItem, DragDrop, DragDropEffects, InputElement, Key, KeyModifiers,
    PointerPressedEventArgs, PointerType,
};
use ferroui_base::interactivity::{RoutedEventArgs, RoutingStrategies};
use ferroui_base::{Point, Ref};
use ferroui_controls::Button;
use ferroui_view::on_cleanup;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use todo_state::{AppViewModel, Panel};

pub(super) const DRAGGED: &str = "todo-suppress-drag-click";
struct Press {
    session: Rc<NativeDrag>,
    start: Point,
    trigger: PointerPressedEventArgs,
}

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
            let previous = arm.borrow_mut().take();
            if let Some(previous) = previous {
                previous.session.cancel();
            }
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
                || arm_vm.panel() != Panel::None
                || arm_vm.rename_session().is_some()
                || crate::list::task_title::is_link_press(event)
            {
                return;
            }
            if let Some(item) = source.identity(&button).filter(|item| item.exists(&arm_vm)) {
                if let Some(session) =
                    NativeDrag::arm(&button, &arm_vm, item, event.pointer().clone())
                {
                    *arm.borrow_mut() = Some(Press {
                        session,
                        start: event.get_position(None),
                        trigger: event.clone(),
                    });
                }
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
            let Some(pending) = press.as_ref() else {
                return;
            };
            if event.pointer().type_() != PointerType::Mouse
                || event.pointer().id() != pending.session.pointer.id()
                || !event.properties().is_left_button_pressed
                || source.identity(&button) != Some(pending.session.origin.item)
                || !pending.session.active()
            {
                let canceled = press.take();
                drop(press);
                if let Some(canceled) = canceled {
                    canceled.session.cancel();
                }
                return;
            }
            let position = event.get_position(None);
            if (position.x - pending.start.x).hypot(position.y - pending.start.y) < 6.0 {
                return;
            }
            let Press {
                session, trigger, ..
            } = press.take().expect("the pending press was checked");
            drop(press); // Starting DragDrop releases capture and can re-enter the cleanup handler.
            if !session.start(&vm) {
                return;
            }
            let item = session.origin.item;
            event.set_handled(true);
            button.classes().set(DRAGGED, true);
            let current_epoch = epoch.get().wrapping_add(1);
            epoch.set(current_epoch);
            let portable = match item {
                DragItem::Task(_) => match vm
                    .prepare_task_clipboard(session.origin.task_ids(), false)
                    .and_then(|offer| crate::task_transfer::item(&offer.data))
                {
                    Ok(data) => Some(data),
                    Err(error) => {
                        vm.report_error(format!("External task drag is unavailable: {error}"));
                        None
                    }
                },
                DragItem::Sidebar(_) => None,
            };
            let effects = if portable.is_some() {
                DragDropEffects::COPY | DragDropEffects::MOVE
            } else {
                DragDropEffects::MOVE
            };
            let exported = portable.unwrap_or_else(DataTransferItem::new);
            exported.set(
                &format(),
                Some(ItemDrag {
                    item,
                    session: session.clone(),
                }),
            );
            let data = DataTransfer::new();
            data.add(exported);
            // Clear Button.IsPressed even when no platform drag source is available.
            session.release_capture_for_drag();
            let operation = DragDrop::do_drag_drop_async(&trigger, data, effects);
            super::completion::observe(
                operation,
                session,
                button.downgrade(),
                epoch.clone(),
                current_epoch,
            );
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let released_state = pending.clone();
    let released = button.add_handler_with(
        InputElement::pointer_released_event(),
        move |_, event| {
            let matches = released_state
                .borrow()
                .as_ref()
                .is_some_and(|press| press.session.pointer.id() == event.pointer().id());
            if matches {
                let released = released_state.borrow_mut().take();
                if let Some(released) = released {
                    released.session.finish();
                }
            }
        },
        RoutingStrategies::TUNNEL,
        true,
    );
    let lost_state = pending.clone();
    let lost = button.add_handler(InputElement::pointer_capture_lost_event(), move |_, _| {
        let lost = lost_state.borrow_mut().take();
        if let Some(lost) = lost {
            lost.session.cancel();
        }
    });
    let weak = button.downgrade();
    let keyboard_state = pending.clone();
    let key = button.add_handler_with(
        InputElement::key_down_event(),
        move |_, event| {
            if event.key == Key::Escape {
                let canceled = keyboard_state.borrow_mut().take();
                if let Some(canceled) = canceled {
                    canceled.session.cancel();
                }
            }
            if let Some(button) = weak.upgrade() {
                button.classes().set(DRAGGED, false);
            }
        },
        RoutingStrategies::TUNNEL,
        true,
    );
    let detached_state = pending.clone();
    let detached = button.detached_from_visual_tree(move |_| {
        let pending = detached_state.borrow_mut().take();
        if let Some(pending) = pending {
            pending.session.cancel();
        }
    });
    let weak = button.downgrade();
    on_cleanup(move || {
        detached.dispose();
        let pending = pending.borrow_mut().take();
        if let Some(pending) = pending {
            pending.session.cancel();
        }
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

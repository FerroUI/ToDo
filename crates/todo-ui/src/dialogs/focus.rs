use ferroui_base::input::{FocusManager, InputElement, KeyboardDevice};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Border, Control, TextBox};
use ferroui_view::on_cleanup;
use std::{cell::RefCell, rc::Rc};

#[derive(PartialEq)]
struct ModalSurface;

struct ReturnFocus {
    previous: Option<WeakRef<InputElement>>,
    root: RefCell<Option<WeakRef<Control>>>,
    step_origin: bool,
}

/// Enter the modal's tab cycle after it has been attached and laid out.
pub(super) fn activate(surface: &Ref<Border>) {
    let previous = KeyboardDevice::instance().and_then(|keyboard| keyboard.focused_element());
    let restore = Rc::new(ReturnFocus {
        step_origin: previous
            .as_ref()
            .and_then(|control| control.name())
            .is_some_and(|name| name.starts_with("StepOptions")),
        previous: previous.as_ref().map(Ref::downgrade),
        root: RefCell::new(None),
    });
    // Mark pending modals before their deferred activation. A closing modal
    // must not restore focus over a replacement that has yet to receive it.
    surface.set_tag(Some(Rc::new(ModalSurface)));
    let closing = restore.clone();
    on_cleanup(move || {
        Dispatcher::ui_thread().post_local(
            move || restore_focus(&closing),
            DispatcherPriority::BACKGROUND,
        );
    });
    let weak = surface.downgrade();
    Dispatcher::ui_thread().post_local(
        move || {
            let Some(surface) = weak
                .upgrade()
                .filter(|surface| surface.is_attached_to_visual_tree())
            else {
                return;
            };
            *restore.root.borrow_mut() = surface
                .get_visual_ancestors()
                .filter_map(|visual| visual.cast::<Control>())
                .find(|control| control.name().as_deref() == Some("TodoApp"))
                .map(|root| root.downgrade());
            if let Some(input) =
                surface.find_descendant_of_type_where::<TextBox>(false, |input| available(input))
            {
                input.focus();
            } else if let Some(control) =
                surface.find_descendant_of_type_where::<Control>(false, available)
            {
                control.focus();
            }
        },
        DispatcherPriority::BACKGROUND,
    );
}

fn restore_focus(state: &ReturnFocus) {
    let Some(root) = state
        .root
        .borrow()
        .as_ref()
        .and_then(WeakRef::upgrade)
        .filter(|root| root.is_attached_to_visual_tree() && root.is_effectively_visible())
    else {
        return;
    };
    if root
        .find_descendant_of_type_where::<Border>(false, |surface| {
            surface.is_effectively_visible()
                && surface
                    .tag()
                    .is_some_and(|tag| tag.downcast_ref::<ModalSurface>().is_some())
        })
        .is_some()
        || KeyboardDevice::instance()
            .and_then(|keyboard| keyboard.focused_element())
            .is_some_and(|focused| FocusManager::can_focus(&focused))
    {
        return;
    }
    if state
        .previous
        .as_ref()
        .and_then(WeakRef::upgrade)
        .filter(|previous| FocusManager::can_focus(previous))
        .is_some_and(|previous| previous.focus())
    {
        return;
    }
    // A promoted/deleted step removes its menu button. Keep editing in the
    // same task; other dialogs return to the task composer when possible.
    let names: &[&str] = if state.step_origin {
        &["NewStep", "NewTask"]
    } else {
        &["NewTask"]
    };
    for name in names {
        if root
            .find_descendant_of_type_where::<TextBox>(false, |input| {
                input.name().as_deref() == Some(*name) && FocusManager::can_focus(input)
            })
            .is_some_and(|input| input.focus())
        {
            return;
        }
    }
    root.focus();
}

pub(super) fn name_field(input: &Ref<TextBox>) {
    let weak = input.downgrade();
    Dispatcher::ui_thread().post_local(
        move || {
            if let Some(input) = weak
                .upgrade()
                .filter(|input| input.is_attached_to_visual_tree())
            {
                input.focus();
                input.select_all();
            }
        },
        DispatcherPriority::BACKGROUND,
    );
}

fn available(control: &Control) -> bool {
    control.focusable()
        && control.is_tab_stop()
        && control.is_effectively_enabled()
        && control.is_effectively_visible()
}

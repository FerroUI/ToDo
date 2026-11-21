use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::Ref;
use ferroui_controls::{Border, Control, TextBox};

/// Enter the modal's tab cycle after it has been attached and laid out.
pub(super) fn activate(surface: &Ref<Border>) {
    let weak = surface.downgrade();
    Dispatcher::ui_thread().post_local(
        move || {
            let Some(surface) = weak
                .upgrade()
                .filter(|surface| surface.is_attached_to_visual_tree())
            else {
                return;
            };
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

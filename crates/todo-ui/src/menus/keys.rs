use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::Ref;
use ferroui_controls::Control;
use ferroui_view::on_cleanup;

pub(super) fn is_menu_key(event: &KeyEventArgs) -> bool {
    event.key == Key::Apps
        || (event.key == Key::F10 && event.key_modifiers.contains(KeyModifiers::SHIFT))
}

/// Menus own commands while open; application shortcuts must not modify the
/// selection behind the popup. Native arrows, access keys and Escape still route.
pub(super) fn isolate(event: &KeyEventArgs) {
    if event.key == Key::Delete
        || event
            .key_modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
    {
        event.set_handled(true);
    }
}

pub(super) fn attach(anchor: &Ref<Control>) {
    let down = anchor.add_handler_with(
        InputElement::key_down_event(),
        |_, event| {
            if is_menu_key(event) {
                event.set_handled(true);
            }
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let weak = anchor.downgrade();
    let up = anchor.add_handler_with(
        InputElement::key_up_event(),
        move |_, event| {
            if is_menu_key(event) {
                if let Some(anchor) = weak.upgrade() {
                    // Open on release, like Control's native Apps-key gesture. Opening
                    // on key-down lets the menu's key-up handler immediately close it.
                    super::open(&anchor);
                    event.set_handled(true);
                }
            }
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let weak = anchor.downgrade();
    on_cleanup(move || {
        if let Some(anchor) = weak.upgrade() {
            anchor.remove_handler(InputElement::key_down_event(), down);
            anchor.remove_handler(InputElement::key_up_event(), up);
        }
    });
}

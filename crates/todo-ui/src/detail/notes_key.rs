use ferroui_base::input::{InputElement, Key, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_controls::TextBox;
use ferroui_view::element::Handler;
use ferroui_view::prelude::*;
use std::rc::Rc;

/// Save before TextBox's bubbling Return handler inserts a native newline.
pub(super) fn with_commit(mut editor: El<TextBox>, commit: Rc<dyn Fn(&TextBox)>) -> El<TextBox> {
    // Reattach with the current declarative callback when the editor is retained.
    // A mount-only closure would keep the first render's model callback forever.
    editor
        .element_mut()
        .handlers
        .push(Handler::attached_again(Rc::new(move |control| {
            let commit = commit.clone();
            let weak = control.downgrade();
            let token = control.add_handler_with(
                InputElement::key_down_event(),
                move |_, event| {
                    if event.key == Key::Return
                        && event
                            .key_modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
                    {
                        if let Some(field) =
                            weak.upgrade().and_then(|field| field.cast::<TextBox>())
                        {
                            if field.is_composing_text() {
                                return;
                            }
                            event.set_handled(true);
                            commit(&field);
                        }
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let weak = control.downgrade();
            on_cleanup(move || {
                if let Some(control) = weak.upgrade() {
                    control.remove_handler(InputElement::key_down_event(), token);
                }
            });
        })));
    editor
}

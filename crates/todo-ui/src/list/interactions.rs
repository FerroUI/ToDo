use ferroui_base::input::{InputElement, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Button, Control};
use ferroui_view::on_cleanup;
use std::rc::Rc;
use todo_domain::Id;
use todo_state::AppViewModel;

/// Modifier gestures are intercepted before Button captures the pointer, so they do not also open details.
pub(super) fn attach(button: &Ref<Button>, vm: Rc<AppViewModel>) {
    let button_weak = button.downgrade();
    let token = button.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |_, event| {
            let Some(id) = button_weak.upgrade().and_then(|button| task_id(&button)) else {
                return;
            };
            if !event
                .get_current_point(None)
                .properties
                .is_left_button_pressed
            {
                return;
            }
            let modifiers = event.key_modifiers();
            if modifiers.contains(KeyModifiers::SHIFT) {
                vm.select_task_range(&vm.visible_task_ids(), id);
            } else if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META) {
                vm.toggle_task_selection(id);
            } else {
                return;
            }
            event.set_handled(true);
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let weak = button.downgrade();
    on_cleanup(move || {
        if let Some(button) = weak.upgrade() {
            button.remove_handler(InputElement::pointer_pressed_event(), token);
        }
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct TaskIdentity(Id);

pub(super) fn task_tag(id: Id) -> Option<BoxedValue> {
    Some(Rc::new(TaskIdentity(id)))
}

pub(super) fn task_id(control: &Control) -> Option<Id> {
    control
        .tag()
        .and_then(|tag| tag.downcast_ref::<TaskIdentity>().map(|value| value.0))
}

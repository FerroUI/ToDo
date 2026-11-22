use ferroui_base::input::{FocusManager, KeyboardDevice};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Control, TextBox};
use ferroui_view::on_cleanup;
use std::{cell::RefCell, rc::Rc};
use todo_state::RenameTarget;

pub(super) fn activate(input: &Ref<TextBox>, target: RenameTarget) {
    let root = Rc::new(RefCell::new(None::<WeakRef<Control>>));
    let closing = root.clone();
    on_cleanup(move || {
        let root = closing.borrow().clone();
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(root) = root.and_then(|weak| weak.upgrade()) else {
                    return;
                };
                if KeyboardDevice::instance()
                    .and_then(|keyboard| keyboard.focused_element())
                    .is_some_and(|input| FocusManager::can_focus(&input))
                {
                    return;
                }
                let name = match target {
                    RenameTarget::List(_) => "RenameList".into(),
                    RenameTarget::Group(id) => format!("Group{id}"),
                };
                if let Some(control) =
                    root.find_descendant_of_type_where::<Control>(false, |control| {
                        control.name().as_deref() == Some(name.as_str())
                            && FocusManager::can_focus(control)
                    })
                {
                    control.focus();
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    });
    let weak = input.downgrade();
    Dispatcher::ui_thread().post_local(
        move || {
            if let Some(input) = weak
                .upgrade()
                .filter(|input| input.is_attached_to_visual_tree())
            {
                *root.borrow_mut() = input
                    .get_visual_ancestors()
                    .filter_map(|visual| visual.cast::<Control>())
                    .find(|control| control.name().as_deref() == Some("TodoApp"))
                    .map(|control| control.downgrade());
                input.focus();
                input.select_all();
                input.bring_into_view();
            }
        },
        DispatcherPriority::BACKGROUND,
    );
}

use ferroui_base::input::{FocusManager, InputElement, KeyboardDevice};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Control, TextBox};
use std::rc::Rc;
use todo_state::{AppViewModel, Panel};

pub(super) struct ReturnFocus {
    previous: Option<WeakRef<InputElement>>,
    anchor: WeakRef<Control>,
    root: Option<WeakRef<Control>>,
}

impl ReturnFocus {
    pub fn capture(anchor: &Ref<Control>) -> Self {
        Self {
            previous: KeyboardDevice::instance()
                .and_then(|keyboard| keyboard.focused_element())
                .map(|input| input.downgrade()),
            anchor: anchor.downgrade(),
            root: anchor
                .get_visual_ancestors()
                .filter_map(|visual| visual.cast::<Control>())
                .find(|control| control.name().as_deref() == Some("TodoApp"))
                .map(|root| root.downgrade()),
        }
    }

    pub fn schedule(self, vm: Rc<AppViewModel>) {
        Dispatcher::ui_thread().post_local(
            move || {
                // Pointer light-dismiss or a command may have already focused another
                // control. A command opening a modal owns its own focus transition.
                if !matches!(
                    vm.panel(),
                    Panel::None | Panel::Suggestions | Panel::Settings
                ) || KeyboardDevice::instance()
                    .and_then(|keyboard| keyboard.focused_element())
                    .is_some_and(|focused| FocusManager::can_focus(&focused))
                {
                    return;
                }
                if self
                    .previous
                    .and_then(|weak| weak.upgrade())
                    .filter(|input| FocusManager::can_focus(input))
                    .is_some_and(|input| input.focus())
                {
                    return;
                }
                if let Some(anchor) = self.anchor.upgrade() {
                    if FocusManager::can_focus(&anchor) && anchor.focus() {
                        return;
                    }
                    if anchor
                        .find_descendant_of_type_where::<Control>(false, |control| {
                            FocusManager::can_focus(control)
                        })
                        .is_some_and(|control| control.focus())
                    {
                        return;
                    }
                }
                if let Some(root) = self.root.and_then(|weak| weak.upgrade()) {
                    if !root
                        .find_descendant_of_type_where::<TextBox>(false, |input| {
                            input.name().as_deref() == Some("NewTask")
                                && FocusManager::can_focus(input)
                        })
                        .is_some_and(|input| input.focus())
                    {
                        root.focus();
                    }
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
}

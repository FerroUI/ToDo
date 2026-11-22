use crate::focus_origin::FocusOrigin;
use ferroui_base::input::{FocusManager, KeyboardDevice};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Control, TextBox};
use std::rc::Rc;
use todo_state::{AppViewModel, Panel};

pub(super) struct ReturnFocus {
    previous: Option<FocusOrigin>,
    anchor: FocusOrigin,
    root: Option<WeakRef<Control>>,
    step_origin: bool,
}

impl ReturnFocus {
    pub fn capture(anchor: &Ref<Control>) -> Self {
        Self {
            step_origin: anchor
                .name()
                .is_some_and(|name| name.starts_with("StepOptions")),
            previous: KeyboardDevice::instance()
                .and_then(|keyboard| keyboard.focused_element())
                .map(|input| FocusOrigin::capture(&input)),
            anchor: FocusOrigin::capture(anchor),
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
                if self.previous.as_ref().is_some_and(FocusOrigin::restore) {
                    return;
                }
                if let Some(anchor) = self
                    .anchor
                    .resolve()
                    .and_then(|anchor| anchor.cast::<Control>())
                {
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
                    if self.step_origin
                        && root
                            .find_descendant_of_type_where::<TextBox>(false, |input| {
                                input.name().as_deref() == Some("NewStep")
                                    && FocusManager::can_focus(input)
                            })
                            .is_some_and(|input| input.focus())
                    {
                        return;
                    }
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

use ferroui_base::input::{FocusManager, InputElement};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Control, ListBoxItem};
use todo_domain::Id;

/// A recycled control can survive while its task changes. Focus returns to the captured
/// logical task only; the weak handle never extends a row or popup's lifetime.
#[derive(Clone)]
pub(crate) struct FocusOrigin {
    element: WeakRef<InputElement>,
    task: Option<Id>,
}

impl FocusOrigin {
    pub fn capture(element: &InputElement) -> Self {
        Self {
            element: element.to_ref().downgrade(),
            task: task_identity(element),
        }
    }

    /// Also guards nonfocusable menu anchors before searching their descendants.
    pub fn resolve(&self) -> Option<Ref<InputElement>> {
        self.element.upgrade().filter(|element| {
            element.is_attached_to_visual_tree()
                && element.is_effectively_visible()
                && task_identity(element) == self.task
        })
    }

    pub fn restore(&self) -> bool {
        self.resolve()
            .filter(|element| FocusManager::can_focus(element))
            .is_some_and(|element| element.focus())
    }
}

fn task_identity(element: &InputElement) -> Option<Id> {
    for control in element
        .get_self_and_visual_ancestors()
        .filter_map(|visual| visual.cast::<Control>())
    {
        if let Some(id) = crate::touch::task_id(&control) {
            return Some(id);
        }
        // Keyboard navigation focuses the native container, outside its task surface.
        if let Some(container) = control.cast::<ListBoxItem>() {
            return container
                .find_descendant_of_type_where::<Control>(false, |child| {
                    crate::touch::task_id(child).is_some()
                })
                .and_then(|surface| crate::touch::task_id(&surface));
        }
    }
    None
}

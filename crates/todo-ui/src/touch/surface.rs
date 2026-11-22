use super::{actions, task::TaskRecognizer, TaskSurfaceId};
use ferroui_base::input::{FocusManager, KeyboardDevice};
use ferroui_base::media::TranslateTransform;
use ferroui_base::{BoxedValue, Ref, WeakRef};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Border, Button, Control, Grid};
use ferroui_view::prelude::*;
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};
use todo_domain::Id;
use todo_state::{gestures::swipe::SwipeLimits, AppViewModel};

/// The existing, templated card remains intact; only its visual offset changes during a swipe.
pub(crate) fn task_surface(vm: Rc<AppViewModel>, id: Id, _dark: bool, card: Element) -> Element {
    let attach_vm = vm.clone();
    grid()
        .name(format!("TaskSwipe{id}"))
        .tag(Some(Rc::new(TaskSurfaceId(id)) as BoxedValue))
        .clip_to_bounds(true)
        .background("Transparent")
        .with(move |root| TaskRecognizer::attach(root, attach_vm.clone()))
        .child(actions::view(vm, id))
        .child(border().name("TaskSwipeContent").child(card))
        .into()
}

pub(super) struct Surface {
    pub root: WeakRef<Grid>,
    pub offset: Cell<f64>,
    transform: Ref<TranslateTransform>,
    vm: Weak<AppViewModel>,
}

impl Surface {
    pub fn new(root: &Ref<Grid>, vm: &Rc<AppViewModel>) -> Rc<Self> {
        Rc::new(Self {
            root: root.downgrade(),
            offset: Cell::new(0.0),
            transform: TranslateTransform::new(),
            vm: Rc::downgrade(vm),
        })
    }

    pub fn limits(&self) -> SwipeLimits {
        let (Some(root), Some(vm)) = (self.root.upgrade(), self.vm.upgrade()) else {
            return SwipeLimits::default();
        };
        let state = vm.state();
        let Some(task) = super::task_id(&root).and_then(|id| state.task(id)) else {
            return SwipeLimits::default();
        };
        let mounted = |name| {
            root.find_descendant_of_type_where::<Button>(false, |button| {
                button.name().as_deref() == Some(name)
            })
            .is_some()
        };
        SwipeLimits::new(
            mounted("SwipeMyDay"),
            task.can_move() && mounted("SwipeMove"),
            task.can_delete() && mounted("SwipeDelete"),
        )
    }

    pub fn render(&self, offset: f64) {
        let offset = self.limits().clamp(offset);
        let previous = self.offset.replace(offset);
        let Some(root) = self.root.upgrade() else {
            return;
        };
        let return_focus = offset == 0.0
            && previous != 0.0
            && KeyboardDevice::instance()
                .and_then(|keyboard| keyboard.focused_element())
                .is_some_and(|focused| {
                    root.is_visual_ancestor_of(&focused)
                        && matches!(
                            focused.name().as_deref(),
                            Some("SwipeMyDay" | "SwipeMove" | "SwipeDelete")
                        )
                });
        if let Some(content) = root.find_descendant_of_type_where::<Border>(false, |control| {
            control.name().as_deref() == Some("TaskSwipeContent")
        }) {
            content.set_render_transform(Some(self.transform.clone().into()));
            self.transform.set_x(offset);
        }
        for (name, visible) in [
            ("TaskSwipeRightActions", offset > 0.0),
            ("TaskSwipeDeleteAction", offset < 0.0),
        ] {
            if let Some(control) = root.find_descendant_of_type_where::<Control>(false, |control| {
                control.name().as_deref() == Some(name)
            }) {
                control.set_is_visible(visible);
            }
        }
        // Escape and action activation must not leave an invisible rail Button focused.
        // Only return this rail's focus; ordinary touch scrolling never steals focus.
        if return_focus && root.is_attached_to_visual_tree() {
            if let Some(content) = root.find_descendant_of_type_where::<Border>(false, |control| {
                control.name().as_deref() == Some("TaskSwipeContent")
            }) {
                if let Some(title) = content
                    .find_descendant_of_type_where::<Button>(false, |button| {
                        !button.is::<ToggleButton>() && FocusManager::can_focus(button)
                    })
                {
                    title.focus();
                }
            }
        }
    }

    pub fn dim(&self, dragging: bool) {
        if let Some(root) = self.root.upgrade() {
            if let Some(content) = root.find_descendant_of_type_where::<Border>(false, |control| {
                control.name().as_deref() == Some("TaskSwipeContent")
            }) {
                content.set_opacity(if dragging { 0.65 } else { 1.0 });
            }
        }
    }

    pub fn close(&self) {
        self.render(0.0);
        self.dim(false);
    }
}

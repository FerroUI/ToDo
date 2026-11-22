//! Geometry and host lifetime changes invalidate the coordinates of a live gesture.
use ferroui_base::reactive::Subscription;
use ferroui_base::Ref;
use ferroui_controls::{Control, Grid, TopLevel};
use ferroui_view::on_cleanup;
use std::{cell::RefCell, rc::Rc};

pub(super) fn attach(root: &Ref<Grid>) {
    let connections = Rc::new(Connections::default());
    let weak = root.downgrade();
    let connected = connections.clone();
    let attached = root.attached_to_visual_tree(move |_| {
        if let Some(root) = weak.upgrade() {
            connected.connect(&root);
        }
    });
    let disconnected = connections.clone();
    let detached = root.detached_from_visual_tree(move |_| disconnected.clear());
    let weak = root.downgrade();
    let resized = root.size_changed(move |_, _| {
        if let Some(root) = weak.upgrade() {
            crate::cancel_gestures(&root);
        }
    });
    connections.connect(root);
    let weak = root.downgrade();
    on_cleanup(move || {
        attached.dispose();
        detached.dispose();
        connections.clear();
        if let Some(root) = weak.upgrade() {
            root.remove_handler(Control::size_changed_event(), resized);
        }
    });
}

#[derive(Default)]
struct Connections(RefCell<Vec<Subscription>>);

impl Connections {
    fn connect(&self, root: &Ref<Grid>) {
        self.clear();
        if !root.is_attached_to_visual_tree() {
            return;
        }
        let Some(top_level) = TopLevel::get_top_level(Some(root)) else {
            return;
        };
        let interrupt = || {
            let root = root.downgrade();
            move || {
                if let Some(root) = root.upgrade() {
                    crate::cancel_gestures(&root);
                }
            }
        };
        *self.0.borrow_mut() = vec![
            top_level.platform_lost_focus(interrupt()),
            top_level.platform_deactivated(interrupt()),
            top_level.scaling_changed(interrupt()),
            top_level.closed(interrupt()),
        ];
    }

    fn clear(&self) {
        let previous = std::mem::take(&mut *self.0.borrow_mut());
        drop(previous);
    }
}

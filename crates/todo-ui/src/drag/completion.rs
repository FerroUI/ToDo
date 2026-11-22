use super::{native::NativeDrag, sidebar_identity, source::DRAGGED, DragItem};
use ferroui_base::input::{DragDropEffects, FocusManager, LocalBoxFuture};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::WeakRef;
use ferroui_controls::Button;
use std::{cell::Cell, rc::Rc};

pub(super) fn observe(
    operation: LocalBoxFuture<DragDropEffects>,
    session: Rc<NativeDrag>,
    button: WeakRef<Button>,
    epoch: Rc<Cell<u64>>,
    generation: u64,
) {
    Dispatcher::ui_thread().spawn_local(async move {
        let effect = operation.await;
        session.finish();
        let root = session.root.clone();
        let item = session.origin.item;
        // Protect the current release, then let accessibility Invoke work without a new
        // press. A canceled drag must not move focus out of a newly opened editor/pane.
        Dispatcher::ui_thread().post_local(
            move || {
                if epoch.get() != generation {
                    return;
                }
                if let Some(button) = button.upgrade() {
                    button.classes().set(DRAGGED, false);
                }
                if effect != DragDropEffects::MOVE {
                    return;
                }
                if let (DragItem::Sidebar(item), Some(root)) = (item, root.upgrade()) {
                    if let Some(button) =
                        root.find_descendant_of_type_where::<Button>(false, |button| {
                            sidebar_identity(button) == Some(item)
                                && FocusManager::can_focus(button)
                        })
                    {
                        button.focus();
                    }
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    });
}

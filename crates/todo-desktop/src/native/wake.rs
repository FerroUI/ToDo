use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

thread_local! { static CALLBACKS: RefCell<HashMap<u64, Rc<dyn Fn()>>> = RefCell::new(HashMap::new()); }
static NEXT: AtomicU64 = AtomicU64::new(1);

/// Only an integer crosses threads. Rc-based VM/window handles remain in the
/// dispatcher's thread-local registry, with no unsafe Send wrappers.
pub fn register(callback: impl Fn() + 'static) -> (u64, Arc<dyn Fn() + Send + Sync>) {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    CALLBACKS.with(|callbacks| callbacks.borrow_mut().insert(id, Rc::new(callback)));
    let dispatcher = Dispatcher::ui_thread();
    (
        id,
        Arc::new(move || {
            dispatcher.post(
                move || {
                    let callback = CALLBACKS.with(|callbacks| callbacks.borrow().get(&id).cloned());
                    if let Some(callback) = callback {
                        callback();
                    }
                },
                DispatcherPriority::BACKGROUND,
            )
        }),
    )
}

pub fn unregister(id: u64) {
    CALLBACKS.with(|callbacks| callbacks.borrow_mut().remove(&id));
}

use super::wake;
use ferroui_base::Ref;
use ferroui_controls::Window;
use std::{cell::Cell, rc::Rc, sync::mpsc};
use todo_state::AppViewModel;

pub fn show(
    vm: &Rc<AppViewModel>,
    window: &Ref<Window>,
    title: String,
    body: String,
    requested: bool,
) {
    let weak = Rc::downgrade(vm);
    let weak_window = window.downgrade();
    let (sender, result) = mpsc::channel();
    let token = Rc::new(Cell::new(0));
    let retained = token.clone();
    let (id, wake) = wake::register(move || {
        let Ok(outcome) = result.try_recv() else {
            return;
        };
        wake::unregister(retained.get());
        let Some(vm) = weak.upgrade() else {
            return;
        };
        match outcome {
            Ok(()) if requested => vm.set_status("A test reminder was requested. Allow To Do notifications in your system settings if it does not appear.".into()),
            Ok(()) => {},
            Err(error) => {
                vm.report_error(error);
                if let Some(window) = weak_window.upgrade() { if !window.is_visible() { window.show(); } }
            }
        }
    });
    token.set(id);
    std::thread::spawn(move || {
        let _ = sender.send(todo_desktop::notification_delivery::deliver(&title, &body));
        wake();
    });
}

use ferroui_base::{
    threading::{DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::Window;
use std::{cell::RefCell, path::Path, rc::Rc, time::Duration};
use todo_desktop::reminders::Scheduler;
use todo_state::AppViewModel;

pub fn install(
    vm: &Rc<AppViewModel>,
    window: &Ref<Window>,
    directory: &Path,
    scheduler: Scheduler,
) {
    vm.set_background_reminders_status(
        false,
        "Checking this computer's background reminder service…".into(),
    );
    if let Ok(json) = std::fs::read_to_string(directory.join("background-reminders-error.json")) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(message) = value.get("message").and_then(serde_json::Value::as_str) {
                vm.report_error(format!(
                    "The last background reminder attempt failed: {message}"
                ));
            }
        }
    }
    if let Ok(json) = vm.export_json() {
        scheduler.submit(&json);
    }
    let weak = Rc::downgrade(vm);
    let timer = DispatcherTimer::run(
        move || {
            let Some(vm) = weak.upgrade() else {
                return false;
            };
            if let Some(status) = scheduler.take_status() {
                vm.set_background_reminders_status(status.available, status.message.clone());
                if status.error {
                    vm.report_error(status.message);
                }
            }
            true
        },
        Duration::from_millis(500),
        DispatcherPriority::BACKGROUND,
    );
    let timer = RefCell::new(Some(timer));
    window
        .closed(move || {
            if let Some(timer) = timer.borrow_mut().take() {
                timer.dispose();
            }
        })
        .detach();
}

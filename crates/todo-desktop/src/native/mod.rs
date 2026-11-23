pub mod audio;
mod background;
mod callbacks;
mod credentials;
pub use credentials::child_main as credential_child;
pub mod notifications;
mod process;
mod pst;
mod pst_limits;
pub use pst::child_main as pst_child;
#[cfg(test)]
mod tests;
mod transport;
mod wake;

use ferroui_base::{
    threading::{DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::Window;
use serde_json::{json, Value};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
use todo_state::AppViewModel;
use transport::{Event, Request, Transport};

pub struct NativeServices {
    transport: Transport,
    queue: RefCell<VecDeque<Value>>,
    epoch: Cell<u64>,
    busy: Cell<bool>,
    available: Cell<bool>,
    checked: Cell<bool>,
    runtime_error: RefCell<Option<String>>,
    paused: Cell<bool>,
    failures: Cell<u32>,
    next_sync: Cell<Instant>,
    changed: Cell<Option<Instant>>,
    revision: Cell<u64>,
    document: RefCell<String>,
    demo: bool,
    wake_id: u64,
}

impl NativeServices {
    pub fn install(
        vm: &Rc<AppViewModel>,
        window: &Ref<Window>,
        directory: PathBuf,
        demo: bool,
    ) -> Rc<Self> {
        if !demo {
            background::install(vm, window);
        }
        let directory = if demo {
            std::env::temp_dir().join(format!("ferroui-todo-demo-{}", std::process::id()))
        } else {
            directory
        };
        let weak_vm = Rc::downgrade(vm);
        let weak_window = window.downgrade();
        let host = Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let weak_host = weak.clone();
            let (wake_id, wake) = wake::register(move || {
                if let (Some(host), Some(vm), Some(window)) = (
                    weak_host.upgrade(),
                    weak_vm.upgrade(),
                    weak_window.upgrade(),
                ) {
                    host.poll(&vm, &window);
                }
            });
            Self {
                transport: Transport::start(directory, demo, wake),
                queue: RefCell::new(VecDeque::from([json!({"kind":"initialize"})])),
                epoch: Cell::new(0),
                busy: Cell::new(false),
                available: Cell::new(false),
                checked: Cell::new(false),
                runtime_error: RefCell::new(None),
                paused: Cell::new(false),
                failures: Cell::new(0),
                next_sync: Cell::new(Instant::now() + Duration::from_secs(5)),
                changed: Cell::new(None),
                revision: Cell::new(vm.revision()),
                document: RefCell::new(vm.export_json().unwrap_or_default()),
                demo,
                wake_id,
            }
        });
        vm.set_cloud_hint("Checking the native Microsoft service runtime…");
        let weak = Rc::downgrade(&host);
        let weak_vm = Rc::downgrade(vm);
        let weak_window = window.downgrade();
        let timer = DispatcherTimer::run(
            move || {
                let (Some(host), Some(vm), Some(window)) =
                    (weak.upgrade(), weak_vm.upgrade(), weak_window.upgrade())
                else {
                    return false;
                };
                host.poll(&vm, &window);
                true
            },
            Duration::from_millis(250),
            DispatcherPriority::BACKGROUND,
        );
        let timer = RefCell::new(Some(timer));
        let closed = host.clone();
        window
            .closed(move || {
                wake::unregister(closed.wake_id);
                closed.cancel();
                if let Some(timer) = timer.borrow_mut().take() {
                    timer.dispose();
                }
            })
            .detach();
        host
    }

    pub fn submit(&self, vm: &AppViewModel, action: Value) -> Result<(), String> {
        if self.checked.get() && !self.available.get() {
            let error = self.runtime_error.borrow().clone().unwrap_or_else(|| {
                "Native services are unavailable. Restart To Do to retry.".into()
            });
            self.unavailable(vm, &error);
            return Err(error);
        }
        let kind = action["kind"].as_str().unwrap_or("");
        if kind == "disconnectMicrosoft" {
            self.cancel();
        }
        if kind == "syncMicrosoft" {
            self.paused.set(false);
            self.queue
                .borrow_mut()
                .retain(|queued| queued["kind"] != "syncMicrosoft");
        }
        self.queue.borrow_mut().push_back(action);
        self.start_next()
            .inspect_err(|error| self.unavailable(vm, error))
    }

    pub fn cancel(&self) {
        self.epoch.set(self.epoch.get().wrapping_add(1));
        self.transport.cancel();
        self.busy.set(false);
        self.queue.borrow_mut().clear();
        self.changed.set(None);
    }

    fn start_next(&self) -> Result<(), String> {
        if !self.available.get() || self.busy.get() {
            return Ok(());
        }
        let Some(action) = self.queue.borrow_mut().pop_front() else {
            return Ok(());
        };
        self.busy.set(true);
        if self
            .transport
            .sender
            .send(Request {
                generation: self.epoch.get(),
                action,
            })
            .is_err()
        {
            self.available.set(false);
            self.busy.set(false);
            self.queue.borrow_mut().clear();
            let error = "The native service connection closed. Restart To Do to retry.".to_owned();
            self.runtime_error.replace(Some(error.clone()));
            return Err(error);
        }
        Ok(())
    }

    fn dispatch_next(&self, vm: &AppViewModel) {
        if let Err(error) = self.start_next() {
            self.unavailable(vm, &error);
            vm.report_error(error);
        }
    }

    fn unavailable(&self, vm: &AppViewModel, error: &str) {
        vm.finish_refresh();
        vm.set_cloud_state(false, "Local account");
        vm.set_cloud_capabilities(false, vm.cloud_configured(), false);
        vm.set_cloud_hint(error);
    }

    fn poll(&self, vm: &Rc<AppViewModel>, window: &Ref<Window>) {
        for _ in 0..32 {
            let Ok(event) = self.transport.events.try_recv() else {
                break;
            };
            match event {
                Event::Available(result) => {
                    self.checked.set(true);
                    match result {
                        Ok(()) => {
                            self.available.set(true);
                            vm.set_cloud_capabilities(true, false, false);
                            self.dispatch_next(vm);
                        }
                        Err(error) => {
                            self.available.set(false);
                            self.runtime_error.replace(Some(error.clone()));
                            self.queue.borrow_mut().clear();
                            vm.finish_refresh();
                            vm.set_cloud_capabilities(false, false, false);
                            vm.set_cloud_hint(&error);
                        }
                    }
                }
                Event::Callback {
                    generation,
                    message,
                    reply,
                } => {
                    if generation == self.epoch.get() {
                        callbacks::respond(vm, window, message, reply);
                    } else {
                        let _ = reply.send(callbacks::rejected(&message));
                    }
                }
                Event::Complete {
                    generation,
                    action,
                    response,
                } => {
                    if generation != self.epoch.get() {
                        continue;
                    }
                    self.busy.set(false);
                    let automatic = action["automatic"].as_bool().unwrap_or(false);
                    if response.is_err() {
                        vm.set_cloud_state(false, "Local account");
                        vm.set_cloud_capabilities(true, vm.cloud_configured(), false);
                    }
                    let outcome = response.and_then(|result| {
                        if result["connected"].as_bool() == Some(false) && vm.cloud_connected() {
                            vm.set_cloud_state(false, "Local account");
                            vm.set_cloud_capabilities(true, vm.cloud_configured(), false);
                        }
                        if result["needsInteraction"].as_bool().unwrap_or(false) {
                            vm.set_cloud_state(
                                result["connected"].as_bool().unwrap_or(false),
                                "Sign-in required · Reconnect Microsoft",
                            );
                        }
                        if let Some(error) = result["error"].as_str() {
                            self.paused.set(result["review"].as_bool().unwrap_or(false));
                            return Err(error.to_owned());
                        }
                        Ok(result)
                    });
                    match outcome {
                        Ok(_) => {
                            self.failures.set(0);
                            self.next_sync.set(Instant::now() + Duration::from_secs(5));
                            self.changed.set(None);
                            self.document.replace(vm.export_json().unwrap_or_default());
                        }
                        Err(error) => {
                            let failures = self.failures.get().saturating_add(1).min(6);
                            self.failures.set(failures);
                            self.next_sync.set(
                                Instant::now()
                                    + Duration::from_secs((5 * 2u64.pow(failures)).min(60)),
                            );
                            if !automatic || self.paused.get() || failures == 1 {
                                vm.report_error(error);
                            }
                        }
                    }
                    if action["kind"] == "syncMicrosoft" {
                        vm.finish_refresh();
                    }
                    self.dispatch_next(vm);
                }
            }
        }
        if self.revision.replace(vm.revision()) != vm.revision() {
            if let Ok(document) = vm.export_json() {
                if *self.document.borrow() != document {
                    self.document.replace(document);
                    self.changed
                        .set(Some(Instant::now() + Duration::from_millis(900)));
                    self.paused.set(false);
                }
            }
        }
        let active = window.is_active()
            || (vm.background_supported() && vm.state().settings.keep_running_in_background);
        if !self.demo
            && active
            && self.available.get()
            && !self.busy.get()
            && !self.paused.get()
            && vm.cloud_connected()
            && vm.state().settings.auto_sync
        {
            let now = Instant::now();
            if self.changed.get().is_some_and(|time| time <= now) || self.next_sync.get() <= now {
                self.changed.set(None);
                if let Err(error) = self.submit(vm, json!({"kind":"syncMicrosoft", "automatic":true, "today":vm.today().to_string()})) {
                    vm.report_error(error);
                }
            }
        }
    }
}

use super::{platform, registration, Registration};
use chrono::Local;
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::Duration,
};
use todo_domain::AppState;

#[derive(Clone, Debug)]
pub struct SchedulerStatus {
    pub available: bool,
    pub message: String,
    pub error: bool,
}

#[derive(Default)]
struct Pending {
    json: Option<String>,
    stopped: bool,
}
struct Shared {
    pending: Mutex<Pending>,
    changed: Condvar,
    result: Mutex<Option<SchedulerStatus>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

/// Latest-value mailbox: rapid saves replace pending work and OS registration
/// runs on one background thread. finish() flushes the final save before exit.
#[derive(Clone)]
pub struct Scheduler(Arc<Shared>);

impl Scheduler {
    pub fn start(directory: PathBuf, executable: PathBuf) -> Self {
        let directory = directory.canonicalize().unwrap_or(directory);
        let executable = executable.canonicalize().unwrap_or(executable);
        let mut previous: Option<Option<Registration>> = None;
        let mut available = None;
        Self::with_backend(move |json| {
            let checked = available.get_or_insert_with(platform::probe);
            let supported = checked.is_ok();
            let result = (|| {
                let state = AppState::from_json(json).map_err(|error| error.to_string())?;
                let plan = registration(&state, &directory, &executable, Local::now())?;
                if state.settings.background_reminders {
                    checked.as_ref().map_err(Clone::clone)?;
                }
                if previous.as_ref() != Some(&plan) {
                    platform::reconcile(&directory, plan.as_ref())?;
                    previous = Some(plan.clone());
                }
                let message = match plan {
                    Some(plan) => format!("Next background reminder: {}. Delivery uses the active workspace while you are signed in to this computer.", plan.next.format("%b %-d, %H:%M")),
                    None if state.settings.background_reminders => "Background reminders are on. No task reminders are pending in the active workspace.".into(),
                    None if !supported => format!("Background reminders are unavailable: {}", checked.as_ref().unwrap_err()),
                    None => "Background reminders are off. Enable them to schedule the active workspace after this app closes.".into(),
                };
                Ok::<_, String>(message)
            })();
            match result {
                Ok(message) => SchedulerStatus {
                    available: supported,
                    message,
                    error: false,
                },
                Err(error) => {
                    let message = format!("Could not schedule background reminders: {error}");
                    // Re-probe on the next edit: a session service can recover without a restart.
                    available = None;
                    SchedulerStatus {
                        available: supported,
                        message,
                        error: true,
                    }
                }
            }
        })
    }

    fn with_backend(mut backend: impl FnMut(&str) -> SchedulerStatus + Send + 'static) -> Self {
        let shared = Arc::new(Shared {
            pending: Mutex::new(Pending::default()),
            changed: Condvar::new(),
            result: Mutex::new(None),
            worker: Mutex::new(None),
        });
        let state = shared.clone();
        let worker = std::thread::spawn(move || loop {
            let request = {
                let mut pending = state
                    .pending
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                while pending.json.is_none() && !pending.stopped {
                    pending = state
                        .changed
                        .wait(pending)
                        .unwrap_or_else(|error| error.into_inner());
                }
                if pending.json.is_none() && pending.stopped {
                    break;
                }
                while !pending.stopped {
                    let (next, timeout) = state
                        .changed
                        .wait_timeout(pending, Duration::from_millis(350))
                        .unwrap_or_else(|error| error.into_inner());
                    pending = next;
                    if timeout.timed_out() {
                        break;
                    }
                }
                pending.json.take()
            };
            if let Some(json) = request {
                let status = backend(&json);
                *state
                    .result
                    .lock()
                    .unwrap_or_else(|error| error.into_inner()) = Some(status);
            }
        });
        *shared.worker.lock().unwrap() = Some(worker);
        Self(shared)
    }

    pub fn submit(&self, json: &str) {
        let mut pending = self
            .0
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if pending.stopped {
            return;
        }
        pending.json = Some(json.to_owned());
        self.0.changed.notify_one();
    }

    pub fn take_status(&self) -> Option<SchedulerStatus> {
        self.0
            .result
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
    }

    pub fn finish(&self) {
        {
            let mut pending = self
                .0
                .pending
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            pending.stopped = true;
            self.0.changed.notify_one();
        }
        let worker = self
            .0
            .worker
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_burst_is_coalesced_and_shutdown_flushes_the_latest_document_once() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let target = received.clone();
        let scheduler = Scheduler::with_backend(move |json| {
            target.lock().unwrap().push(json.to_owned());
            SchedulerStatus {
                available: true,
                message: json.into(),
                error: false,
            }
        });
        for n in 0..100 {
            scheduler.submit(&n.to_string());
        }
        scheduler.finish();
        assert_eq!(*received.lock().unwrap(), ["99"]);
        assert_eq!(scheduler.take_status().unwrap().message, "99");
        assert!(scheduler.take_status().is_none());
        scheduler.submit("after shutdown");
        scheduler.finish();
        assert_eq!(*received.lock().unwrap(), ["99"]);
    }
}

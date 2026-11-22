use std::process::{Child, Command};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

pub type ProcessSlot = Arc<Mutex<Option<Child>>>;

/// A cancellation epoch is checked while holding the same slot that cancel()
/// locks. A process cannot be launched immediately after cancellation misses it.
pub fn spawn(
    command: &mut Command,
    slot: &ProcessSlot,
    epoch: &AtomicU64,
    generation: u64,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut active = slot.lock().map_err(|_| "The native process lock failed")?;
    if epoch.load(Ordering::Acquire) != generation {
        return Err("The native operation was cancelled.".into());
    }
    *active = Some(command.spawn().map_err(|error| error.to_string())?);
    Ok(())
}

pub fn kill(slot: &ProcessSlot) {
    if let Ok(mut child) = slot.lock() {
        if let Some(process) = child.as_mut() {
            let _ = process.kill();
        }
    }
}

/// Reaping is performed only on the service actor, never the UI dispatcher.
pub fn reap(slot: &ProcessSlot) {
    let child = slot.lock().ok().and_then(|mut slot| slot.take());
    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }
}

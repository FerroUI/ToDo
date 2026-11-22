use super::{
    process::{self, ProcessSlot},
    pst_limits,
};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{atomic::AtomicU64, mpsc};
use std::time::Duration;
use todo_domain::AppState;

const MAX_REPORT: u64 = 264 * 1024 * 1024;

/// A malformed third-party PST can request large allocations in its decoder.
/// Isolate it from both the UI and long-lived Microsoft token process.
pub fn prepare(
    action: &Value,
    slot: &ProcessSlot,
    epoch: &AtomicU64,
    generation: u64,
) -> Result<Value, String> {
    if action["kind"] != "importPst" {
        return Ok(action.clone());
    }
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg("--prepare-pst")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    process::spawn(&mut command, slot, epoch, generation)
        .map_err(|error| format!("Could not start the isolated Outlook parser: {error}"))?;
    let started = (|| {
        let mut active = slot
            .lock()
            .map_err(|_| "The Outlook parser process lock failed")?;
        let process = active.as_mut().ok_or("The Outlook parser was cancelled")?;
        let limit = pst_limits::attach(process)?;
        let input = process
            .stdin
            .take()
            .ok_or("The Outlook parser input is unavailable")?;
        let output = process
            .stdout
            .take()
            .ok_or("The Outlook parser output is unavailable")?;
        Ok::<_, String>((input, output, limit, process.id()))
    })();
    let (mut input, output, _limit, pid) = match started {
        Ok(value) => value,
        Err(error) => {
            process::reap(slot);
            return Err(error);
        }
    };
    let (done, wait) = mpsc::channel();
    let timed = slot.clone();
    std::thread::spawn(move || {
        if wait.recv_timeout(Duration::from_secs(60)).is_err() {
            if let Ok(mut slot) = timed.lock() {
                if let Some(child) = slot.as_mut().filter(|child| child.id() == pid) {
                    let _ = child.kill();
                }
            }
        }
    });
    let operation = (|| {
        serde_json::to_writer(&mut input, action).map_err(|e| e.to_string())?;
        input.flush().map_err(|e| e.to_string())?;
        drop(input);
        let mut bytes = Vec::new();
        output
            .take(MAX_REPORT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_REPORT {
            return Err("The Outlook import exceeds the supported archive size.".into());
        }
        let result: Value = serde_json::from_slice(&bytes).map_err(|_| "The Outlook parser stopped before producing a valid result. Export a smaller task-only PST and retry.".to_owned())?;
        if let Some(error) = result["error"].as_str() {
            return Err(error.to_owned());
        }
        Ok(
            json!({"kind":"importPstReport", "expected":action["expected"], "report":result["report"]}),
        )
    })();
    process::reap(slot);
    let _ = done.send(());
    operation
}

pub fn child_main() -> i32 {
    // This is the only process mode that enables allocation accounting. The
    // source parser's unchecked Vec counts cannot allocate beyond this cap.
    if crate::ALLOCATOR
        .enable(pst_limits::MEMORY_LIMIT as usize)
        .is_err()
    {
        return 2;
    }
    let result = (|| -> Result<Value, String> {
        let mut input = Vec::new();
        std::io::stdin()
            .take(33 * 1024 * 1024 + 1)
            .read_to_end(&mut input)
            .map_err(|e| e.to_string())?;
        if input.len() > 33 * 1024 * 1024 {
            return Err("The Outlook import document is too large.".into());
        }
        // On Windows the parent attaches the job before sending input. Reading
        // the bounded request first makes that synchronization explicit.
        pst_limits::in_child()?;
        let action: Value =
            serde_json::from_slice(&input).map_err(|_| "The Outlook parser request is invalid")?;
        parse(&action)
    })();
    let output = match result {
        Ok(report) => json!({"report":report}),
        Err(error) => json!({"error":error}),
    };
    let mut stdout = std::io::stdout().lock();
    if serde_json::to_writer(&mut stdout, &output).is_err() || stdout.flush().is_err() {
        1
    } else {
        0
    }
}

fn parse(action: &Value) -> Result<Value, String> {
    let path = action["path"]
        .as_str()
        .ok_or("No Outlook file was selected")?;
    let expected = action["expected"]
        .as_str()
        .ok_or("The import has no document revision")?;
    let state = AppState::from_json(expected).map_err(|e| e.to_string())?;
    let today = action["today"]
        .as_str()
        .ok_or("The import has no current date")?
        .parse()
        .map_err(|_| "The import date is invalid")?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > todo_interop::MAX_PST_BYTES as u64 {
        return Err("The Outlook PST must be 256 MiB or smaller.".into());
    }
    let mut bytes = Vec::new();
    file.take(todo_interop::MAX_PST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let report = todo_interop::import_pst_owned(bytes, &state, today).map_err(|e| e.to_string())?;
    serde_json::to_value(report).map_err(|e| e.to_string())
}

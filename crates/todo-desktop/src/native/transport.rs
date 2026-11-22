use super::process::{self, ProcessSlot};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Arc, Mutex,
};

include!(concat!(env!("OUT_DIR"), "/native_assets.rs"));
const MAX_FRAME: usize = 272 * 1024 * 1024;

pub struct Request {
    pub generation: u64,
    pub action: Value,
}

pub enum Event {
    Available(Result<(), String>),
    Callback {
        generation: u64,
        message: Value,
        reply: mpsc::Sender<Value>,
    },
    Complete {
        generation: u64,
        action: Value,
        response: Result<Value, String>,
    },
}

pub struct Transport {
    pub sender: mpsc::Sender<Request>,
    pub events: mpsc::Receiver<Event>,
    child: ProcessSlot,
    parser: ProcessSlot,
    epoch: Arc<AtomicU64>,
}

impl Transport {
    pub fn start(directory: PathBuf, demo: bool, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (sender, commands) = mpsc::channel::<Request>();
        let (events_sender, events) = mpsc::channel();
        let child = Arc::new(Mutex::new(None));
        let parser = Arc::new(Mutex::new(None));
        let epoch = Arc::new(AtomicU64::new(0));
        let child_thread = child.clone();
        let parser_thread = parser.clone();
        let thread_epoch = epoch.clone();
        std::thread::spawn(move || {
            let binary = std::env::var_os("TODO_NODE_BINARY").unwrap_or_else(|| "node".into());
            let setup = prepare_runtime(&directory).and_then(|script| {
                let version = Command::new(&binary).arg("--version").env_remove("NODE_OPTIONS").env_remove("NODE_PATH").output()
                    .map_err(|_| "Native services require Node.js 22 or newer. Install Node.js or set TODO_NODE_BINARY to its executable.".to_owned())?;
                let major = String::from_utf8_lossy(&version.stdout).trim().trim_start_matches('v').split('.').next().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
                if !version.status.success() || major < 22 { return Err("Native services require Node.js 22 or newer. Update Node.js or set TODO_NODE_BINARY.".into()); }
                Ok(script)
            });
            let _ = events_sender.send(Event::Available(
                setup.as_ref().map(|_| ()).map_err(Clone::clone),
            ));
            wake();
            let Ok(script) = setup else {
                return;
            };
            let mut pipes = None;
            let mut pipe_generation = 0;
            for request in commands {
                let outcome = (|| -> Result<Value, String> {
                    if thread_epoch.load(Ordering::Acquire) != request.generation {
                        return Err("The native operation was cancelled.".into());
                    }
                    if pipe_generation != request.generation {
                        pipes = None;
                        process::reap(&child_thread);
                        pipe_generation = request.generation;
                    }
                    let action = match super::pst::prepare(
                        &request.action,
                        &parser_thread,
                        &thread_epoch,
                        request.generation,
                    ) {
                        Ok(action) => action,
                        Err(error) => {
                            return Ok(
                                json!({"type":"result", "id":request.generation, "error":error}),
                            )
                        }
                    };
                    if pipes.is_none() {
                        let mut command = Command::new(&binary);
                        command
                            .arg(&script)
                            .env_remove("NODE_OPTIONS")
                            .env_remove("NODE_PATH")
                            .stdin(Stdio::piped())
                            .stdout(Stdio::piped())
                            .stderr(Stdio::null());
                        process::spawn(
                            &mut command,
                            &child_thread,
                            &thread_epoch,
                            request.generation,
                        )
                        .map_err(|error| format!("Could not start native services: {error}"))?;
                        let (mut input, mut output) = {
                            let mut active = child_thread
                                .lock()
                                .map_err(|_| "Native service process lock failed")?;
                            let process =
                                active.as_mut().ok_or("Native services were cancelled")?;
                            let input = process
                                .stdin
                                .take()
                                .ok_or("Native service input is unavailable")?;
                            let output = process
                                .stdout
                                .take()
                                .ok_or("Native service output is unavailable")?;
                            (input, output)
                        };
                        write_frame(
                            &mut input,
                            &json!({"type":"initialize", "directory":directory,"demo":demo}),
                        )?;
                        let ready = read_frame(&mut output)?;
                        if ready["type"] != "ready" {
                            return Err("Native services did not initialize correctly.".into());
                        }
                        pipes = Some((input, output));
                    }
                    if thread_epoch.load(Ordering::Acquire) != request.generation {
                        return Err("The native operation was cancelled.".into());
                    }
                    let (input, output) = pipes.as_mut().unwrap();
                    write_frame(
                        input,
                        &json!({"type":"command","id":request.generation,"action":action}),
                    )?;
                    loop {
                        let message = read_frame(output)?;
                        match message["type"].as_str() {
                            Some("callback") => {
                                let (reply, result) = mpsc::channel();
                                events_sender
                                    .send(Event::Callback {
                                        generation: request.generation,
                                        message,
                                        reply,
                                    })
                                    .map_err(|_| "The application window closed")?;
                                wake();
                                write_frame(
                                    input,
                                    &result.recv().map_err(|_| "The application window closed")?,
                                )?;
                            }
                            Some("result")
                                if message["id"].as_u64() == Some(request.generation) =>
                            {
                                return Ok(message)
                            }
                            _ => {
                                return Err(
                                    "Native services returned an unexpected response.".into()
                                )
                            }
                        }
                    }
                })();
                if outcome.is_err() {
                    pipes = None;
                    process::reap(&child_thread);
                }
                if events_sender
                    .send(Event::Complete {
                        generation: request.generation,
                        action: request.action,
                        response: outcome,
                    })
                    .is_err()
                {
                    break;
                }
                wake();
            }
            process::reap(&child_thread);
            process::reap(&parser_thread);
        });
        Self {
            sender,
            events,
            child,
            parser,
            epoch,
        }
    }

    pub fn cancel(&self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
        process::kill(&self.child);
        process::kill(&self.parser);
    }
}

impl Drop for Transport {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn prepare_runtime(directory: &Path) -> Result<PathBuf, String> {
    // Version the extracted code by content, allowing running releases to retain
    // their own imports while another application binary is installed.
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for (name, bytes) in ASSETS {
        name.hash(&mut hash);
        bytes.hash(&mut hash);
    }
    let root = directory
        .join("runtime")
        .join(format!("{:016x}", hash.finish()));
    for (name, bytes) in ASSETS {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        if std::fs::read(&path).ok().as_deref() != Some(bytes) {
            std::fs::write(path, bytes).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(
        root.join("package.json"),
        b"{\"type\":\"module\",\"private\":true}",
    )
    .map_err(|e| e.to_string())?;
    Ok(root.join("native/worker.mjs"))
}

fn write_frame(writer: &mut impl Write, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err("The native service message exceeds its size limit.".into());
    }
    writer
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .and_then(|_| writer.write_all(&bytes))
        .and_then(|_| writer.flush())
        .map_err(|_| "The native service connection closed.".into())
}

fn read_frame(reader: &mut impl Read) -> Result<Value, String> {
    let mut length = [0; 4];
    reader.read_exact(&mut length).map_err(|_| {
        "The native service stopped. Reconnect Microsoft if needed and retry the operation."
            .to_owned()
    })?;
    let length = u32::from_le_bytes(length) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err("The native service returned an invalid message size.".into());
    }
    let mut bytes = vec![0; length];
    reader
        .read_exact(&mut bytes)
        .map_err(|_| "The native service response was interrupted.".to_owned())?;
    serde_json::from_slice(&bytes)
        .map_err(|_| "The native service returned an invalid response.".into())
}

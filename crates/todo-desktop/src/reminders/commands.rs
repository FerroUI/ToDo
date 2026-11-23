use std::{
    io::Read,
    process::{Command, Output, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

pub(crate) fn output(command: &mut Command) -> Result<Output, String> {
    bounded_output(command, Duration::from_secs(15))
}

const OUTPUT_LIMIT: usize = 1024 * 1024;

fn drain(mut pipe: impl Read + Send + 'static) -> Receiver<Result<Vec<u8>, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| {
            let mut bytes = Vec::new();
            let mut buffer = [0; 16384];
            loop {
                let count = pipe.read(&mut buffer).map_err(|error| error.to_string())?;
                if count == 0 {
                    break;
                }
                let retain = count.min(OUTPUT_LIMIT.saturating_sub(bytes.len()));
                bytes.extend_from_slice(&buffer[..retain]);
            }
            Ok(bytes)
        })();
        let _ = sender.send(result);
    });
    receiver
}

fn bounded_output(command: &mut Command, timeout: Duration) -> Result<Output, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    // launchctl can print a complete GUI domain. Drain both pipes while waiting,
    // retaining only bounded diagnostics so a full pipe cannot stall the helper.
    let stdout = drain(child.stdout.take().expect("piped stdout"));
    let stderr = drain(child.stderr.take().expect("piped stderr"));
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let collect = |receiver: Receiver<Result<Vec<u8>, String>>| {
                    receiver.recv_timeout(Duration::from_secs(1)).map_err(|_| {
                        "The system reminder service did not close its output".to_owned()
                    })?
                };
                return Ok(Output {
                    status,
                    stdout: collect(stdout)?,
                    stderr: collect(stderr)?,
                });
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("The system reminder service did not respond within 15 seconds".into());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_output_child() {
        let Some(mode) = std::env::var_os("TODO_REMINDER_COMMAND_TEST") else {
            return;
        };
        if mode == "timeout" {
            std::thread::sleep(Duration::from_secs(10));
            return;
        }
        use std::io::Write;
        let bytes = vec![b'x'; OUTPUT_LIMIT * 2];
        std::io::stdout().write_all(&bytes).unwrap();
        std::io::stderr().write_all(&bytes).unwrap();
    }

    fn child(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "reminders::commands::tests::command_output_child",
                "--nocapture",
            ])
            .env("TODO_REMINDER_COMMAND_TEST", mode);
        command
    }

    #[test]
    fn large_stdout_and_stderr_are_drained_without_deadlock_or_unbounded_capture() {
        let output = output(&mut child("large")).unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), OUTPUT_LIMIT);
        assert_eq!(output.stderr.len(), OUTPUT_LIMIT);
    }

    #[test]
    fn an_unresponsive_helper_is_killed_and_reaped_within_the_deadline() {
        let start = Instant::now();
        assert!(bounded_output(&mut child("timeout"), Duration::from_millis(100)).is_err());
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}

pub(crate) fn run(command: &mut Command) -> Result<(), String> {
    let result = output(command)?;
    if result.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&result.stderr);
        let error: String = error.trim().chars().take(1200).collect();
        Err(if error.is_empty() {
            format!("The system reminder service exited with {}", result.status)
        } else {
            error
        })
    }
}

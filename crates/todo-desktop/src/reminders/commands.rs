use std::{
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

pub(crate) fn output(command: &mut Command) -> Result<Output, String> {
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
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().map_err(|error| error.to_string()),
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

use super::{commands, identity, templates, Registration};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn probe() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return commands::run(Command::new("systemctl").args(["--user", "show-environment"]));
    #[cfg(target_os = "macos")]
    return commands::run(
        Command::new("/bin/launchctl")
            .args(["print", &format!("gui/{}", unsafe { libc::getuid() })]),
    );
    #[cfg(target_os="windows")]
    return commands::run(Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; if ((Get-Service Schedule).Status -ne 'Running') { throw 'Task Scheduler is unavailable' }"]));
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err("Background reminders are unavailable on this platform".into())
}

pub(crate) fn write_atomic(path: &Path, value: &str) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().ok_or("Invalid reminder service path")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    file.write_all(value.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| error.to_string())?;
    drop(file);
    std::fs::rename(temporary, path).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    std::fs::File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or("The current user home directory is unavailable".into())
}

pub(crate) fn reconcile(
    directory: &Path,
    registration: Option<&Registration>,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return linux(directory, registration);
    #[cfg(target_os = "macos")]
    return macos(directory, registration);
    #[cfg(target_os = "windows")]
    return windows(directory, registration);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (directory, registration);
        Err("Background reminders are unavailable on this platform".into())
    }
}

#[cfg(target_os = "linux")]
fn linux(directory: &Path, registration: Option<&Registration>) -> Result<(), String> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(path) => PathBuf::from(path),
        None => home()?.join(".config"),
    }
    .join("systemd/user");
    let id = identity(directory);
    let service = base.join(format!("{id}.service"));
    let timer = base.join(format!("{id}.timer"));
    let timer_name = format!("{id}.timer");
    let Some(plan) = registration else {
        if !timer.exists() && !service.exists() {
            return Ok(());
        }
        let stopped = commands::run(Command::new("systemctl").args([
            "--user",
            "disable",
            "--now",
            &timer_name,
        ]));
        // Remove the persistent timestamp too, so a later opt-in cannot replay an old timer.
        let _ = commands::run(Command::new("systemctl").args([
            "--user",
            "clean",
            "--what=state",
            &timer_name,
        ]));
        for path in [&timer, &service] {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        let reloaded = commands::run(Command::new("systemctl").args(["--user", "daemon-reload"]));
        stopped.and(reloaded)?;
        return Ok(());
    };
    write_atomic(&service, &templates::linux_service(plan)?)?;
    write_atomic(&timer, &templates::linux_timer(plan))?;
    commands::run(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
    commands::run(Command::new("systemctl").args(["--user", "enable", &timer_name]))?;
    commands::run(Command::new("systemctl").args(["--user", "restart", &timer_name]))
}

#[cfg(target_os = "macos")]
fn macos(directory: &Path, registration: Option<&Registration>) -> Result<(), String> {
    let id = identity(directory);
    let agent = home()?
        .join("Library/LaunchAgents")
        .join(format!("{id}.plist"));
    let domain = format!("gui/{}", unsafe { libc::getuid() });
    let target = format!("{domain}/{id}");
    let loaded = commands::output(Command::new("/bin/launchctl").args(["print", &target]))?
        .status
        .success();
    if let Some(plan) = registration {
        let existing = match std::fs::read_to_string(&agent) {
            Ok(value) => Some(value),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        let Some(installation) = templates::macos_installation(plan, loaded, existing.as_deref())?
        else {
            return Ok(());
        };
        if loaded {
            commands::run(Command::new("/bin/launchctl").args(["bootout", &target]))?;
        }
        // launchd retains the loaded configuration. Its login file keeps RunAtLoad,
        // while this one-time bootstrap suppresses another immediate catch-up loop.
        let bootstrap = directory.join("background-reminder-bootstrap.plist");
        write_atomic(&bootstrap, &installation.bootstrap)?;
        let result = commands::run(
            Command::new("/bin/launchctl")
                .arg("bootstrap")
                .arg(&domain)
                .arg(&bootstrap),
        );
        let _ = std::fs::remove_file(bootstrap);
        result?;
        // Publish the desired login file only after launchd accepts it. Otherwise a
        // failed replacement could leave the old job loaded beside a new file and
        // make the next idempotence check incorrectly skip the necessary retry.
        write_atomic(&agent, &installation.persistent)
    } else {
        if loaded {
            commands::run(Command::new("/bin/launchctl").args(["bootout", &target]))?;
        }
        match std::fs::remove_file(&agent) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(target_os = "windows")]
fn windows(directory: &Path, registration: Option<&Registration>) -> Result<(), String> {
    let id = identity(directory);
    let task = directory.join("background-reminder-task.xml");
    if registration.is_none() {
        // No marker means this application has never registered a task for this directory.
        if !task.exists() {
            return Ok(());
        }
        let query = commands::output(Command::new("schtasks.exe").args(["/Query", "/TN", &id]))?;
        if query.status.success() {
            commands::run(Command::new("schtasks.exe").args(["/Delete", "/TN", &id, "/F"]))?;
        }
        std::fs::remove_file(task).map_err(|error| error.to_string())?;
        return Ok(());
    }
    let sid = commands::output(Command::new("powershell.exe").args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value",
    ]))?;
    if !sid.status.success() {
        return Err("Windows could not identify the current interactive user".into());
    }
    let sid =
        String::from_utf8(sid.stdout).map_err(|_| "Windows returned an invalid user identity")?;
    write_atomic(
        &task,
        &templates::windows_task(registration.unwrap(), sid.trim())?,
    )?;
    commands::run(
        Command::new("schtasks.exe")
            .args(["/Create", "/TN", &id, "/XML"])
            .arg(&task)
            .arg("/F"),
    )
}

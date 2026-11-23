//! Arguments are serialized for each service format, never interpolated into a shell.
use super::Registration;
use chrono::{Datelike, Timelike};

fn path(value: &std::path::Path) -> Result<&str, String> {
    let value = value
        .to_str()
        .ok_or("Reminder paths must use valid Unicode")?;
    if value.chars().any(char::is_control) {
        return Err("Reminder paths cannot contain control characters".into());
    }
    Ok(value)
}

pub fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn unit_argument(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
            .replace('$', "$$")
    )
}

pub fn linux_service(plan: &Registration) -> Result<String, String> {
    Ok(format!("[Unit]\nDescription=To Do background reminders\n\n[Service]\nType=oneshot\nExecStart={} --deliver-reminders --data-dir {}\nNoNewPrivileges=true\nTimeoutStartSec=120\n",
        unit_argument(path(&plan.executable)?), unit_argument(path(&plan.directory)?)))
}

pub fn linux_timer(plan: &Registration) -> String {
    format!("[Unit]\nDescription=Next To Do reminder\n\n[Timer]\nOnCalendar={}\nPersistent=true\nAccuracySec=1s\nOnClockChange=true\nOnTimezoneChange=true\nUnit={}.service\n\n[Install]\nWantedBy=timers.target\n", plan.next.format("%Y-%m-%d %H:%M:%S"), plan.id)
}

pub fn macos_agent(plan: &Registration) -> Result<String, String> {
    macos_agent_with_catch_up(plan, true)
}

pub struct MacosInstallation {
    pub persistent: String,
    pub bootstrap: String,
}

/// Login catch-up runs once when a previously unloaded job is installed. Replacing
/// a loaded job must not immediately run the helper again (including failed deliveries).
pub fn macos_installation(
    plan: &Registration,
    loaded: bool,
    existing: Option<&str>,
) -> Result<Option<MacosInstallation>, String> {
    let persistent = macos_agent(plan)?;
    if loaded && existing == Some(persistent.as_str()) {
        return Ok(None);
    }
    let bootstrap = macos_agent_with_catch_up(plan, !loaded)?;
    Ok(Some(MacosInstallation {
        persistent,
        bootstrap,
    }))
}

fn macos_agent_with_catch_up(plan: &Registration, catch_up: bool) -> Result<String, String> {
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>{}</string>
<key>ProgramArguments</key><array><string>{}</string><string>--deliver-reminders</string><string>--data-dir</string><string>{}</string></array>
<key>RunAtLoad</key><{}/>
<key>ProcessType</key><string>Background</string>
<key>StartCalendarInterval</key><dict><key>Month</key><integer>{}</integer><key>Day</key><integer>{}</integer><key>Hour</key><integer>{}</integer><key>Minute</key><integer>{}</integer></dict>
</dict></plist>
"#,
        xml(&plan.id),
        xml(path(&plan.executable)?),
        xml(path(&plan.directory)?),
        if catch_up { "true" } else { "false" },
        plan.next.month(),
        plan.next.day(),
        plan.next.hour(),
        plan.next.minute()
    ))
}

/// Windows CommandLineToArgvW quoting, distinct from XML escaping.
pub fn windows_argument(value: &str) -> String {
    let mut out = String::from("\"");
    let mut slashes = 0;
    for ch in value.chars() {
        if ch == '\\' {
            slashes += 1;
            continue;
        }
        if ch == '"' {
            out.extend(std::iter::repeat_n('\\', slashes * 2 + 1));
        } else {
            out.extend(std::iter::repeat_n('\\', slashes));
        }
        slashes = 0;
        out.push(ch);
    }
    out.extend(std::iter::repeat_n('\\', slashes * 2));
    out.push('"');
    out
}

pub fn windows_task(plan: &Registration, sid: &str) -> Result<String, String> {
    if !sid.starts_with("S-1-")
        || !sid
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'S' || b == b'-')
    {
        return Err("Windows did not return a valid current user identity".into());
    }
    let arguments = format!(
        "--deliver-reminders --data-dir {}",
        windows_argument(path(&plan.directory)?)
    );
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
<RegistrationInfo><Description>To Do reminders for the active local workspace</Description></RegistrationInfo>
<Triggers><TimeTrigger><StartBoundary>{}</StartBoundary><Enabled>true</Enabled></TimeTrigger><LogonTrigger><Enabled>true</Enabled><UserId>{}</UserId></LogonTrigger></Triggers>
<Principals><Principal id="CurrentUser"><UserId>{}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
<Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><StartWhenAvailable>true</StartWhenAvailable><AllowStartOnDemand>true</AllowStartOnDemand><Enabled>true</Enabled><ExecutionTimeLimit>PT2M</ExecutionTimeLimit></Settings>
<Actions Context="CurrentUser"><Exec><Command>{}</Command><Arguments>{}</Arguments></Exec></Actions>
</Task>
"#,
        plan.next.format("%Y-%m-%dT%H:%M:%S"),
        xml(sid),
        xml(sid),
        xml(path(&plan.executable)?),
        xml(&arguments)
    ))
}

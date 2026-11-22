use super::wake;
use ferroui_base::Ref;
use ferroui_controls::Window;
use std::{cell::Cell, rc::Rc, sync::mpsc};
use todo_state::AppViewModel;

pub fn show(
    vm: &Rc<AppViewModel>,
    window: &Ref<Window>,
    title: String,
    body: String,
    requested: bool,
) {
    let weak = Rc::downgrade(vm);
    let weak_window = window.downgrade();
    let (sender, result) = mpsc::channel();
    let token = Rc::new(Cell::new(0));
    let retained = token.clone();
    let (id, wake) = wake::register(move || {
        let Ok(outcome) = result.try_recv() else {
            return;
        };
        wake::unregister(retained.get());
        let Some(vm) = weak.upgrade() else {
            return;
        };
        match outcome {
            Ok(()) if requested => vm.set_status("A test reminder was requested. Allow To Do notifications in your system settings if it does not appear.".into()),
            Ok(()) => {},
            Err(error) => {
                vm.report_error(error);
                if let Some(window) = weak_window.upgrade() { if !window.is_visible() { window.show(); } }
            }
        }
    });
    token.set(id);
    std::thread::spawn(move || {
        let _ = sender.send(deliver(&title, &body));
        wake();
    });
}

#[cfg(target_os = "linux")]
fn deliver(title: &str, body: &str) -> Result<(), String> {
    zbus::block_on(async {
        let connection = zbus::Connection::session()
            .await
            .map_err(|e| e.to_string())?;
        let proxy = zbus::Proxy::new(
            &connection,
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
        )
        .await
        .map_err(|e| e.to_string())?;
        let hints = std::collections::HashMap::<String, zbus::OwnedVariant>::new();
        let actions = Vec::<String>::new();
        let body = body
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let _: u32 = proxy
            .call(
                "Notify",
                &("To Do", 0u32, "", title, body, actions, hints, -1i32),
            )
            .await
            .map_err(|e| format!("System notifications are unavailable: {e}"))?;
        Ok(())
    })
}

#[cfg(target_os = "macos")]
fn deliver(title: &str, body: &str) -> Result<(), String> {
    let result = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", "on run argv\ndisplay notification (item 2 of argv) with title (item 1 of argv)\nend run", title, body])
        .output().map_err(|_| "System notifications are unavailable")?;
    if result.status.success() {
        Ok(())
    } else {
        Err("macOS did not accept the reminder. Check notification permissions for the system scripting host.".into())
    }
}

#[cfg(target_os = "windows")]
fn deliver(title: &str, body: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // Constant script; task text is passed as environment data and inserted as
    // XML text nodes, never interpolated into PowerShell or markup.
    let script = r#"$ErrorActionPreference='Stop';
$key='HKCU:\Software\Classes\AppUserModelId\FerroUI.ToDo';
New-Item -Path $key -Force | Out-Null;
New-ItemProperty -Path $key -Name DisplayName -Value 'To Do' -PropertyType String -Force | Out-Null;
[Windows.UI.Notifications.ToastNotificationManager,Windows.UI.Notifications,ContentType=WindowsRuntime] | Out-Null;
[Windows.Data.Xml.Dom.XmlDocument,Windows.Data.Xml.Dom.XmlDocument,ContentType=WindowsRuntime] | Out-Null;
$xml=New-Object Windows.Data.Xml.Dom.XmlDocument;
$xml.LoadXml('<toast><visual><binding template="ToastGeneric"><text></text><text></text></binding></visual></toast>');
$nodes=$xml.GetElementsByTagName('text');
$nodes.Item(0).AppendChild($xml.CreateTextNode($env:TODO_NOTIFICATION_TITLE)) | Out-Null;
$nodes.Item(1).AppendChild($xml.CreateTextNode($env:TODO_NOTIFICATION_BODY)) | Out-Null;
$toast=[Windows.UI.Notifications.ToastNotification]::new($xml);
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('FerroUI.ToDo').Show($toast);"#;
    let result = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("TODO_NOTIFICATION_TITLE", title)
        .env("TODO_NOTIFICATION_BODY", body)
        .creation_flags(0x08000000)
        .output()
        .map_err(|_| "Windows notifications are unavailable")?;
    if result.status.success() {
        Ok(())
    } else {
        Err("Windows did not accept the reminder. Check notification permissions for To Do.".into())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn deliver(_: &str, _: &str) -> Result<(), String> {
    Err("This platform does not expose native task notifications.".into())
}

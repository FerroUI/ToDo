//! OS delivery shared by the foreground host and the scheduled headless helper.
#[cfg(target_os = "linux")]
pub fn deliver(title: &str, body: &str) -> Result<(), String> {
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
pub fn deliver(title: &str, body: &str) -> Result<(), String> {
    let result = crate::reminders::output(std::process::Command::new("/usr/bin/osascript").args([
        "-e",
        "on run argv\ndisplay notification (item 2 of argv) with title (item 1 of argv)\nend run",
        title,
        body,
    ]))
    .map_err(|error| format!("System notifications are unavailable: {error}"))?;
    if result.status.success() {
        Ok(())
    } else {
        Err("macOS did not accept the reminder. Check notification permissions for the system scripting host.".into())
    }
}

#[cfg(target_os = "windows")]
pub fn deliver(title: &str, body: &str) -> Result<(), String> {
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
    let result = crate::reminders::output(
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env("TODO_NOTIFICATION_TITLE", title)
            .env("TODO_NOTIFICATION_BODY", body)
            .creation_flags(0x08000000),
    )
    .map_err(|error| format!("Windows notifications are unavailable: {error}"))?;
    if result.status.success() {
        Ok(())
    } else {
        Err("Windows did not accept the reminder. Check notification permissions for To Do.".into())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn deliver(_: &str, _: &str) -> Result<(), String> {
    Err("This platform does not expose native task notifications.".into())
}

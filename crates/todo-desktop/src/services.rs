use crate::native::NativeServices;
use chrono::{Local, Timelike};
use ferroui_base::platform::storage::FilePickerSaveOptions;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::Window;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use todo_state::{AppViewModel, HostRequest, HostServices};
mod files;

pub fn install(vm: &Rc<AppViewModel>, window: &Ref<Window>, directory: PathBuf, demo: bool) {
    let native = NativeServices::install(vm, window, directory.clone(), demo);
    let weak_vm = Rc::downgrade(vm);
    let weak_window = window.downgrade();
    vm.set_services(HostServices {
        request: Rc::new(move |request| {
            let (Some(vm), Some(window)) = (weak_vm.upgrade(), weak_window.upgrade()) else {
                return;
            };
            let directory = directory.clone();
            let native = native.clone();
            Dispatcher::ui_thread().spawn_local(async move {
                if let Err(error) = handle(&vm, &window, directory, native, request).await {
                    vm.report_error(error);
                }
            });
        }),
    });
}

async fn handle(
    vm: &Rc<AppViewModel>,
    window: &Ref<Window>,
    directory: PathBuf,
    native: Rc<NativeServices>,
    request: HostRequest,
) -> Result<(), String> {
    let provider = window.storage_provider();
    match request {
        HostRequest::NavigationBack { .. } => {}
        HostRequest::Refresh { sync } => {
            todo_ui::commit_pending_edits(vm, window);
            let now = Local::now();
            vm.tick(now.date_naive(), (now.hour() * 60 + now.minute()) as u16);
            if sync {
                native.submit(
                    serde_json::json!({"kind":"syncMicrosoft","today":vm.today().to_string()}),
                )?;
            } else {
                vm.finish_refresh();
            }
        }
        HostRequest::Export { filename, contents } => {
            let mut options = FilePickerSaveOptions::new();
            options.set_suggested_file_name(Some(filename));
            options.set_default_extension(Some("json".into()));
            if let Some(file) = provider
                .save_file_picker_async(options)
                .await
                .map_err(|error| error.to_string())?
            {
                let mut stream = file
                    .open_write_async()
                    .await
                    .map_err(|error| error.to_string())?;
                stream
                    .write_all(contents.as_bytes())
                    .map_err(|error| error.to_string())?;
                stream.flush().map_err(|error| error.to_string())?;
            }
        }
        HostRequest::Import
        | HostRequest::ImportMicrosoft
        | HostRequest::ImportPst
        | HostRequest::ExportArchive
        | HostRequest::ExportMicrosoft
        | HostRequest::ExportPst
        | HostRequest::Attach { .. }
        | HostRequest::OpenAttachment { .. } => {
            files::handle(vm, window, &native, request).await?;
        }
        HostRequest::Print { html, .. } => {
            std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
            let path = directory.join("print-list.html");
            std::fs::write(&path, html).map_err(|error| error.to_string())?;
            let uri =
                url::Url::from_file_path(path).map_err(|_| "Cannot open the print preview")?;
            if !window.launcher().launch_uri_async(&uri).await {
                return Err("Could not open print preview".into());
            }
        }
        HostRequest::OpenUrl { url } => {
            let uri = url::Url::parse(&url).map_err(|error| error.to_string())?;
            if !["https", "http", "mailto"].contains(&uri.scheme()) {
                return Err("This link type is not supported".into());
            }
            if !window.launcher().launch_uri_async(&uri).await {
                return Err("Could not open this link".into());
            }
        }
        HostRequest::SetTheme { .. } => {}
        HostRequest::CompletionSound => crate::native::audio::play(&directory),
        HostRequest::Notify { title, body } => {
            vm.set_status(format!("{title}: {body}"));
            crate::native::notifications::show(vm, window, title, body, false);
        }
        HostRequest::RequestNotifications => {
            crate::native::notifications::show(
                vm,
                window,
                "To Do".into(),
                "Your task reminders can appear here while To Do is running.".into(),
                true,
            );
        }
        HostRequest::ConnectMicrosoft { .. }
        | HostRequest::ReconnectMicrosoft
        | HostRequest::SignOutMicrosoft
        | HostRequest::DisconnectMicrosoft
        | HostRequest::SyncMicrosoft
        | HostRequest::DeleteAttachment { .. }
        | HostRequest::EnableOutlookCategories
        | HostRequest::RefreshOutlookCategories
        | HostRequest::DisableOutlookCategories
        | HostRequest::EnablePresentationSync
        | HostRequest::DisablePresentationSync => {
            todo_ui::commit_pending_edits(vm, window);
            if matches!(request, HostRequest::DisconnectMicrosoft) {
                vm.finish_refresh();
                vm.set_cloud_state(false, "Local account");
                vm.set_cloud_capabilities(true, vm.cloud_configured(), false);
            }
            let mut action = serde_json::to_value(request).map_err(|error| error.to_string())?;
            action["today"] = serde_json::Value::String(vm.today().to_string());
            native.submit(action)?;
        }
        HostRequest::OpenMicrosoftToDo => {
            native.submit(serde_json::json!({"kind":"openMicrosoftToDo"}))?;
        }
    }
    Ok(())
}

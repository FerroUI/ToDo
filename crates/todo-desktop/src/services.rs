use chrono::{Local, Timelike};
use ferroui_base::Ref;
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions,
};
use ferroui_base::threading::Dispatcher;
use ferroui_controls::Window;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::rc::Rc;
use todo_state::{AppViewModel, HostRequest, HostServices};

pub fn install(vm: &Rc<AppViewModel>, window: &Ref<Window>, directory: PathBuf) {
    let weak_vm = Rc::downgrade(vm);
    let weak_window = window.downgrade();
    vm.set_services(HostServices {
        request: Rc::new(move |request| {
            let (Some(vm), Some(window)) = (weak_vm.upgrade(), weak_window.upgrade()) else {
                return;
            };
            let directory = directory.clone();
            Dispatcher::ui_thread().spawn_local(async move {
                if let Err(error) = handle(&vm, &window, directory, request).await {
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
    request: HostRequest,
) -> Result<(), String> {
    let provider = window.storage_provider();
    match request {
        HostRequest::NavigationBack { .. } => {}
        HostRequest::Refresh { sync } => {
            todo_ui::commit_pending_edits(vm, window);
            let now = Local::now();
            vm.tick(now.date_naive(), (now.hour() * 60 + now.minute()) as u16);
            vm.finish_refresh();
            if sync {
                return Err("Microsoft synchronization is available in the browser app.".into());
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
        HostRequest::Import => {
            let mut options = FilePickerOpenOptions::new();
            options.set_file_type_filter(Some(vec![
                FilePickerFileType::new(Some("To Do backup")).with_patterns(&["*.json"]),
            ]));
            let files = provider
                .open_file_picker_async(options)
                .await
                .map_err(|error| error.to_string())?;
            if let Some(file) = files.first() {
                let stream = file
                    .open_read_async()
                    .await
                    .map_err(|error| error.to_string())?;
                let mut json = String::new();
                stream
                    .take(25 * 1024 * 1024 + 1)
                    .read_to_string(&mut json)
                    .map_err(|error| error.to_string())?;
                if json.len() > 25 * 1024 * 1024 {
                    return Err("The backup must be 25 MB or smaller".into());
                }
                vm.import_json(&json)?;
            }
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
        HostRequest::SetTheme { .. } | HostRequest::CompletionSound => {}
        HostRequest::Notify { title, body } => vm.set_status(format!("{title}: {body}")),
        HostRequest::RequestNotifications => {
            vm.set_status("Reminders appear in To Do while the desktop app is open.".into())
        }
        HostRequest::Attach { .. }
        | HostRequest::OpenAttachment { .. }
        | HostRequest::DeleteAttachment { .. } => {
            return Err("File attachments are available in the browser app.".into());
        }
        HostRequest::ConnectMicrosoft { .. }
        | HostRequest::DisconnectMicrosoft
        | HostRequest::SyncMicrosoft => {
            return Err(
                "Microsoft account synchronization is available in the browser app.".into(),
            );
        }
    }
    Ok(())
}

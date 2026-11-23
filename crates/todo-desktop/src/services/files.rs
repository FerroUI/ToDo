use crate::native::NativeServices;
use ferroui_base::{
    platform::storage::{
        FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageItem,
    },
    Ref,
};
use ferroui_controls::Window;
use serde_json::json;
use std::rc::Rc;
use todo_state::{AppViewModel, HostRequest};

pub async fn handle(
    vm: &Rc<AppViewModel>,
    window: &Ref<Window>,
    native: &Rc<NativeServices>,
    request: HostRequest,
) -> Result<(), String> {
    let provider = window.storage_provider();
    match request {
        HostRequest::Import
        | HostRequest::ImportMicrosoft
        | HostRequest::ImportPst
        | HostRequest::Attach { .. } => {
            let (label, patterns) = match request {
                HostRequest::ImportPst => ("Outlook data file", vec!["*.pst"]),
                HostRequest::Attach { .. } => ("Task attachment", vec!["*"]),
                _ => ("To Do backup", vec!["*.json"]),
            };
            let mut options = FilePickerOpenOptions::new();
            options.set_file_type_filter(Some(vec![
                FilePickerFileType::new(Some(label)).with_patterns(&patterns)
            ]));
            let files = provider
                .open_file_picker_async(options)
                .await
                .map_err(|error| error.to_string())?;
            let Some(file) = files.first() else {
                return Ok(());
            };
            let path = (file.as_ref() as &dyn IStorageItem)
                .try_get_local_path()
                .ok_or("Choose a file stored on this computer.")?;
            todo_ui::commit_pending_edits(vm, window);
            let mut action = serde_json::to_value(&request).map_err(|error| error.to_string())?;
            action["path"] = json!(path);
            action["name"] = json!(file.name());
            action["expected"] = json!(vm.export_json()?);
            action["today"] = json!(vm.today().to_string());
            native.submit(vm, action)?;
        }
        HostRequest::ExportArchive
        | HostRequest::ExportMicrosoft
        | HostRequest::ExportPst
        | HostRequest::OpenAttachment { .. } => {
            let filename = match &request {
                HostRequest::ExportArchive => format!("todo-portable-backup-{}.json", vm.today()),
                HostRequest::ExportMicrosoft => format!("microsoft-todo-{}.json", vm.today()),
                HostRequest::ExportPst => format!("todo-outlook-tasks-{}.pst", vm.today()),
                HostRequest::OpenAttachment { key } => vm
                    .state()
                    .tasks
                    .iter()
                    .flat_map(|task| task.attachments.iter())
                    .find(|file| file.storage_key == *key)
                    .map(|file| file.name.clone())
                    .unwrap_or_else(|| "attachment".into()),
                _ => unreachable!(),
            };
            let mut options = FilePickerSaveOptions::new();
            options.set_suggested_file_name(Some(filename));
            if !matches!(request, HostRequest::OpenAttachment { .. }) {
                options.set_default_extension(Some(
                    if matches!(request, HostRequest::ExportPst) {
                        "pst"
                    } else {
                        "json"
                    }
                    .into(),
                ));
            }
            let Some(file) = provider
                .save_file_picker_async(options)
                .await
                .map_err(|error| error.to_string())?
            else {
                return Ok(());
            };
            let path = (file.as_ref() as &dyn IStorageItem)
                .try_get_local_path()
                .ok_or("Choose a destination on this computer.")?;
            let mut action = serde_json::to_value(request).map_err(|error| error.to_string())?;
            action["path"] = json!(path);
            native.submit(vm, action)?;
        }
        _ => return Err("Unsupported file operation.".into()),
    }
    Ok(())
}

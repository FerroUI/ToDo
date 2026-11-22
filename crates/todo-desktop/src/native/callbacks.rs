use ferroui_base::{threading::Dispatcher, Ref};
use ferroui_controls::Window;
use serde_json::{json, Value};
use std::{rc::Rc, sync::mpsc};
use todo_domain::{Action, AppState, Attachment};
use todo_state::AppViewModel;

pub fn respond(
    vm: &Rc<AppViewModel>,
    window: &Ref<Window>,
    message: Value,
    reply: mpsc::Sender<Value>,
) {
    let id = message["id"].clone();
    let method = message["method"].as_str().unwrap_or("");
    let args = &message["args"];
    if method == "openUrl" || method == "openExternalUrl" {
        let window = window.clone();
        let url = args["url"].as_str().unwrap_or("").to_owned();
        let external = method == "openExternalUrl";
        Dispatcher::ui_thread().spawn_local(async move {
            let result = async {
                let uri = url::Url::parse(&url)
                    .map_err(|_| "The Microsoft sign-in URL is invalid".to_owned())?;
                let allowed = if external {
                    matches!(
                        uri.host_str(),
                        Some(
                            "to-do.office.com"
                                | "outlook.office365.us"
                                | "outlook-dod.office365.us"
                        )
                    )
                } else {
                    matches!(
                        uri.host_str(),
                        Some(
                            "login.microsoftonline.com"
                                | "login.microsoftonline.us"
                                | "login.chinacloudapi.cn"
                        )
                    )
                };
                if uri.scheme() != "https"
                    || !allowed
                    || !uri.username().is_empty()
                    || uri.password().is_some()
                {
                    return Err("The Microsoft sign-in URL uses an unexpected authority.".into());
                }
                if !window.launcher().launch_uri_async(&uri).await {
                    return Err("Could not open the system browser.".into());
                }
                Ok(Value::Null)
            }
            .await;
            let _ = reply.send(envelope(id, result));
        });
        return;
    }
    let result = handle(vm, window, method, args);
    let _ = reply.send(envelope(id, result));
}

pub fn rejected(message: &Value) -> Value {
    envelope(
        message["id"].clone(),
        Err("The Microsoft account or document operation was cancelled.".into()),
    )
}

fn envelope(id: Value, result: Result<Value, String>) -> Value {
    match result {
        Ok(value) => json!({"type":"callbackResult","id":id,"value":value}),
        Err(error) => json!({"type":"callbackResult","id":id,"error":error}),
    }
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("The native service omitted {key}."))
}

fn handle(
    vm: &Rc<AppViewModel>,
    window: &Ref<Window>,
    method: &str,
    args: &Value,
) -> Result<Value, String> {
    match method {
        "exportData" => return vm.export_json().map(Value::String),
        "prepareCloudData" => {
            return vm
                .prepare_cloud_json(text(args, "json")?)
                .map(Value::String)
        }
        "validateImportData" => {
            return AppState::from_json(text(args, "json")?)
                .and_then(|state| state.to_json())
                .map(Value::String)
                .map_err(|e| e.to_string())
        }
        "replaceFromCloud" | "importData" => {
            if vm.export_json()? != text(args, "expected")? {
                return Err("Tasks changed while the service was working. Your edits were retained; retry the operation.".into());
            }
            if method == "replaceFromCloud" {
                vm.replace_from_cloud(text(args, "json")?)?;
            } else {
                vm.import_json(text(args, "json")?)?;
            }
        }
        "cloudState" => vm.set_cloud_state(
            args["connected"].as_bool().unwrap_or(false),
            text(args, "label")?,
        ),
        "cloudCapabilities" => vm.set_cloud_capabilities(
            args["supported"].as_bool().unwrap_or(false),
            args["configured"].as_bool().unwrap_or(false),
            args["workAccount"].as_bool().unwrap_or(false),
        ),
        "cloudHint" => vm.set_cloud_hint(text(args, "message")?),
        "outlookCategories" => vm.set_outlook_categories(text(args, "json")?)?,
        "setIntegrationPreference" => vm.set_integration_preference(
            text(args, "feature")?,
            args["enabled"].as_bool().unwrap_or(false),
        )?,
        "hostError" => vm.report_error(text(args, "message")?.to_owned()),
        "hostStatus" => vm.set_status(text(args, "message")?.to_owned()),
        "flushEdits" => todo_ui::commit_pending_edits(vm, window),
        "addAttachment" => {
            let task_id = args["task_id"]
                .as_u64()
                .ok_or("The attachment task is invalid")?;
            let attachment: Attachment = serde_json::from_value(args["attachment"].clone())
                .map_err(|_| "The attachment metadata is invalid")?;
            let mut candidate = vm.state().clone();
            candidate
                .apply(
                    Action::AddAttachment {
                        task_id,
                        attachment: attachment.clone(),
                    },
                    vm.today(),
                )
                .map_err(|e| e.to_string())?;
            let key = attachment.storage_key.clone();
            vm.dispatch(Action::AddAttachment {
                task_id,
                attachment,
            });
            if !vm.state().tasks.iter().any(|task| {
                task.id == task_id && task.attachments.iter().any(|file| file.storage_key == key)
            }) {
                return Err("The task no longer accepts attachments.".into());
            }
        }
        _ => return Err("The native service requested an unknown application operation.".into()),
    }
    Ok(Value::Null)
}

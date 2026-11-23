use serde::Serialize;
use std::rc::Rc;
use todo_domain::{Id, Theme};

/// Explicit boundary between portable view models and browser/desktop capabilities.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HostRequest {
    Export {
        filename: String,
        contents: String,
    },
    ExportArchive,
    ExportMicrosoft,
    ImportMicrosoft,
    ImportPst,
    ExportPst,
    Import,
    Attach {
        task_id: Id,
    },
    OpenAttachment {
        key: String,
    },
    DeleteAttachment {
        key: String,
    },
    Print {
        title: String,
        html: String,
    },
    Notify {
        title: String,
        body: String,
    },
    RequestNotifications,
    CompletionSound,
    SetTheme {
        theme: Theme,
    },
    ConnectMicrosoft {
        client_id: String,
    },
    ReconnectMicrosoft,
    SwitchMicrosoftAccount,
    OpenLocalWorkspace,
    SignOutMicrosoft,
    DisconnectMicrosoft,
    SyncMicrosoft,
    EnableOutlookCategories,
    RefreshOutlookCategories,
    DisableOutlookCategories,
    EnableOutlookCategoryEditing,
    DisableOutlookCategoryEditing,
    CreateOutlookCategory {
        name: String,
        color: String,
    },
    UpdateOutlookCategory {
        id: String,
        color: String,
        expected_color: String,
    },
    DeleteOutlookCategory {
        id: String,
    },
    EnablePresentationSync,
    DisablePresentationSync,
    OpenMicrosoftToDo,
    Refresh {
        sync: bool,
    },
    NavigationBack {
        available: bool,
    },
    OpenUrl {
        url: String,
    },
}

#[derive(Clone)]
pub struct HostServices {
    pub request: Rc<dyn Fn(HostRequest)>,
}

impl Default for HostServices {
    fn default() -> Self {
        Self {
            request: Rc::new(|_| {}),
        }
    }
}

pub type SaveCallback = Rc<dyn Fn(&str) -> Result<(), String>>;

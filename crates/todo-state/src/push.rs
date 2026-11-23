use crate::AppViewModel;
use serde::Deserialize;

/// Host presentation only. Credentials and enrollment drafts never enter the
/// task document, undo history, cloud synchronization, or portable backups.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PushState {
    pub available: bool,
    pub supported: bool,
    pub enabled: bool,
    pub service_url: String,
    pub public_key: String,
    pub status: String,
}

impl AppViewModel {
    pub fn push_state(&self) -> PushState {
        self.push_state.borrow().clone()
    }

    pub fn set_push_state(&self, contents: &str) -> Result<(), String> {
        if contents.len() > 8192 {
            return Err("Background reminder status is too large".into());
        }
        let state: PushState = serde_json::from_str(contents)
            .map_err(|_| "Background reminder status is invalid".to_owned())?;
        if state.service_url.len() > 2048
            || state.public_key.len() > 256
            || state.status.len() > 2048
        {
            return Err("Background reminder status is invalid".into());
        }
        if *self.push_state.borrow() != state {
            *self.push_state.borrow_mut() = state;
            self.notify();
        }
        Ok(())
    }
}

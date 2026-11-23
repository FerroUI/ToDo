use crate::AppViewModel;
use serde::Deserialize;
use todo_domain::Action;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct OutlookCategory {
    pub id: String,
    pub name: String,
    pub color: String,
    pub color_name: String,
    #[serde(default)]
    pub preset: String,
}

impl AppViewModel {
    pub fn background_reminders_supported(&self) -> bool {
        self.background_reminders_supported.get()
    }
    pub fn background_reminders_status(&self) -> String {
        self.background_reminders_status.borrow().clone()
    }
    pub fn set_background_reminders_status(&self, supported: bool, message: String) {
        let changed = self.background_reminders_supported.replace(supported) != supported
            || *self.background_reminders_status.borrow() != message;
        *self.background_reminders_status.borrow_mut() = message;
        if changed {
            self.notify();
        }
    }
    pub fn background_supported(&self) -> bool {
        self.background_supported.get()
    }

    pub fn set_background_supported(&self, supported: bool) {
        if self.background_supported.replace(supported) != supported {
            self.notify();
        }
    }

    pub fn cloud_hint(&self) -> String {
        self.cloud_hint.borrow().clone()
    }

    pub fn set_cloud_hint(&self, hint: &str) {
        if *self.cloud_hint.borrow() != hint {
            *self.cloud_hint.borrow_mut() = hint.to_owned();
            self.notify();
        }
    }

    pub fn outlook_categories(&self) -> Vec<OutlookCategory> {
        self.outlook_categories.borrow().clone()
    }

    pub fn set_outlook_categories(&self, contents: &str) -> Result<(), String> {
        let categories: Vec<OutlookCategory> = serde_json::from_str(contents)
            .map_err(|_| "Microsoft returned an invalid category catalogue".to_owned())?;
        if categories.len() > 2048
            || categories.iter().any(|category| {
                category.id.len() > 1024
                    || category.name.is_empty()
                    || category.name.len() > 1024
                    || category.color_name.len() > 64
                    || (!category.preset.is_empty()
                        && category.preset != "none"
                        && !category
                            .preset
                            .strip_prefix("preset")
                            .and_then(|value| value.parse::<u8>().ok())
                            .is_some_and(|value| {
                                value < 25 && category.preset == format!("preset{value}")
                            }))
                    || category.color.len() != 7
                    || !category.color.starts_with('#')
                    || !category.color[1..]
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit())
            })
        {
            return Err("Microsoft returned an invalid category catalogue".into());
        }
        if *self.outlook_categories.borrow() != categories {
            *self.outlook_categories.borrow_mut() = categories;
            self.notify();
        }
        Ok(())
    }

    pub fn set_integration_preference(&self, feature: &str, enabled: bool) -> Result<(), String> {
        let mut settings = self.state().settings.clone();
        match feature {
            "outlook_categories_enabled" => settings.outlook_categories_enabled = enabled,
            "outlook_categories_write_enabled" => {
                settings.outlook_categories_write_enabled = enabled
            }
            "presentation_sync_enabled" => settings.presentation_sync_enabled = enabled,
            _ => return Err("Unknown connected application preference".into()),
        }
        self.dispatch(Action::SetSettings(settings));
        Ok(())
    }

    pub fn cloud_supported(&self) -> bool {
        self.cloud_supported.get()
    }
    pub fn cloud_configured(&self) -> bool {
        self.cloud_configured.get()
    }
    pub fn cloud_work_account(&self) -> bool {
        self.cloud_work_account.get()
    }

    pub fn set_cloud_capabilities(&self, supported: bool, configured: bool, work_account: bool) {
        let changed = (self.cloud_supported.replace(supported) != supported)
            | (self.cloud_configured.replace(configured) != configured)
            | (self.cloud_work_account.replace(work_account) != work_account);
        if changed {
            self.notify();
        }
    }
}

use crate::AppViewModel;

impl AppViewModel {
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

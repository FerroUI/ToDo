use crate::{AppViewModel, HostRequest};

impl AppViewModel {
    /// Refresh retains the mounted editors and document; the host supplies the
    /// current clock and, when connected, runs the existing synchronization path.
    pub fn refresh(&self) {
        if self.refreshing.replace(true) {
            return;
        }
        self.notify();
        self.request(HostRequest::Refresh {
            sync: self.cloud_connected(),
        });
    }

    pub fn refreshing(&self) -> bool {
        self.refreshing.get()
    }

    pub fn finish_refresh(&self) {
        if self.refreshing.replace(false) {
            self.notify();
        }
    }
}

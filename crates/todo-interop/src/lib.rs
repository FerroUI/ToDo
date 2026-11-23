//! Task interchange at the host boundary. Parsing never mutates the live document.
//! PST reads use Microsoft's MIT-licensed MS-PST reference implementation.
#[cfg(not(target_arch = "wasm32"))]
pub mod allocation_budget;
mod files;
mod metadata;
mod named;
mod properties;
mod reader;
mod recurrence;
mod task;
mod walk;

use serde::Serialize;
use todo_domain::{AppState, Date};

pub const MAX_PST_BYTES: usize = 256 * 1024 * 1024;
pub(crate) const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;
pub(crate) const MAX_FILES_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const MAX_ITEMS: usize = 100_000;

#[derive(Debug)]
pub struct InteropError(String);
impl InteropError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
impl std::fmt::Display for InteropError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for InteropError {}
impl From<std::io::Error> for InteropError {
    fn from(error: std::io::Error) -> Self {
        Self(format!("The Outlook PST could not be read: {error}"))
    }
}
pub(crate) type Result<T> = std::result::Result<T, InteropError>;

#[derive(Serialize)]
pub struct ArchiveFile {
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub mime_type: String,
    pub size: usize,
    pub base64: String,
    pub sha256: String,
}

#[derive(Serialize)]
pub struct PortableArchive {
    pub kind: &'static str,
    pub version: u32,
    pub document: AppState,
    pub files: Vec<ArchiveFile>,
}

/// PST reminder timestamps are UTC. Hosts convert these into their local civil
/// timezone before validating/restoring `archive`, preserving DST at that date.
#[derive(Serialize)]
pub struct UtcReminder {
    pub task_id: u64,
    pub date_time: String,
}

#[derive(Serialize)]
pub struct UtcDate {
    pub task_id: u64,
    pub field: &'static str,
    pub date_time: String,
}

#[derive(Serialize)]
pub struct PstImport {
    pub archive: PortableArchive,
    pub warnings: Vec<String>,
    pub skipped_messages: usize,
    pub utc_reminders: Vec<UtcReminder>,
    pub utc_dates: Vec<UtcDate>,
}

/// Prepare independent local tasks from an ANSI/Unicode PST, including by-value
/// file attachments. Non-task mailbox items are counted, never imported as tasks.
/// No account identifiers, credentials, or active synchronization mappings are set.
pub fn import_pst(bytes: &[u8], current: &AppState, today: Date) -> Result<PstImport> {
    check_input_length(bytes.len())?;
    import_pst_owned(bytes.to_vec(), current, today)
}

/// The owned-input variant avoids a second PST-sized allocation in worker hosts.
/// Hosts must still isolate parsing and enforce a memory/time budget: upstream
/// MS-PST structures can request allocations before our projection limits run.
pub fn import_pst_owned(bytes: Vec<u8>, current: &AppState, today: Date) -> Result<PstImport> {
    check_input_length(bytes.len())?;
    let store = reader::open(bytes)?;
    walk::read(store, current, today)
}

fn check_input_length(length: usize) -> Result<()> {
    if length > MAX_PST_BYTES {
        return Err(InteropError::new(
            "The Outlook PST must be 256 MiB or smaller.",
        ));
    }
    Ok(())
}

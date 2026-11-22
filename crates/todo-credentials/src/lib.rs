//! Narrow, bounded protocol for the native Microsoft cache encryption key.
//!
//! Only a validated 32-byte key, encoded as lowercase hexadecimal, crosses this
//! boundary. MSAL cache contents are never passed to the platform credential
//! store. Callers own process isolation, timeouts, and private pipe transport.

mod protocol;
mod store;

pub const MAX_REQUEST_BYTES: usize = 4096;
pub const MAX_RESPONSE_BYTES: usize = 1024;

/// Execute one JSON credential operation without logging request or error data.
///
/// Responses contain only `status` and, for a successful read, the validated
/// `value`. Unsupported platforms and inaccessible stores return `unavailable`.
/// Invalid requests never initialize or access the platform credential store.
pub fn handle_request(bytes: &[u8]) -> Vec<u8> {
    protocol::handle(bytes, store::Vault::open)
}

#[cfg(test)]
mod tests;

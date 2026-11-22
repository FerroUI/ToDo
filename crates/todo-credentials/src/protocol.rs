use crate::{store::Store, store::StoreError, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES};
use serde::Deserialize;

const OK: &[u8] = br#"{"status":"ok"}"#;
const MISSING: &[u8] = br#"{"status":"missing"}"#;
const INVALID: &[u8] = br#"{"status":"invalid"}"#;
const UNAVAILABLE: &[u8] = br#"{"status":"unavailable"}"#;

// Separate variants reject even a null `value` on a read/delete request.
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "lowercase", deny_unknown_fields)]
enum Request {
    Read { scope: String },
    Write { scope: String, value: String },
    Delete { scope: String },
}

impl Request {
    fn scope(&self) -> &str {
        match self {
            Self::Read { scope } | Self::Write { scope, .. } | Self::Delete { scope } => scope,
        }
    }

    fn valid(&self) -> bool {
        valid_hex(self.scope())
            && match self {
                Self::Write { value, .. } => valid_hex(value),
                _ => true,
            }
    }
}

pub(crate) fn valid_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn handle<S: Store>(
    bytes: &[u8],
    open: impl FnOnce(&str) -> Result<S, StoreError>,
) -> Vec<u8> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return INVALID.to_vec();
    }
    let request = match serde_json::from_slice::<Request>(bytes) {
        Ok(request) if request.valid() => request,
        _ => return INVALID.to_vec(),
    };
    let mut store = match open(request.scope()) {
        Ok(store) => store,
        Err(_) => return UNAVAILABLE.to_vec(),
    };
    let response = match request {
        Request::Read { .. } => match store.read() {
            Ok(value) if valid_hex(&value) => {
                // Validated ASCII hexadecimal cannot introduce JSON syntax.
                let mut result = br#"{"status":"ok","value":""#.to_vec();
                result.extend_from_slice(value.as_bytes());
                result.extend_from_slice(br#""}"#);
                result
            }
            Err(StoreError::Missing) => MISSING.to_vec(),
            _ => UNAVAILABLE.to_vec(),
        },
        Request::Write { value, .. } => {
            // Do not advertise persistence until the backend can read back the
            // exact key. An unavailable/locked/no-op store is never a fallback.
            if store.write(&value).is_ok() && store.read().is_ok_and(|stored| stored == value) {
                OK.to_vec()
            } else {
                UNAVAILABLE.to_vec()
            }
        }
        Request::Delete { .. } => match store.delete() {
            Ok(()) | Err(StoreError::Missing) => OK.to_vec(),
            Err(_) => UNAVAILABLE.to_vec(),
        },
    };
    if response.len() <= MAX_RESPONSE_BYTES {
        response
    } else {
        UNAVAILABLE.to_vec()
    }
}

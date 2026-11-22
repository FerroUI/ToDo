use crate::{protocol, store::Store, store::StoreError, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES};
use serde_json::{json, Value};
use std::{cell::RefCell, rc::Rc};

const SCOPE: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const KEY: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

#[derive(Default)]
struct State {
    value: Option<String>,
    read_error: Option<StoreError>,
    write_error: Option<StoreError>,
    delete_error: Option<StoreError>,
    ignore_write: bool,
    calls: Vec<&'static str>,
}

#[derive(Clone, Default)]
struct MemoryStore(Rc<RefCell<State>>);

impl Store for MemoryStore {
    fn read(&mut self) -> Result<String, StoreError> {
        let mut state = self.0.borrow_mut();
        state.calls.push("read");
        if let Some(error) = state.read_error {
            return Err(error);
        }
        state.value.clone().ok_or(StoreError::Missing)
    }

    fn write(&mut self, value: &str) -> Result<(), StoreError> {
        let mut state = self.0.borrow_mut();
        state.calls.push("write");
        if let Some(error) = state.write_error {
            return Err(error);
        }
        if !state.ignore_write {
            state.value = Some(value.to_owned());
        }
        Ok(())
    }

    fn delete(&mut self) -> Result<(), StoreError> {
        let mut state = self.0.borrow_mut();
        state.calls.push("delete");
        if let Some(error) = state.delete_error {
            return Err(error);
        }
        state.value.take().ok_or(StoreError::Missing)?;
        Ok(())
    }
}

fn response(bytes: Vec<u8>) -> Value {
    assert!(bytes.len() <= MAX_RESPONSE_BYTES);
    serde_json::from_slice(&bytes).expect("bounded protocol response must be valid JSON")
}

fn run(request: Value, store: &MemoryStore) -> Value {
    response(protocol::handle(
        &serde_json::to_vec(&request).unwrap(),
        |scope| {
            assert_eq!(scope, SCOPE);
            Ok(store.clone())
        },
    ))
}

fn invalid(bytes: &[u8]) {
    let output = protocol::handle::<MemoryStore>(bytes, |_| {
        panic!("invalid input must not open an OS credential store")
    });
    assert_eq!(response(output), json!({"status":"invalid"}));
}

#[test]
fn strict_protocol_rejects_unknown_duplicate_and_inapplicable_fields() {
    for value in [
        json!({}),
        json!({"operation":"read"}),
        json!({"operation":"Read","scope":SCOPE}),
        json!({"operation":"inspect","scope":SCOPE}),
        json!({"operation":"read","scope":SCOPE,"extra":"field"}),
        json!({"operation":"read","scope":SCOPE,"value":KEY}),
        json!({"operation":"read","scope":SCOPE,"value":null}),
        json!({"operation":"delete","scope":SCOPE,"value":KEY}),
        json!({"operation":"delete","scope":SCOPE,"value":null}),
        json!({"operation":"write","scope":SCOPE}),
        json!({"operation":"write","scope":SCOPE,"value":null}),
        json!({"operation":"write","scope":SCOPE,"value":KEY,"service":"other"}),
        json!([{"operation":"read","scope":SCOPE}]),
    ] {
        invalid(&serde_json::to_vec(&value).unwrap());
    }
    invalid(format!(r#"{{"operation":"read","operation":"delete","scope":"{SCOPE}"}}"#).as_bytes());
    invalid(format!(r#"{{"operation":"read","scope":"{SCOPE}","scope":"{SCOPE}"}}"#).as_bytes());
    invalid(b"{} trailing");
    invalid(b"\xff");
    invalid(b"");
}

#[test]
fn scope_and_keys_require_exact_lowercase_ascii_hex() {
    for value in [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        "0".repeat(63) + "\0",
        "é".repeat(32),
        " ".repeat(64),
    ] {
        invalid(&serde_json::to_vec(&json!({"operation":"read","scope":value})).unwrap());
        invalid(
            &serde_json::to_vec(&json!({"operation":"write","scope":SCOPE,"value":value})).unwrap(),
        );
    }
    invalid(&serde_json::to_vec(&json!({"operation":"read","scope":123})).unwrap());
    invalid(&serde_json::to_vec(&json!({"operation":"write","scope":SCOPE,"value":123})).unwrap());
}

#[test]
fn request_limit_is_checked_before_deserializing_or_opening_the_store() {
    let request = serde_json::to_vec(&json!({"operation":"read","scope":SCOPE})).unwrap();
    let mut oversized = request.clone();
    oversized.resize(MAX_REQUEST_BYTES + 1, b' ');
    invalid(&oversized);

    let mut boundary = request;
    boundary.resize(MAX_REQUEST_BYTES, b' ');
    assert_eq!(
        response(protocol::handle(&boundary, |_| Ok(MemoryStore::default()))),
        json!({"status":"missing"})
    );
}

#[test]
fn reads_return_only_valid_keys_and_distinguish_a_missing_entry() {
    let store = MemoryStore::default();
    assert_eq!(
        run(json!({"operation":"read","scope":SCOPE}), &store),
        json!({"status":"missing"})
    );
    store.0.borrow_mut().value = Some(KEY.to_owned());
    assert_eq!(
        run(json!({"operation":"read","scope":SCOPE}), &store),
        json!({"status":"ok","value":KEY})
    );
    for malformed in [
        "secret outside the protocol".to_owned(),
        "A".repeat(64),
        "a".repeat(65),
        "".into(),
    ] {
        store.0.borrow_mut().value = Some(malformed);
        assert_eq!(
            run(json!({"operation":"read","scope":SCOPE}), &store),
            json!({"status":"unavailable"})
        );
    }
}

#[test]
fn writes_verify_the_exact_persisted_key_without_echoing_it() {
    let store = MemoryStore::default();
    assert_eq!(
        run(
            json!({"operation":"write","scope":SCOPE,"value":KEY}),
            &store
        ),
        json!({"status":"ok"})
    );
    let state = store.0.borrow();
    assert_eq!(state.calls, ["write", "read"]);
    assert_eq!(state.value.as_deref(), Some(KEY));
}

#[test]
fn failed_or_unverified_writes_never_report_persistence() {
    let store = MemoryStore::default();
    store.0.borrow_mut().write_error = Some(StoreError::Unavailable);
    assert_eq!(
        run(
            json!({"operation":"write","scope":SCOPE,"value":KEY}),
            &store
        ),
        json!({"status":"unavailable"})
    );
    assert_eq!(store.0.borrow().calls, ["write"]);

    let store = MemoryStore::default();
    store.0.borrow_mut().ignore_write = true;
    for previous in [
        None,
        Some(SCOPE.to_owned()),
        Some("malformed secret".into()),
    ] {
        store.0.borrow_mut().value = previous;
        assert_eq!(
            run(
                json!({"operation":"write","scope":SCOPE,"value":KEY}),
                &store
            ),
            json!({"status":"unavailable"})
        );
    }
    store.0.borrow_mut().ignore_write = false;
    store.0.borrow_mut().read_error = Some(StoreError::Unavailable);
    assert_eq!(
        run(
            json!({"operation":"write","scope":SCOPE,"value":KEY}),
            &store
        ),
        json!({"status":"unavailable"})
    );
}

#[test]
fn deletion_is_idempotent_but_storage_failure_is_not_success() {
    let store = MemoryStore::default();
    for previous in [Some(KEY.to_owned()), None] {
        store.0.borrow_mut().value = previous;
        assert_eq!(
            run(json!({"operation":"delete","scope":SCOPE}), &store),
            json!({"status":"ok"})
        );
        assert!(store.0.borrow().value.is_none());
    }
    store.0.borrow_mut().delete_error = Some(StoreError::Unavailable);
    assert_eq!(
        run(json!({"operation":"delete","scope":SCOPE}), &store),
        json!({"status":"unavailable"})
    );
}

#[test]
fn unavailable_backend_returns_a_fixed_response_without_request_data() {
    for operation in ["read", "delete"] {
        let bytes = serde_json::to_vec(&json!({"operation":operation,"scope":SCOPE})).unwrap();
        assert_eq!(
            response(protocol::handle::<MemoryStore>(&bytes, |_| Err(
                StoreError::Unavailable
            ))),
            json!({"status":"unavailable"})
        );
    }
    let store = MemoryStore::default();
    store.0.borrow_mut().read_error = Some(StoreError::Unavailable);
    assert_eq!(
        run(json!({"operation":"read","scope":SCOPE}), &store),
        json!({"status":"unavailable"})
    );
}

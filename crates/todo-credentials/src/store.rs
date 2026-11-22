pub(crate) const SERVICE: &str = "FerroUI.ToDo.MicrosoftCache.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StoreError {
    Missing,
    Unavailable,
}

// Backend details are erased at the boundary: they can contain credential bytes.
pub(crate) trait Store {
    fn read(&mut self) -> Result<String, StoreError>;
    fn write(&mut self, value: &str) -> Result<(), StoreError>;
    fn delete(&mut self) -> Result<(), StoreError>;
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub(crate) struct Vault(keyring::Entry);

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl Vault {
    pub(crate) fn open(scope: &str) -> Result<Self, StoreError> {
        keyring::Entry::new(SERVICE, scope)
            .map(Self)
            .map_err(redact)
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn redact(error: keyring::Error) -> StoreError {
    match error {
        keyring::Error::NoEntry => StoreError::Missing,
        _ => StoreError::Unavailable,
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl Store for Vault {
    fn read(&mut self) -> Result<String, StoreError> {
        self.0.get_password().map_err(redact)
    }

    fn write(&mut self, value: &str) -> Result<(), StoreError> {
        self.0.set_password(value).map_err(redact)
    }

    fn delete(&mut self) -> Result<(), StoreError> {
        self.0.delete_credential().map_err(redact)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub(crate) struct Vault;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl Vault {
    pub(crate) fn open(_scope: &str) -> Result<Self, StoreError> {
        Err(StoreError::Unavailable)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl Store for Vault {
    fn read(&mut self) -> Result<String, StoreError> {
        Err(StoreError::Unavailable)
    }

    fn write(&mut self, _value: &str) -> Result<(), StoreError> {
        Err(StoreError::Unavailable)
    }

    fn delete(&mut self) -> Result<(), StoreError> {
        Err(StoreError::Unavailable)
    }
}

#[cfg(all(
    test,
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
mod tests {
    use super::*;
    use std::{error::Error, fmt};

    #[derive(Debug)]
    struct NeverFormat;

    impl fmt::Display for NeverFormat {
        fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            panic!("credential backend details must never be formatted");
        }
    }

    impl Error for NeverFormat {}

    #[test]
    fn backend_errors_are_classified_without_formatting_secret_details() {
        assert_eq!(redact(keyring::Error::NoEntry), StoreError::Missing);
        for error in [
            keyring::Error::BadEncoding(b"untrusted-secret-bytes".to_vec()),
            keyring::Error::BadDataFormat(
                b"untrusted-secret-bytes".to_vec(),
                Box::new(NeverFormat),
            ),
            keyring::Error::PlatformFailure(Box::new(NeverFormat)),
            keyring::Error::NoStorageAccess(Box::new(NeverFormat)),
            keyring::Error::NoDefaultStore,
        ] {
            assert_eq!(redact(error), StoreError::Unavailable);
        }
    }
}

/// Retrieve a password from the system keychain.
///
/// # Errors
///
/// Returns `crab_common::Error::Auth` if the keychain entry cannot be accessed.
pub fn get(service: &str, key: &str) -> crab_common::Result<String> {
    let entry = keyring::Entry::new(service, key).map_err(|e| {
        crab_common::Error::Auth(format!("keychain init failed: {e}"))
    })?;
    entry.get_password().map_err(|e| {
        crab_common::Error::Auth(format!("keychain read failed: {e}"))
    })
}

/// Store a password in the system keychain.
///
/// # Errors
///
/// Returns `crab_common::Error::Auth` if the keychain entry cannot be written.
pub fn set(service: &str, key: &str, value: &str) -> crab_common::Result<()> {
    let entry = keyring::Entry::new(service, key).map_err(|e| {
        crab_common::Error::Auth(format!("keychain init failed: {e}"))
    })?;
    entry.set_password(value).map_err(|e| {
        crab_common::Error::Auth(format!("keychain write failed: {e}"))
    })
}

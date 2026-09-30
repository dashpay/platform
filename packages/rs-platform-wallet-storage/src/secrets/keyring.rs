//! OS-keyring construction helper.
//!
//! Built on `keyring-core 1.0.0` (the SPI) plus the per-platform
//! credential-store crates; the `keyring` 4.x sample CLI crate is
//! deliberately not a dependency. There is no crate-local wrapper —
//! [`default_credential_store`]'s return value is used directly via
//! [`keyring_core::api::CredentialStoreApi`] or installed as the process
//! default via [`keyring_core::set_default_store`].
//!
//! Threat coverage: protects **A1** (other local user) and **A4** (lost
//! laptop) where the platform encrypts items at rest and scopes them to
//! the user. Does **not** cover **A2/A3** same-user malware (most OS
//! keyrings hand the secret to any same-user process) or **A5** keyring
//! scraping; headless Linux without Secret Service fails closed
//! ([`keyring_core::Error::NoDefaultStore`]), never degrading to plaintext.
//!
//! Metadata is enumerable plaintext: entries are keyed by `service =
//! SERVICE_PREFIX + hex(wallet_id)` and `user = label`, stored as
//! plaintext keyring metadata. Same-user list-only tooling can enumerate
//! which wallet ids and slot kinds exist without unlocking a secret —
//! dominated by the accepted A2/A3 residual, with no portable knob to
//! redact it. Operators wanting metadata hiding should prefer
//! [`EncryptedFileStore`](super::EncryptedFileStore), whose
//! `(wallet_id, label)` map lives only inside the sealed vault.
//!
//! Per-OS: items persist `UntilDelete` everywhere. Linux/FreeBSD use
//! Secret Service (gnome-keyring / KWallet), which needs a D-Bus session
//! and an unlocked collection. macOS Keychain ACL accepts a re-signed
//! binary with the same code-signing identity (residual). Windows DPAPI
//! is user-profile scoped and defends A1/A4 only, not same-user malware.

use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::Error as KeyringError;

/// Open the platform's default credential store, failing closed (typed
/// [`KeyringError::NoDefaultStore`]) when none is reachable (headless / no
/// Secret Service / no D-Bus). Never panics, never falls back to a weaker
/// store. The returned `Arc` works with
/// [`keyring_core::set_default_store`] or builds entries directly.
///
/// SPI-direct consumers: format the returned [`KeyringError`] with
/// `Display` (`{}`), **never** `Debug` — upstream `BadEncoding` /
/// `BadDataFormat` variants embed raw bytes in `Debug` (CWE-209/CWE-532).
/// The typed [`SecretStore`](super::SecretStore) path avoids the SPI error.
pub fn default_credential_store() -> Result<Arc<dyn CredentialStoreApi + Send + Sync>, KeyringError>
{
    platform_default_store()
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn platform_default_store() -> Result<Arc<dyn CredentialStoreApi + Send + Sync>, KeyringError> {
    // Secret Service is the only backend; an unreachable D-Bus session
    // (headless / SSH / CI) is fail-closed by design.
    match dbus_secret_service_keyring_store::Store::new() {
        Ok(s) => Ok(s),
        Err(e) => {
            tracing::debug!(error = %e, "secret service keyring init failed; falling back to NoDefaultStore");
            Err(KeyringError::NoDefaultStore)
        }
    }
}

#[cfg(target_os = "macos")]
fn platform_default_store() -> Result<Arc<dyn CredentialStoreApi + Send + Sync>, KeyringError> {
    // `apple-native-keyring-store` >= 1.0 exposes `Store` under the
    // `keychain` module, not the crate root like the sibling backends.
    match apple_native_keyring_store::keychain::Store::new() {
        Ok(s) => Ok(s),
        Err(e) => {
            tracing::debug!(error = %e, "keychain keyring init failed; falling back to NoDefaultStore");
            Err(KeyringError::NoDefaultStore)
        }
    }
}

#[cfg(target_os = "windows")]
fn platform_default_store() -> Result<Arc<dyn CredentialStoreApi + Send + Sync>, KeyringError> {
    match windows_native_keyring_store::Store::new() {
        Ok(s) => Ok(s),
        Err(e) => {
            tracing::debug!(error = %e, "dpapi keyring init failed; falling back to NoDefaultStore");
            Err(KeyringError::NoDefaultStore)
        }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "macos",
    target_os = "windows"
)))]
fn platform_default_store() -> Result<Arc<dyn CredentialStoreApi + Send + Sync>, KeyringError> {
    Err(KeyringError::NoDefaultStore)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_fails_closed_not_panic() {
        // On headless CI the constructor returns `NoDefaultStore`;
        // where a keyring exists it succeeds. Either way: typed, no
        // panic, no plaintext fallback.
        match default_credential_store() {
            Ok(_) | Err(KeyringError::NoDefaultStore) => {}
            Err(other) => panic!("unexpected: {other}"),
        }
    }
}

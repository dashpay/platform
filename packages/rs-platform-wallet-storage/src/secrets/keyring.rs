//! OS-keyring construction helper.
//!
//! Built on `keyring-core 1.0.0` (the SPI library) plus the
//! per-platform credential-store crates; the `keyring` 4.x sample CLI
//! crate itself is intentionally not a dependency.
//!
//! There is no crate-local wrapper around the per-platform store: a
//! caller takes [`default_credential_store`]'s return value and either
//! uses it directly via [`keyring_core::api::CredentialStoreApi`] or
//! installs it as the process default via
//! [`keyring_core::set_default_store`].
//!
//! ## Threat coverage
//!
//! Covers **A1** (other local user) and **A4** (lost laptop) where the
//! platform encrypts keyring items at rest and scopes them to the user.
//! Does **not** cover **A2/A3** same-user malware (most OS keyrings
//! hand the secret to any same-user process that asks), **A5** if the
//! keyring daemon itself is scraped, or **headless Linux** with no
//! Secret Service — that fails closed
//! ([`keyring_core::Error::NoDefaultStore`]), never degrades to
//! plaintext.
//!
//! ### Metadata is enumerable plaintext
//!
//! On every OS arm the entry is keyed by `service = SERVICE_PREFIX +
//! hex(wallet_id)` and `user = label`. Those two strings are stored as
//! **plaintext, enumerable** keyring metadata: same-user list-only
//! tooling can see *which wallet ids exist* and *which slot kinds*
//! (labels) each has, without ever unlocking a secret. This is strictly
//! weaker than — and dominated by — the already-accepted A2/A3 same-user
//! residual (most OS keyrings hand the secret itself to any same-user
//! process). The `keyring-core` 1.0.0 `build` modifiers are vendor-
//! specific creation hints that do NOT replace the `(service, user)`
//! identity, so they cannot hide this pair; there is no portable knob to
//! redact it. Operators who want metadata hiding (or non-descriptive
//! labels) should prefer the
//! [`EncryptedFileStore`](super::EncryptedFileStore), whose `(wallet_id,
//! label)` map lives only inside the AEAD-sealed vault.
//!
//! ### Per-OS reality
//!
//! - **Linux/FreeBSD:** Secret Service (gnome-keyring / KWallet) is the
//!   sole backend. It requires a D-Bus session + unlocked collection;
//!   headless / SSH / CI boxes frequently lack it, in which case the
//!   store fails closed with `NoDefaultStore` and the operator selects
//!   [`EncryptedFileStore`](super::EncryptedFileStore) explicitly.
//!   Items persist `UntilDelete`. Callers that need durable storage on
//!   a headless host should pin
//!   [`EncryptedFileStore`](super::EncryptedFileStore) instead.
//! - **macOS:** Keychain ACL — a re-signed binary with the same
//!   code-signing identity is an accepted residual risk. Items persist
//!   `UntilDelete`.
//! - **Windows:** Credential Manager / DPAPI is user-profile scoped; a
//!   same-user process can unprotect it. DPAPI is **not** a defense
//!   against same-user malware, only A1/A4. Items persist
//!   `UntilDelete`.

use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::Error as KeyringError;

/// Open the platform's default credential store, failing closed
/// (typed [`KeyringError::NoDefaultStore`]) when none is reachable
/// (headless / no Secret Service / no D-Bus). Never panics, never
/// falls back to a weaker store.
///
/// The returned `Arc` may be passed straight to
/// [`keyring_core::set_default_store`] or used directly to build
/// entries.
///
/// # Error formatting (SPI-direct consumers)
///
/// A consumer driving the raw `keyring_core` SPI through this store holds
/// upstream [`keyring_core::Error`] values. Always format them with
/// `Display` (`{}`), **never** `Debug` (`{:?}`): the upstream
/// `BadEncoding(Vec<u8>)` / `BadDataFormat(Vec<u8>, _)` variants embed
/// raw bytes in their `Debug` output (CWE-209/CWE-532). This crate's own
/// backends never construct those variants with secret bytes, and the
/// typed [`SecretStore`](super::SecretStore) path avoids the SPI error
/// entirely.
pub fn default_credential_store() -> Result<Arc<dyn CredentialStoreApi + Send + Sync>, KeyringError>
{
    platform_default_store()
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn platform_default_store() -> Result<Arc<dyn CredentialStoreApi + Send + Sync>, KeyringError> {
    // Secret Service (gnome-keyring / KWallet) is the only OS backend.
    // No reachable D-Bus session / unlocked collection (headless, SSH,
    // CI) is fail-closed by design — the operator selects
    // EncryptedFileStore explicitly instead.
    match dbus_secret_service_keyring_store::Store::new() {
        Ok(s) => Ok(s),
        Err(_) => Err(KeyringError::NoDefaultStore),
    }
}

#[cfg(target_os = "macos")]
fn platform_default_store() -> Result<Arc<dyn CredentialStoreApi + Send + Sync>, KeyringError> {
    // `apple-native-keyring-store` >= 1.0 with the `keychain` feature
    // exposes `Store` under the `keychain` module, not at the crate
    // root (sibling backends — `dbus-secret-service-keyring-store`,
    // `windows-native-keyring-store` — do put `Store` at the root, hence
    // the asymmetric path).
    match apple_native_keyring_store::keychain::Store::new() {
        Ok(s) => Ok(s),
        Err(_) => Err(KeyringError::NoDefaultStore),
    }
}

#[cfg(target_os = "windows")]
fn platform_default_store() -> Result<Arc<dyn CredentialStoreApi + Send + Sync>, KeyringError> {
    match windows_native_keyring_store::Store::new() {
        Ok(s) => Ok(s),
        Err(_) => Err(KeyringError::NoDefaultStore),
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

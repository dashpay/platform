//! Build-only proof (M-S4) that the `secrets` public surface is reachable
//! from the crate root, not only by a deep module path.
//!
//! Naming every re-export in a body that never runs a backend is the whole
//! assertion: it fails to COMPILE if a type stops being re-exported at
//! `platform_wallet_storage::secrets`.
//!
//! It does NOT prove that `secrets` is default-on, and cannot: the
//! dev-dependency this file compiles under sets `default-features = false` and
//! then lists `secrets` explicitly, so the feature is on by request here, not
//! by default. Proving the shipped default set would take a separate crate or
//! a CI step that builds with real defaults.

#![cfg(feature = "secrets")]

use platform_wallet_storage::secrets::{
    default_credential_store, EncryptedFileStore, SecretBytes, SecretStoreError, SecretString,
    WalletId, MAX_PLAINTEXT_LEN, MIN_PASSPHRASE_LEN, SERVICE_PREFIX,
};

#[test]
fn default_build_exposes_secrets_surface() {
    // Type-only proof: name every public re-export.
    fn _accepts_path(
        p: &std::path::Path,
        pw: SecretString,
    ) -> Result<EncryptedFileStore, SecretStoreError> {
        EncryptedFileStore::open(p, pw)
    }
    let _ = _accepts_path as fn(_, _) -> _;
    let _ = SERVICE_PREFIX.len();
    // The Tier-2 public consts are re-exported on the default build.
    let _ = MAX_PLAINTEXT_LEN;
    let _ = MIN_PASSPHRASE_LEN;
    let _ = std::mem::size_of::<WalletId>();
    let _ = std::mem::size_of::<SecretBytes>();
    let _ = std::mem::size_of::<SecretStoreError>();
    let _: fn() -> Result<_, keyring_core::Error> = default_credential_store;
}

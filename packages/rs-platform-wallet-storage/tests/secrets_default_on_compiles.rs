//! Build-only proof (M-S4) that the default build (no flag passed)
//! reaches `EncryptedFileStore` as a public type.
//!
//! With `secrets` in the default feature set, importing the type from
//! the crate root without enabling any feature flag is the assertion.
//! The test body never exercises a backend — it only compiles.

#![cfg(feature = "secrets")]

use platform_wallet_storage::secrets::{
    default_credential_store, EncryptedFileStore, SecretBytes, SecretStoreError, SecretString,
    WalletId, SERVICE_PREFIX,
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
    let _ = std::mem::size_of::<WalletId>();
    let _ = std::mem::size_of::<SecretBytes>();
    let _ = std::mem::size_of::<SecretStoreError>();
    let _: fn() -> Result<_, keyring_core::Error> = default_credential_store;
}

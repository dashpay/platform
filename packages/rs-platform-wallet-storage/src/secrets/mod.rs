//! Out-of-band storage for wallet secret material (mnemonic / seed /
//! xpriv), kept entirely off the SQLite persister's data path.
//!
//! Consumers use [`SecretStore`], the public never-leaking front door:
//! reads yield a zeroizing [`SecretBytes`] (a raw `Vec<u8>` never crosses
//! the boundary), writes take `&SecretBytes`, and errors are the typed
//! [`SecretStoreError`] (lossless on the file arm). Pick a backend
//! explicitly — [`SecretStore::file`] (Argon2id + XChaCha20-Poly1305
//! vault, headless/server) or [`SecretStore::os`] (OS keyring, desktop;
//! fail-closed on headless Linux). There is no silent fallback.
//!
//! Below `SecretStore` the backend SPI is upstream's
//! [`keyring_core::api::CredentialStoreApi`] / [`CredentialApi`], exposed
//! directly by [`EncryptedFileStore`] / [`default_credential_store`];
//! its `keyring_core::Error` projection is lossy and string-only, so
//! consumers should prefer `SecretStore`.
//!
//! [`CredentialApi`]: keyring_core::api::CredentialApi
//! [`CredentialStoreApi`]: keyring_core::api::CredentialStoreApi
//!
//! This `src/secrets/` tree is the sole secret-bearing module:
//! `tests/secrets_scan.rs` exempts it, so it owns its own review
//! discipline via `tests/secrets_guard.rs`.

mod error;
mod file;
mod keyring;
mod secret;
mod store;
mod validate;

pub use error::{IoError, OsKeyringErrorKind, SecretStoreError};
pub use file::{
    EncryptedFileCredential, EncryptedFileStore, MAX_SECRET_LEN, MAX_VAULT_SIZE_BYTES,
    SERVICE_PREFIX,
};
pub use keyring::default_credential_store;
pub use secret::{SecretBytes, SecretString};
pub use store::SecretStore;
pub use validate::WalletId;

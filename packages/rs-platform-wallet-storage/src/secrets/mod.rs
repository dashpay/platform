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
//!
//! Cryptographic wire format lives in [`mod@wire`]: the Tier-2
//! envelope (`wire::envelope`) and the three AAD constructions
//! (`wire::aad`) are bincode-encoded against a single `WIRE_CONFIG`, so
//! a future bincode-config drift is caught by the golden-vector tests
//! in `wire::envelope::tests` rather than silently corrupting every
//! stored blob.

mod error;
mod file;
mod guarded;
mod keyring;
mod secret;
mod store;
mod validate;
mod wire;

pub use error::{IoError, OsKeyringErrorKind, SecretStoreError};
pub use file::{
    EncryptedFileCredential, EncryptedFileStore, MAX_SECRET_LEN, MAX_VAULT_SIZE_BYTES,
    SERVICE_PREFIX,
};
pub use keyring::default_credential_store;
pub use secret::{SecretBytes, SecretString, MAX_PASSPHRASE_LEN, MIN_PASSPHRASE_LEN};
pub use store::SecretStore;
pub use validate::WalletId;
pub use wire::envelope::MAX_PLAINTEXT_LEN;

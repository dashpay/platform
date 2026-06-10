//! Out-of-band storage for wallet secret material (mnemonic / seed /
//! xpriv), kept entirely off the SQLite persister's data path.
//!
//! # Consumer entry point: [`SecretStore`]
//!
//! [`SecretStore`] is the public, never-leaking front door. Its read
//! path ([`SecretStore::get`]) yields a zeroizing [`SecretBytes`] — a raw
//! `Vec<u8>` never crosses this boundary — and its write path
//! ([`SecretStore::set`]) takes `&SecretBytes`, so a caller cannot pass an
//! unwrapped buffer. Errors surface as the typed [`SecretStoreError`],
//! losslessly for the file arm (`WrongPassphrase` vs `Corruption` vs
//! `AlreadyLocked` stay distinct).
//!
//! - [`SecretStore::file`] — Argon2id + XChaCha20-Poly1305 vault file.
//!   Recommended on **headless / server** hosts; fully self-contained.
//! - [`SecretStore::os`] — the platform OS keyring, fail-closed on
//!   headless Linux. Recommended on **desktop**.
//!
//! # Internal SPI
//!
//! Below `SecretStore`, the backend SPI is upstream's
//! [`keyring_core::api::CredentialStoreApi`] / [`CredentialApi`].
//! [`EncryptedFileStore`] and [`default_credential_store`] expose that
//! SPI directly; their `keyring_core::Error` projection is **lossy and
//! string-only** (the typed distinction lives on the `SecretStore` path).
//! Consumers should prefer `SecretStore`.
//!
//! - [`SecretBytes`] / [`SecretString`] — zeroize-on-drop wrappers.
//! - [`SecretStoreError`] — the typed error returned by `SecretStore`
//!   and both backends, projected into `keyring_core::Error` for the SPI.
//!
//! [`CredentialApi`]: keyring_core::api::CredentialApi
//! [`CredentialStoreApi`]: keyring_core::api::CredentialStoreApi
//!
//! Everything secret-bearing lives under this `src/secrets/` tree by
//! design: `tests/secrets_scan.rs` scans only `src/sqlite/schema/` +
//! `migrations/` and exempts this module, so this module owns its own
//! review discipline (`tests/secrets_guard.rs`).
//!
//! # Memory hygiene
//!
//! At the SPI seam the upstream `get_secret` returns `Vec<u8>`;
//! [`SecretStore::get`] wraps it via [`SecretBytes::new`] **immediately**
//! (no named intermediate `Vec` binding) so the bare buffer's window is
//! zero statements: `SecretBytes::new` moves the `Vec` into a
//! `Zeroizing<Vec<u8>>` without copying.
//!
//! # Backend selection
//!
//! Selection is an explicit operator decision — there is no silent
//! fallback between the file vault and the OS keyring.

mod error;
mod file;
mod keyring;
mod secret;
mod store;
mod validate;

pub use error::{IoError, OsKeyringErrorKind, SecretStoreError};
pub use file::{EncryptedFileCredential, EncryptedFileStore, MAX_VAULT_SIZE_BYTES, SERVICE_PREFIX};
pub use keyring::default_credential_store;
pub use secret::{SecretBytes, SecretString};
pub use store::SecretStore;
pub use validate::WalletId;

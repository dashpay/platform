//! Out-of-band storage for wallet secret material (mnemonic / seed /
//! xpriv), kept entirely off the SQLite persister's data path.
//!
//! The SPI is upstream's
//! [`keyring_core::api::CredentialStoreApi`] / [`CredentialApi`].
//! This crate contributes:
//!
//! - [`EncryptedFileStore`] — Argon2id + XChaCha20-Poly1305 vault file
//!   `CredentialStoreApi` implementation. Recommended on **headless /
//!   server** hosts; fully self-contained, no environment caveat.
//! - [`default_credential_store`] — opens the platform OS keyring as a
//!   `CredentialStoreApi`, fail-closed with
//!   [`keyring_core::Error::NoDefaultStore`] on headless Linux
//!   (SEC-REQ-2.1.3 / AR-4). Recommended on **desktop** OSes.
//! - [`SecretBytes`] / [`SecretString`] — zeroize-on-drop wrappers
//!   applied at the consumer seam (the upstream SPI returns bare
//!   `Vec<u8>` from `get_secret`; we re-wrap immediately).
//! - [`FileStoreError`] / [`FileStoreFailure`] — file-backend
//!   construction errors + the unit-only marker bridged into
//!   `keyring_core::Error` for the `CredentialApi` seam.
//!
//! [`CredentialApi`]: keyring_core::api::CredentialApi
//! [`CredentialStoreApi`]: keyring_core::api::CredentialStoreApi
//!
//! Everything secret-bearing lives under this `src/secrets/` tree by
//! design: `tests/secrets_scan.rs` scans only `src/sqlite/schema/` +
//! `migrations/` and exempts this module, so this module owns its own
//! review discipline (`tests/secrets_guard.rs`, SEC-REQ-4.5/4.5.1).
//!
//! # Memory hygiene
//!
//! The upstream SPI returns `Vec<u8>` from `get_secret`. Consumers
//! MUST wrap it via [`SecretBytes::new`] **immediately** (no named
//! intermediate `Vec` binding) so the bare buffer's window is zero
//! statements (Smythe EDIT-1): `SecretBytes::new` `std::mem::take`s
//! the `Vec` into a `Zeroizing<Vec<u8>>` without copying.
//!
//! # Backend selection
//!
//! Selection is an explicit operator decision — there is no silent
//! fallback between [`EncryptedFileStore`] and the OS keyring
//! (SEC-REQ-2.1.3 / AR-4).

mod file;
mod keyring;
mod secret;
mod seed_provider_adapter;
mod validate;

#[cfg(any(test, feature = "__secrets-test-helpers"))]
mod memory;

pub use file::error::FileStoreError;
pub use file::error_bridge::{downcast_failure, FileStoreFailure};
pub use file::{EncryptedFileCredential, EncryptedFileStore, SERVICE_PREFIX};
pub use keyring::default_credential_store;
pub use secret::{SecretBytes, SecretString};
pub use seed_provider_adapter::CredentialStoreSeedProvider;
pub use validate::WalletId;

#[cfg(any(test, feature = "__secrets-test-helpers"))]
pub use memory::{MemoryCredential, MemoryCredentialStore};

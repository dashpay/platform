//! Out-of-band storage for wallet secret material (mnemonic / seed /
//! xpriv), kept entirely off the SQLite persister's data path.
//!
//! Enabled by the opt-in `secrets` feature (never on by `default`).
//! Everything secret-bearing lives under this `src/secrets/` tree by
//! design: `tests/secrets_scan.rs` scans only `src/sqlite/schema/` +
//! `migrations/` and exempts this module, so this module owns its own
//! review discipline (`tests/secrets_guard.rs`, SEC-REQ-4.5/4.5.1).
//!
//! # Backends & selection
//!
//! Two production backends ship; **selection is an explicit operator
//! decision — there is no silent fallback between them** (SEC-REQ-2.1.3
//! / AR-4):
//!
//! - [`KeyringStore`] — OS keyring. Recommended default on **desktop**
//!   OSes. Fails closed on headless Linux (no Secret Service) with a
//!   typed [`SecretStoreError::BackendUnavailable`], never a degraded
//!   plaintext store.
//! - [`EncryptedFileStore`] — Argon2id + XChaCha20-Poly1305 vault file.
//!   Recommended default on **headless / server** hosts; fully
//!   self-contained, no environment caveat.
//!
//! [`MemoryStore`] is test-only and gated so it is unreachable from
//! production builds.
//!
//! # Memory hygiene
//!
//! Secrets cross every boundary inside [`SecretBytes`] / [`SecretString`]
//! (zeroize-on-drop, redacting `Debug`, no `Display`/`Serialize`,
//! best-effort `mlock`). Errors are a concrete enum with no secret in
//! any variant.

mod error;
mod file;
mod keyring;
mod secret;
mod store;
mod validate;

#[cfg(any(test, feature = "__secrets-test-helpers"))]
mod memory;

pub use error::SecretStoreError;
pub use file::EncryptedFileStore;
pub use keyring::KeyringStore;
pub use secret::{SecretBytes, SecretString};
pub use store::SecretStore;
pub use validate::WalletId;

#[cfg(any(test, feature = "__secrets-test-helpers"))]
pub use memory::MemoryStore;

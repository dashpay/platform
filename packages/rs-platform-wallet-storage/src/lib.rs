//! Storage backends for the `platform-wallet` crate.
//!
//! The SQLite-backed [`sqlite::SqlitePersister`] implements
//! [`PlatformWalletPersistence`](platform_wallet::changeset::PlatformWalletPersistence)
//! for the persister DTO (public wallet state — no secrets).
//!
//! The [`secrets`] submodule's consumer entry point is
//! [`secrets::SecretStore`]: `get` yields a zeroizing
//! [`secrets::SecretBytes`] (never a raw `Vec<u8>`) and `set` takes
//! `&SecretBytes`, over an Argon2id + XChaCha20-Poly1305 vault file or
//! the platform OS keyring. The internal SPI is
//! `keyring_core::api::CredentialStoreApi`
//! ([`secrets::EncryptedFileStore`], [`secrets::default_credential_store`]).
//! See [`SECRETS.md`](../SECRETS.md) for the full key shape,
//! memory-hygiene contract, and audit hooks.
//!
//! ## Canonical type paths
//!
//! Both work; pick whichever reads better in your call site:
//!
//! ```rust,ignore
//! use platform_wallet_storage::SqlitePersister;             // root re-export
//! use platform_wallet_storage::sqlite::SqlitePersister;     // submodule re-export
//! use platform_wallet_storage::sqlite::persister::SqlitePersister; // deep path
//! ```

#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]

#[cfg(feature = "kv")]
pub mod kv;
#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "secrets")]
pub mod secrets;

// Convenience re-exports kept under the crate root so embedders don't
// have to spell out the `::sqlite::` middle segment for the common
// names. Adding to or trimming from this list does NOT count as a
// breaking change of the submodule API.
#[cfg(feature = "kv")]
pub use kv::{KvError, KvStore, ObjectId};
#[cfg(feature = "sqlite")]
pub use sqlite::{
    default_auto_backup_dir, AutoBackupOperation, CommitReport, DeleteWalletReport, FlushMode,
    JournalMode, PruneReport, RetentionPolicy, SqlitePersister, SqlitePersisterConfig, Synchronous,
    WalletStorageError,
};

// Compile-time assertions — `Send + Sync`, `PlatformWalletPersistence`
// object-safety, and the no-boxed-trait-object error policy.
// Lint-gated to the SQLite feature because they reference its types.
#[cfg(feature = "sqlite")]
#[allow(dead_code)]
const fn _send_sync_check<T: Send + Sync>() {}
#[cfg(feature = "sqlite")]
const _: () = {
    _send_sync_check::<SqlitePersister>();
    _send_sync_check::<WalletStorageError>();
};

#[cfg(feature = "sqlite")]
#[allow(dead_code)]
fn _object_safety_check(persister: SqlitePersister) {
    let _: std::sync::Arc<dyn platform_wallet::changeset::PlatformWalletPersistence> =
        std::sync::Arc::new(persister);
}

// The keyring SPI must be object-safe and its error `Send + Sync`, so
// a backend can be held behind `Arc<dyn CredentialStoreApi + Send +
// Sync>` and its errors crossed between threads / FFI.
#[cfg(feature = "secrets")]
#[allow(dead_code)]
const fn _secrets_send_sync_check<T: Send + Sync>() {}
#[cfg(feature = "secrets")]
const _: () = {
    _secrets_send_sync_check::<keyring_core::Error>();
    _secrets_send_sync_check::<secrets::SecretStoreError>();
};
#[cfg(feature = "secrets")]
#[allow(dead_code)]
fn _credential_store_object_safety_check(store: secrets::EncryptedFileStore) {
    let _: std::sync::Arc<dyn keyring_core::api::CredentialStoreApi + Send + Sync> =
        std::sync::Arc::new(store);
}

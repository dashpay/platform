//! Storage backends for the `platform-wallet` crate.
//!
//! Today this crate ships the SQLite-backed
//! [`sqlite::SqlitePersister`] implementation of
//! [`PlatformWalletPersistence`](platform_wallet::changeset::PlatformWalletPersistence).
//! The crate is structured so a future `secrets` submodule — a
//! `SecretStore` for mnemonic / private-key material, sketched in
//! [`SECRETS.md`](../SECRETS.md) — can ship alongside it without a
//! crate split.
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

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "secrets")]
pub mod secrets;

// Convenience re-exports kept under the crate root so embedders don't
// have to spell out the `::sqlite::` middle segment for the common
// names. Adding to or trimming from this list does NOT count as a
// breaking change of the submodule API.
#[cfg(feature = "sqlite")]
#[allow(deprecated)]
pub use sqlite::{
    AutoBackupOperation, DeleteWalletReport, FlushMode, JournalMode, PruneReport, RetentionPolicy,
    SqlitePersister, SqlitePersisterConfig, SqlitePersisterError, Synchronous, WalletStorageError,
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

// `SecretStore` must be object-safe and its error `Send + Sync`, so a
// backend can be held behind `Arc<dyn SecretStore>` and its errors
// crossed between threads / FFI.
#[cfg(feature = "secrets")]
#[allow(dead_code)]
const fn _secrets_send_sync_check<T: Send + Sync>() {}
#[cfg(feature = "secrets")]
const _: () = {
    _secrets_send_sync_check::<secrets::SecretStoreError>();
};
#[cfg(feature = "secrets")]
#[allow(dead_code)]
fn _secret_store_object_safety_check(store: secrets::EncryptedFileStore) {
    let _: std::sync::Arc<dyn secrets::SecretStore> = std::sync::Arc::new(store);
}

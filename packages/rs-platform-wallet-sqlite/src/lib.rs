//! SQLite-backed implementation of
//! [`PlatformWalletPersistence`](platform_wallet::changeset::PlatformWalletPersistence).
//!
//! See the crate README for the public API tour, the SECRETS.md note
//! for the private-key boundary, and the workflow-feature plan
//! (`1-i-think-we-jazzy-hare.md`) for the full design rationale.

#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod backup;
pub mod buffer;
pub mod config;
pub mod error;
pub mod migrations;
pub mod persister;
pub mod schema;

pub use config::{FlushMode, JournalMode, SqlitePersisterConfig, Synchronous};
pub use error::{AutoBackupOperation, SqlitePersisterError};
pub use persister::{DeleteWalletReport, PruneReport, RetentionPolicy, SqlitePersister};

// Compile-time assertions — TC-076, TC-077, TC-082 enforcement.
//
// TC-076: SqlitePersister: Send + Sync.
// TC-077: SqlitePersister implements PlatformWalletPersistence.
// TC-082: Public error types are concrete (typed enum), never
//         boxed-trait-object errors — enforced by
//         `From<SqlitePersisterError> for PersistenceError` in
//         `error.rs` and audited via the lint test in
//         `tests/persist_roundtrip.rs::tc082_no_box_dyn_error_in_src`.
#[allow(dead_code)]
const fn _send_sync_check<T: Send + Sync>() {}
const _: () = {
    _send_sync_check::<SqlitePersister>();
    _send_sync_check::<SqlitePersisterError>();
};

// Object-safety check at the type-level (TC-078).
#[allow(dead_code)]
fn _object_safety_check(persister: SqlitePersister) {
    let _: std::sync::Arc<dyn platform_wallet::changeset::PlatformWalletPersistence> =
        std::sync::Arc::new(persister);
}

//! SQLite-backed persistence for `platform-wallet`.
//!
//! Implements [`PlatformWalletPersistence`](platform_wallet::changeset::PlatformWalletPersistence)
//! with a per-wallet in-memory buffer, atomic per-wallet flushes, online
//! backup, retention, and a maintenance CLI. The submodules form the
//! internal layout — most callers reach for the re-exports at the crate
//! root instead.

pub mod backup;
pub(crate) mod buffer;
pub mod config;
pub(crate) mod conn;
pub mod error;
#[cfg(feature = "kv")]
pub mod kv;
pub mod persister;
pub mod reports;
pub mod util;

// `schema` and `migrations` exist only to support the persister. They are
// `pub(crate)` in production builds; the `__test-helpers` feature widens
// them to `pub` so this crate's integration tests can drive the
// per-area readers/writers directly.
#[cfg(any(test, feature = "__test-helpers"))]
pub mod migrations;
#[cfg(not(any(test, feature = "__test-helpers")))]
pub(crate) mod migrations;
#[cfg(any(test, feature = "__test-helpers"))]
pub mod schema;
#[cfg(not(any(test, feature = "__test-helpers")))]
pub(crate) mod schema;

// `LoadCtx` is an input to those same readers, so it is public exactly
// when a caller that can pass one is: this crate's tests, and the
// `rehydration-apply` consumer whose `apply_persisted_core_state` takes
// one. `LoadSite` and `LoadDegradation` stay public unconditionally —
// `last_load_degradation()` returns them.
#[cfg(any(test, feature = "__test-helpers", feature = "rehydration-apply"))]
pub mod load_ctx;
#[cfg(not(any(test, feature = "__test-helpers", feature = "rehydration-apply")))]
pub(crate) mod load_ctx;

pub use config::{
    default_auto_backup_dir, FlushMode, JournalMode, LoadPolicy, SqlitePersisterConfig, Synchronous,
};
pub use error::{AutoBackupOperation, WalletStorageError};
#[cfg(any(test, feature = "__test-helpers", feature = "rehydration-apply"))]
pub use load_ctx::LoadCtx;
pub use load_ctx::{LoadDegradation, LoadSite};
pub use persister::{PruneReport, RetentionPolicy, SqlitePersister};
pub use reports::{CommitReport, DeleteWalletReport};

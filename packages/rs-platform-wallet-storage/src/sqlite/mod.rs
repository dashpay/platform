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
pub mod load_ctx;
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

pub use config::{
    default_auto_backup_dir, FlushMode, JournalMode, LoadPolicy, SqlitePersisterConfig, Synchronous,
};
pub use error::{AutoBackupOperation, WalletStorageError};
pub use load_ctx::{LoadCtx, LoadDegradation, LoadSite};
pub use persister::{PruneReport, RetentionPolicy, SqlitePersister};
pub use reports::{CommitReport, DeleteWalletReport};

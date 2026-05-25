//! SQLite-backed persistence for `platform-wallet`.
//!
//! Implements [`PlatformWalletPersistence`](platform_wallet::changeset::PlatformWalletPersistence)
//! with a per-wallet in-memory buffer, atomic per-wallet flushes, online
//! backup, retention, and a maintenance CLI. The submodules form the
//! internal layout — most callers reach for the re-exports at the crate
//! root instead.

pub mod backup;
pub mod buffer;
pub mod config;
pub(crate) mod conn;
pub mod error;
pub mod migrations;
pub mod persister;
pub mod schema;
pub mod util;

pub use config::{FlushMode, JournalMode, SqlitePersisterConfig, Synchronous};
#[allow(deprecated)]
pub use error::{AutoBackupOperation, SqlitePersisterError, WalletStorageError};
pub use persister::{PruneReport, RetentionPolicy, SqlitePersister};

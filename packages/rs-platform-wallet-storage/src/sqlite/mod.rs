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
pub mod error;
pub mod migrations;
pub mod persister;
pub mod schema;

pub use config::{FlushMode, JournalMode, SqlitePersisterConfig, Synchronous};
pub use error::{AutoBackupOperation, SqlitePersisterError};
pub use persister::{DeleteWalletReport, PruneReport, RetentionPolicy, SqlitePersister};

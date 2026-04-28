//! Optional runtime diagnostics for platform-wallet.
//!
//! Currently a single submodule:
//!
//! - [`instrumented_lock`] — an `InstrumentedRwLock<T>` newtype wrapping
//!   [`tokio::sync::RwLock`] that, when the `lock-stats` Cargo feature
//!   is enabled, records per-call-site acquisition counts plus wait /
//!   hold durations. With the feature off the wrapper compiles down to
//!   the underlying tokio lock (zero added overhead in the hot path).

pub mod instrumented_lock;

pub use instrumented_lock::{
    InstrumentedArcExt, InstrumentedRwLock, InstrumentedRwLockExt, ReadGuard, WriteGuard,
};

#[cfg(feature = "lock-stats")]
pub use instrumented_lock::{LockStats, SiteStats, Snapshot};

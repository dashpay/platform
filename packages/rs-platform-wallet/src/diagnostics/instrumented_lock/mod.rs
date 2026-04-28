//! `InstrumentedRwLock<T>` — opt-in instrumented wrapper around
//! [`tokio::sync::RwLock`].
//!
//! # Build modes
//!
//! - **`lock-stats` feature OFF (default)** — `InstrumentedRwLock<T>` is
//!   a literal type alias for [`tokio::sync::RwLock<T>`]. There is no
//!   wrapper struct, no extra `Arc`, and no `Drop` glue. The tagged
//!   methods (`read_at` / `write_at` / `try_*_at` / `blocking_*_at`)
//!   plus `raw_arc` are provided by zero-cost extension traits
//!   ([`InstrumentedRwLockExt`] and [`InstrumentedArcExt`]) whose
//!   methods are `#[inline]` and drop the tag at the call site. After
//!   inlining, every call collapses to the equivalent inherent
//!   [`tokio::sync::RwLock`] method.
//!
//! - **`lock-stats` feature ON** — `InstrumentedRwLock<T>` is a
//!   wrapper struct holding `Arc<TokioRwLock<T>>` plus
//!   `Arc<`[`LockStats`]`>`. Each acquisition records the wait time
//!   (until the guard resolves) and the hold time (until the guard
//!   drops). The tagged methods bucket the acquisition under the
//!   given tag; untagged calls bucket into [`UNTAGGED`].
//!
//! # Why an inner Arc when the feature is on
//!
//! The wrapper has to hand `Arc<tokio::sync::RwLock<T>>` to APIs that
//! take that concrete type literally (e.g. `dash_spv::DashSpvClient::new`
//! which takes `Arc<RwLock<W>>`). With the feature off the wallet-manager
//! field IS `Arc<TokioRwLock<T>>`, so [`InstrumentedArcExt::raw_arc`]
//! reduces to `Arc::clone(self)`. With the feature on the wrapper holds
//! its tokio lock as `Arc<TokioRwLock<T>>` internally so the same
//! `raw_arc()` call extracts the inner Arc. SPV's own acquisitions go
//! through that inner Arc directly and are NOT seen by the wrapper's
//! stats — the intentional trade is that platform-side acquisitions
//! (everything that goes through `wallet_manager.read()` /
//! `wallet_manager.read_at("…")`) are counted, while upstream's own
//! `process_block` write isn't. That's the right shape for "what does
//! platform-wallet contribute to lock pressure?", which is the question
//! this layer was added to answer.
//!
//! # Tagged call sites
//!
//! Untagged calls (`lock.read().await`) bucket into [`UNTAGGED`] —
//! a useful aggregate but not actionable when you're trying to find
//! the specific code path serializing the lock. Tagging individual
//! sites (`lock.read_at("event_adapter::is_chain_locked").await`) is
//! the path to actionable contention numbers. The tag is `&'static str`
//! so it doesn't allocate on the hot path; with the feature off the
//! tag is dropped at the `read_at` boundary and the call collapses
//! into a plain `read().await`.
//!
//! # Snapshot shape
//!
//! With `lock-stats` enabled, calling [`InstrumentedRwLock::stats`]
//! hands back the shared `Arc<LockStats>`. From there
//! [`LockStats::snapshot`] produces a [`Snapshot`] containing the
//! global counters and the per-tag breakdown — clone the snapshot
//! wherever you need to print or log it (e.g. an FFI accessor or a
//! periodic `tracing::info!`).

#![allow(unused_imports)] // some imports are only used under one cfg branch

use std::future::Future;
use std::sync::Arc;

use tokio::sync::{
    RwLock as TokioRwLock, RwLockReadGuard as TokioReadGuard, RwLockWriteGuard as TokioWriteGuard,
    TryLockError,
};

#[cfg(feature = "lock-stats")]
mod stats;

#[cfg(feature = "lock-stats")]
pub use stats::{LockStats, SiteStats, Snapshot};

#[cfg(feature = "lock-stats")]
use std::time::Instant;

/// The default tag attributed to acquisitions made through the
/// un-suffixed methods (`read`, `write`, `try_read`, …). Visible in
/// [`LockStats`] snapshots so it's clear which acquisitions came
/// from un-tagged sites.
pub const UNTAGGED: &str = "untagged";

// ---------------------------------------------------------------------------
// Extension traits
//
// Defined unconditionally so call sites import them once and stay agnostic
// to the feature flag. The impls differ per cfg branch — see below.
// ---------------------------------------------------------------------------

/// Tagged-acquisition methods on a `RwLock`-shaped lock.
///
/// In feature-off mode the impl forwards to the corresponding tokio
/// inherent method and drops the tag. In feature-on mode the impl
/// forwards to the wrapper's inherent method, which records the
/// acquisition under `tag`.
pub trait InstrumentedRwLockExt<T: Send + Sync + 'static> {
    /// Acquire a shared lock, attributing the acquisition to `tag`
    /// (when `lock-stats` is enabled).
    fn read_at(&self, tag: &'static str) -> impl Future<Output = ReadGuard<'_, T>> + Send;
    /// Acquire an exclusive lock, attributing the acquisition to `tag`.
    fn write_at(&self, tag: &'static str) -> impl Future<Output = WriteGuard<'_, T>> + Send;
    /// Try to acquire a shared lock without waiting.
    fn try_read_at(&self, tag: &'static str) -> Result<ReadGuard<'_, T>, TryLockError>;
    /// Try to acquire an exclusive lock without waiting.
    fn try_write_at(&self, tag: &'static str) -> Result<WriteGuard<'_, T>, TryLockError>;
    /// Synchronously acquire a shared lock — must NOT be called from a
    /// tokio runtime thread (will panic).
    fn blocking_read_at(&self, tag: &'static str) -> ReadGuard<'_, T>;
    /// Synchronously acquire an exclusive lock — must NOT be called
    /// from a tokio runtime thread.
    fn blocking_write_at(&self, tag: &'static str) -> WriteGuard<'_, T>;
}

/// `raw_arc()` extension on a shared handle to the lock. Returns the
/// `Arc<TokioRwLock<T>>` shape that external APIs take literally
/// (e.g. `dash_spv::DashSpvClient::new`). With the feature off the
/// handle IS the tokio Arc, so this is `Arc::clone(self)`. With the
/// feature on the handle is `Arc<Wrapper>` and this extracts the
/// wrapper's inner Arc.
pub trait InstrumentedArcExt<T> {
    /// Cheap clone of the underlying `Arc<tokio::sync::RwLock<T>>`.
    /// Acquisitions made through the returned Arc bypass the wrapper's
    /// stats — see the module-level docs for the rationale.
    fn raw_arc(&self) -> Arc<TokioRwLock<T>>;
}

// ---------------------------------------------------------------------------
// Feature OFF: type aliases — the wrapper IS tokio's RwLock.
// ---------------------------------------------------------------------------

#[cfg(not(feature = "lock-stats"))]
mod alias_mode {
    use super::*;

    /// Type alias for [`tokio::sync::RwLock<T>`] when `lock-stats` is
    /// off. No wrapper struct, no extra `Arc`, no per-method
    /// instrumentation cost.
    pub type InstrumentedRwLock<T> = TokioRwLock<T>;

    /// Type alias for [`tokio::sync::RwLockReadGuard<'a, T>`] when
    /// `lock-stats` is off.
    pub type ReadGuard<'a, T> = TokioReadGuard<'a, T>;

    /// Type alias for [`tokio::sync::RwLockWriteGuard<'a, T>`] when
    /// `lock-stats` is off.
    pub type WriteGuard<'a, T> = TokioWriteGuard<'a, T>;

    impl<T: Send + Sync + 'static> InstrumentedRwLockExt<T> for TokioRwLock<T> {
        #[inline]
        fn read_at(&self, _tag: &'static str) -> impl Future<Output = ReadGuard<'_, T>> + Send {
            self.read()
        }

        #[inline]
        fn write_at(&self, _tag: &'static str) -> impl Future<Output = WriteGuard<'_, T>> + Send {
            self.write()
        }

        #[inline]
        fn try_read_at(&self, _tag: &'static str) -> Result<ReadGuard<'_, T>, TryLockError> {
            self.try_read()
        }

        #[inline]
        fn try_write_at(&self, _tag: &'static str) -> Result<WriteGuard<'_, T>, TryLockError> {
            self.try_write()
        }

        #[inline]
        fn blocking_read_at(&self, _tag: &'static str) -> ReadGuard<'_, T> {
            self.blocking_read()
        }

        #[inline]
        fn blocking_write_at(&self, _tag: &'static str) -> WriteGuard<'_, T> {
            self.blocking_write()
        }
    }

    impl<T> InstrumentedArcExt<T> for Arc<TokioRwLock<T>> {
        #[inline]
        fn raw_arc(&self) -> Arc<TokioRwLock<T>> {
            Arc::clone(self)
        }
    }
}

#[cfg(not(feature = "lock-stats"))]
pub use alias_mode::{InstrumentedRwLock, ReadGuard, WriteGuard};

// ---------------------------------------------------------------------------
// Feature ON: full wrapper struct with stats.
// ---------------------------------------------------------------------------

#[cfg(feature = "lock-stats")]
mod struct_mode {
    use super::*;
    use std::ops::{Deref, DerefMut};

    /// Wrapper around [`tokio::sync::RwLock<T>`] that records
    /// per-call-site acquisition counts plus wait and hold durations.
    /// See the module-level docs.
    pub struct InstrumentedRwLock<T> {
        inner: Arc<TokioRwLock<T>>,
        stats: Arc<LockStats>,
    }

    impl<T> InstrumentedRwLock<T> {
        /// Construct a new lock holding `value`.
        pub fn new(value: T) -> Self {
            Self {
                inner: Arc::new(TokioRwLock::new(value)),
                stats: Arc::new(LockStats::new()),
            }
        }

        /// Borrow the wrapped tokio lock. Use only for APIs that
        /// genuinely need a `&TokioRwLock<T>`; prefer the wrapper's
        /// own methods so acquisitions stay attributed.
        #[inline]
        pub fn raw(&self) -> &TokioRwLock<T> {
            &self.inner
        }

        /// Cheap clone of the inner `Arc<TokioRwLock<T>>`. See the
        /// [`InstrumentedArcExt::raw_arc`] doc for the trade.
        #[inline]
        pub fn raw_arc(&self) -> Arc<TokioRwLock<T>> {
            Arc::clone(&self.inner)
        }

        /// Shared handle to the per-lock stats snapshot store.
        #[inline]
        pub fn stats(&self) -> Arc<LockStats> {
            Arc::clone(&self.stats)
        }

        /// Acquire a shared lock — buckets into [`UNTAGGED`].
        #[inline]
        pub async fn read(&self) -> ReadGuard<'_, T> {
            self.read_at(UNTAGGED).await
        }

        /// Acquire an exclusive lock — buckets into [`UNTAGGED`].
        #[inline]
        pub async fn write(&self) -> WriteGuard<'_, T> {
            self.write_at(UNTAGGED).await
        }

        /// Acquire a shared lock with a per-call-site tag.
        pub async fn read_at(&self, tag: &'static str) -> ReadGuard<'_, T> {
            let wait_start = Instant::now();
            let inner = self.inner.read().await;
            let wait_ns = wait_start.elapsed().as_nanos() as u64;
            self.stats.record_read_acquired(tag, wait_ns);
            ReadGuard {
                inner,
                stats: Arc::clone(&self.stats),
                tag,
                acquired_at: Instant::now(),
            }
        }

        /// Acquire an exclusive lock with a per-call-site tag.
        pub async fn write_at(&self, tag: &'static str) -> WriteGuard<'_, T> {
            let wait_start = Instant::now();
            let inner = self.inner.write().await;
            let wait_ns = wait_start.elapsed().as_nanos() as u64;
            self.stats.record_write_acquired(tag, wait_ns);
            WriteGuard {
                inner,
                stats: Arc::clone(&self.stats),
                tag,
                acquired_at: Instant::now(),
            }
        }

        /// Try to acquire a shared lock without waiting.
        #[inline]
        pub fn try_read(&self) -> Result<ReadGuard<'_, T>, TryLockError> {
            self.try_read_at(UNTAGGED)
        }

        /// Try to acquire an exclusive lock without waiting.
        #[inline]
        pub fn try_write(&self) -> Result<WriteGuard<'_, T>, TryLockError> {
            self.try_write_at(UNTAGGED)
        }

        /// Tagged variant of [`try_read`](Self::try_read).
        pub fn try_read_at(&self, tag: &'static str) -> Result<ReadGuard<'_, T>, TryLockError> {
            match self.inner.try_read() {
                Ok(inner) => {
                    self.stats.record_read_acquired(tag, 0);
                    Ok(ReadGuard {
                        inner,
                        stats: Arc::clone(&self.stats),
                        tag,
                        acquired_at: Instant::now(),
                    })
                }
                Err(e) => {
                    self.stats.record_read_contended(tag);
                    Err(e)
                }
            }
        }

        /// Tagged variant of [`try_write`](Self::try_write).
        pub fn try_write_at(&self, tag: &'static str) -> Result<WriteGuard<'_, T>, TryLockError> {
            match self.inner.try_write() {
                Ok(inner) => {
                    self.stats.record_write_acquired(tag, 0);
                    Ok(WriteGuard {
                        inner,
                        stats: Arc::clone(&self.stats),
                        tag,
                        acquired_at: Instant::now(),
                    })
                }
                Err(e) => {
                    self.stats.record_write_contended(tag);
                    Err(e)
                }
            }
        }

        /// Synchronously acquire a shared lock — must NOT be called from
        /// a tokio runtime thread.
        #[inline]
        pub fn blocking_read(&self) -> ReadGuard<'_, T> {
            self.blocking_read_at(UNTAGGED)
        }

        /// Synchronously acquire an exclusive lock — must NOT be called
        /// from a tokio runtime thread.
        #[inline]
        pub fn blocking_write(&self) -> WriteGuard<'_, T> {
            self.blocking_write_at(UNTAGGED)
        }

        /// Tagged variant of [`blocking_read`](Self::blocking_read).
        pub fn blocking_read_at(&self, tag: &'static str) -> ReadGuard<'_, T> {
            let wait_start = Instant::now();
            let inner = self.inner.blocking_read();
            let wait_ns = wait_start.elapsed().as_nanos() as u64;
            self.stats.record_read_acquired(tag, wait_ns);
            ReadGuard {
                inner,
                stats: Arc::clone(&self.stats),
                tag,
                acquired_at: Instant::now(),
            }
        }

        /// Tagged variant of [`blocking_write`](Self::blocking_write).
        pub fn blocking_write_at(&self, tag: &'static str) -> WriteGuard<'_, T> {
            let wait_start = Instant::now();
            let inner = self.inner.blocking_write();
            let wait_ns = wait_start.elapsed().as_nanos() as u64;
            self.stats.record_write_acquired(tag, wait_ns);
            WriteGuard {
                inner,
                stats: Arc::clone(&self.stats),
                tag,
                acquired_at: Instant::now(),
            }
        }
    }

    impl<T: Default> Default for InstrumentedRwLock<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }

    impl<T: std::fmt::Debug> std::fmt::Debug for InstrumentedRwLock<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("InstrumentedRwLock")
                .field("inner", &self.inner)
                .finish()
        }
    }

    /// Shared read guard. `Deref<Target = T>`. Records hold time on
    /// `Drop`.
    pub struct ReadGuard<'a, T> {
        inner: TokioReadGuard<'a, T>,
        stats: Arc<LockStats>,
        tag: &'static str,
        acquired_at: Instant,
    }

    impl<T> Deref for ReadGuard<'_, T> {
        type Target = T;

        #[inline]
        fn deref(&self) -> &T {
            &self.inner
        }
    }

    impl<T> Drop for ReadGuard<'_, T> {
        fn drop(&mut self) {
            let held_ns = self.acquired_at.elapsed().as_nanos() as u64;
            self.stats.record_read_released(self.tag, held_ns);
        }
    }

    /// Exclusive write guard. `Deref<Target = T>` + `DerefMut`. Records
    /// hold time on `Drop`.
    pub struct WriteGuard<'a, T> {
        inner: TokioWriteGuard<'a, T>,
        stats: Arc<LockStats>,
        tag: &'static str,
        acquired_at: Instant,
    }

    impl<T> Deref for WriteGuard<'_, T> {
        type Target = T;

        #[inline]
        fn deref(&self) -> &T {
            &self.inner
        }
    }

    impl<T> DerefMut for WriteGuard<'_, T> {
        #[inline]
        fn deref_mut(&mut self) -> &mut T {
            &mut self.inner
        }
    }

    impl<T> Drop for WriteGuard<'_, T> {
        fn drop(&mut self) {
            let held_ns = self.acquired_at.elapsed().as_nanos() as u64;
            self.stats.record_write_released(self.tag, held_ns);
        }
    }

    // Trait impls so feature-agnostic call sites that import the Ext
    // traits keep working. Method resolution prefers inherent methods
    // when they exist, so these are mostly redundant in feature-on
    // mode — they're here so a generic helper bounded by
    // `InstrumentedRwLockExt` can take the wrapper just as easily as
    // it can take a raw `TokioRwLock`.
    impl<T: Send + Sync + 'static> InstrumentedRwLockExt<T> for InstrumentedRwLock<T> {
        #[inline]
        fn read_at(&self, tag: &'static str) -> impl Future<Output = ReadGuard<'_, T>> + Send {
            InstrumentedRwLock::read_at(self, tag)
        }

        #[inline]
        fn write_at(&self, tag: &'static str) -> impl Future<Output = WriteGuard<'_, T>> + Send {
            InstrumentedRwLock::write_at(self, tag)
        }

        #[inline]
        fn try_read_at(&self, tag: &'static str) -> Result<ReadGuard<'_, T>, TryLockError> {
            InstrumentedRwLock::try_read_at(self, tag)
        }

        #[inline]
        fn try_write_at(&self, tag: &'static str) -> Result<WriteGuard<'_, T>, TryLockError> {
            InstrumentedRwLock::try_write_at(self, tag)
        }

        #[inline]
        fn blocking_read_at(&self, tag: &'static str) -> ReadGuard<'_, T> {
            InstrumentedRwLock::blocking_read_at(self, tag)
        }

        #[inline]
        fn blocking_write_at(&self, tag: &'static str) -> WriteGuard<'_, T> {
            InstrumentedRwLock::blocking_write_at(self, tag)
        }
    }

    impl<T> InstrumentedArcExt<T> for Arc<InstrumentedRwLock<T>> {
        #[inline]
        fn raw_arc(&self) -> Arc<TokioRwLock<T>> {
            InstrumentedRwLock::raw_arc(self)
        }
    }
}

#[cfg(feature = "lock-stats")]
pub use struct_mode::{InstrumentedRwLock, ReadGuard, WriteGuard};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn read_write_smoke() {
        let lock = InstrumentedRwLock::new(0u32);
        {
            let guard = lock.read().await;
            assert_eq!(*guard, 0);
        }
        {
            let mut guard = lock.write().await;
            *guard = 42;
        }
        let guard = lock.read().await;
        assert_eq!(*guard, 42);
    }

    #[tokio::test]
    async fn try_read_contended() {
        let lock = InstrumentedRwLock::new(0u32);
        let _w = lock.write().await;
        assert!(lock.try_read().is_err());
    }

    #[tokio::test]
    async fn read_at_smoke() {
        // Tagged calls work in both feature modes — feature-off via the
        // `InstrumentedRwLockExt` trait, feature-on via inherent methods.
        let lock = InstrumentedRwLock::new(0u32);
        let guard = lock.read_at("test::tag").await;
        assert_eq!(*guard, 0);
    }

    #[tokio::test]
    async fn raw_arc_smoke() {
        // `raw_arc()` works in both modes — feature-off via the
        // `InstrumentedArcExt` trait on `Arc<TokioRwLock>`, feature-on
        // via the wrapper's inherent method.
        let lock = Arc::new(InstrumentedRwLock::new(0u32));
        let raw: Arc<TokioRwLock<u32>> = lock.raw_arc();
        let guard = raw.read().await;
        assert_eq!(*guard, 0);
    }

    #[cfg(feature = "lock-stats")]
    #[tokio::test]
    async fn stats_count_and_attribute_to_tag() {
        let lock = InstrumentedRwLock::new(0u32);

        // Two reads tagged "ours", one write tagged "theirs".
        {
            let _r1 = lock.read_at("ours").await;
            let _r2 = lock.read_at("ours").await;
        }
        {
            let _w = lock.write_at("theirs").await;
        }

        let snap = lock.stats().snapshot();
        let ours = snap.per_tag.get("ours").expect("ours tag present");
        assert_eq!(ours.read_acquired, 2);
        assert_eq!(ours.write_acquired, 0);
        let theirs = snap.per_tag.get("theirs").expect("theirs tag present");
        assert_eq!(theirs.read_acquired, 0);
        assert_eq!(theirs.write_acquired, 1);
        assert_eq!(snap.total.read_acquired, 2);
        assert_eq!(snap.total.write_acquired, 1);
    }

    // Untagged calls go to the UNTAGGED bucket so the snapshot still
    // accounts for them — we don't want acquisitions to vanish.
    #[cfg(feature = "lock-stats")]
    #[tokio::test]
    async fn untagged_calls_go_to_untagged_bucket() {
        let lock = InstrumentedRwLock::new(0u32);
        {
            let _r = lock.read().await;
        }
        let snap = lock.stats().snapshot();
        let untagged = snap.per_tag.get(UNTAGGED).expect("UNTAGGED bucket");
        assert_eq!(untagged.read_acquired, 1);
    }

    #[cfg(feature = "lock-stats")]
    #[tokio::test]
    async fn try_read_failure_records_contention() {
        let lock = InstrumentedRwLock::new(0u32);
        let _w = lock.write_at("holder").await;
        let r = lock.try_read_at("contender");
        assert!(r.is_err());
        let snap = lock.stats().snapshot();
        let contender = snap.per_tag.get("contender").expect("contender tag");
        assert_eq!(contender.read_contended, 1);
        assert_eq!(contender.read_acquired, 0);
    }
}

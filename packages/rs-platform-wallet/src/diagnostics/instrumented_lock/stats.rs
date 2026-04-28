//! Stats counters for [`super::InstrumentedRwLock`]. Only compiled when
//! the `lock-stats` Cargo feature is enabled.
//!
//! # Storage shape
//!
//! - **Total counters** — atomic `u64`s, lock-free, bumped from every
//!   acquire / release path. Cheap enough that the bump is in the
//!   noise even under heavy contention.
//! - **Per-tag breakdown** — `BTreeMap<&'static str, SiteStats>` behind
//!   a `parking_lot::Mutex`. The map is touched only on the lock
//!   acquire / release boundary (never across an `.await`), so a sync
//!   mutex is appropriate. The acquire path holds the mutex just long
//!   enough to look up or insert the entry, then bumps the entry's
//!   atomics outside the lock. New tags are inserted lazily on first
//!   use; existing tags re-use the entry.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

/// Aggregate lock-acquisition counters maintained by an
/// [`InstrumentedRwLock`](super::InstrumentedRwLock).
///
/// Use [`LockStats::snapshot`] to clone out a readable [`Snapshot`].
/// The live [`LockStats`] keeps its counters as atomics so reads via
/// `snapshot` don't race with writers.
#[derive(Debug)]
pub struct LockStats {
    total: SiteCounters,
    per_tag: Mutex<BTreeMap<&'static str, Arc<SiteCounters>>>,
}

impl LockStats {
    pub(super) fn new() -> Self {
        Self {
            total: SiteCounters::new(),
            per_tag: Mutex::new(BTreeMap::new()),
        }
    }

    /// Take a snapshot of the current counters. Cheap enough to call
    /// from a debug UI on every refresh; a periodic logger could call
    /// it on a 1-second timer without measurable overhead.
    pub fn snapshot(&self) -> Snapshot {
        let per_tag: BTreeMap<&'static str, SiteStats> = self
            .per_tag
            .lock()
            .iter()
            .map(|(tag, counters)| (*tag, counters.snapshot()))
            .collect();
        Snapshot {
            total: self.total.snapshot(),
            per_tag,
        }
    }

    /// Look up (or insert on first use) the [`SiteCounters`] for a tag.
    /// Holds the per-tag mutex only for the lookup; the returned `Arc`
    /// lets the caller bump atomics outside the mutex.
    fn site(&self, tag: &'static str) -> Arc<SiteCounters> {
        let mut guard = self.per_tag.lock();
        if let Some(existing) = guard.get(tag) {
            return Arc::clone(existing);
        }
        let new = Arc::new(SiteCounters::new());
        guard.insert(tag, Arc::clone(&new));
        new
    }

    pub(super) fn record_read_acquired(&self, tag: &'static str, wait_ns: u64) {
        self.total.read_acquired.fetch_add(1, Ordering::Relaxed);
        self.total
            .read_wait_ns_total
            .fetch_add(wait_ns, Ordering::Relaxed);
        let site = self.site(tag);
        site.read_acquired.fetch_add(1, Ordering::Relaxed);
        site.read_wait_ns_total
            .fetch_add(wait_ns, Ordering::Relaxed);
    }

    pub(super) fn record_read_released(&self, tag: &'static str, held_ns: u64) {
        self.total
            .read_hold_ns_total
            .fetch_add(held_ns, Ordering::Relaxed);
        let site = self.site(tag);
        site.read_hold_ns_total
            .fetch_add(held_ns, Ordering::Relaxed);
    }

    pub(super) fn record_read_contended(&self, tag: &'static str) {
        self.total.read_contended.fetch_add(1, Ordering::Relaxed);
        let site = self.site(tag);
        site.read_contended.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn record_write_acquired(&self, tag: &'static str, wait_ns: u64) {
        self.total.write_acquired.fetch_add(1, Ordering::Relaxed);
        self.total
            .write_wait_ns_total
            .fetch_add(wait_ns, Ordering::Relaxed);
        let site = self.site(tag);
        site.write_acquired.fetch_add(1, Ordering::Relaxed);
        site.write_wait_ns_total
            .fetch_add(wait_ns, Ordering::Relaxed);
    }

    pub(super) fn record_write_released(&self, tag: &'static str, held_ns: u64) {
        self.total
            .write_hold_ns_total
            .fetch_add(held_ns, Ordering::Relaxed);
        let site = self.site(tag);
        site.write_hold_ns_total
            .fetch_add(held_ns, Ordering::Relaxed);
    }

    pub(super) fn record_write_contended(&self, tag: &'static str) {
        self.total.write_contended.fetch_add(1, Ordering::Relaxed);
        let site = self.site(tag);
        site.write_contended.fetch_add(1, Ordering::Relaxed);
    }
}

/// Live atomic counters for a single bucket (the global "total" plus
/// each per-tag site).
#[derive(Debug)]
struct SiteCounters {
    read_acquired: AtomicU64,
    write_acquired: AtomicU64,
    read_contended: AtomicU64,
    write_contended: AtomicU64,
    read_wait_ns_total: AtomicU64,
    write_wait_ns_total: AtomicU64,
    read_hold_ns_total: AtomicU64,
    write_hold_ns_total: AtomicU64,
}

impl SiteCounters {
    fn new() -> Self {
        Self {
            read_acquired: AtomicU64::new(0),
            write_acquired: AtomicU64::new(0),
            read_contended: AtomicU64::new(0),
            write_contended: AtomicU64::new(0),
            read_wait_ns_total: AtomicU64::new(0),
            write_wait_ns_total: AtomicU64::new(0),
            read_hold_ns_total: AtomicU64::new(0),
            write_hold_ns_total: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> SiteStats {
        SiteStats {
            read_acquired: self.read_acquired.load(Ordering::Relaxed),
            write_acquired: self.write_acquired.load(Ordering::Relaxed),
            read_contended: self.read_contended.load(Ordering::Relaxed),
            write_contended: self.write_contended.load(Ordering::Relaxed),
            read_wait_ns_total: self.read_wait_ns_total.load(Ordering::Relaxed),
            write_wait_ns_total: self.write_wait_ns_total.load(Ordering::Relaxed),
            read_hold_ns_total: self.read_hold_ns_total.load(Ordering::Relaxed),
            write_hold_ns_total: self.write_hold_ns_total.load(Ordering::Relaxed),
        }
    }
}

/// Plain-old-data snapshot of a single bucket (the global total or a
/// single tag). All durations are in nanoseconds; cumulative across
/// every acquisition since the lock was created.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteStats {
    /// Number of times a read guard was successfully acquired.
    pub read_acquired: u64,
    /// Number of times a write guard was successfully acquired.
    pub write_acquired: u64,
    /// Number of times a `try_read` returned `Err(TryLockError)`.
    pub read_contended: u64,
    /// Number of times a `try_write` returned `Err(TryLockError)`.
    pub write_contended: u64,
    /// Cumulative wait time before read acquisitions resolved, in ns.
    pub read_wait_ns_total: u64,
    /// Cumulative wait time before write acquisitions resolved, in ns.
    pub write_wait_ns_total: u64,
    /// Cumulative time read guards were held before drop, in ns.
    pub read_hold_ns_total: u64,
    /// Cumulative time write guards were held before drop, in ns.
    pub write_hold_ns_total: u64,
}

impl SiteStats {
    /// Mean wait time for read acquisitions, in nanoseconds. Returns
    /// `None` if no read acquisitions have completed.
    pub fn read_wait_ns_mean(&self) -> Option<u64> {
        if self.read_acquired == 0 {
            None
        } else {
            Some(self.read_wait_ns_total / self.read_acquired)
        }
    }

    /// Mean wait time for write acquisitions, in nanoseconds.
    pub fn write_wait_ns_mean(&self) -> Option<u64> {
        if self.write_acquired == 0 {
            None
        } else {
            Some(self.write_wait_ns_total / self.write_acquired)
        }
    }

    /// Mean hold time for read acquisitions, in nanoseconds.
    pub fn read_hold_ns_mean(&self) -> Option<u64> {
        if self.read_acquired == 0 {
            None
        } else {
            Some(self.read_hold_ns_total / self.read_acquired)
        }
    }

    /// Mean hold time for write acquisitions, in nanoseconds.
    pub fn write_hold_ns_mean(&self) -> Option<u64> {
        if self.write_acquired == 0 {
            None
        } else {
            Some(self.write_hold_ns_total / self.write_acquired)
        }
    }
}

/// Snapshot of a [`LockStats`]: aggregate totals plus the per-tag
/// breakdown. Cheap to clone; suitable for shipping through a debug
/// UI or a periodic log line.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Aggregate counters across every tag (including `UNTAGGED`).
    pub total: SiteStats,
    /// Per-tag breakdown. Tags that have never been used don't appear.
    pub per_tag: BTreeMap<&'static str, SiteStats>,
}

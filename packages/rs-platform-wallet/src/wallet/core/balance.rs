//! Lock-free wallet balance using atomics.
//!
//! Updated from `ManagedWalletInfo` on every SPV block/mempool processing
//! and RPC refresh. Readable from any context without locking.

use std::sync::atomic::{AtomicU64, Ordering};

/// Lock-free wallet balance — readable from any context (sync UI, async
/// backend) without acquiring a lock on `ManagedWalletInfo`.
///
/// Updated automatically after SPV block processing, mempool transaction
/// detection, and RPC balance refresh via [`CoreWallet::refresh_balance`].
#[derive(Debug)]
pub struct WalletBalance {
    confirmed: AtomicU64,
    unconfirmed: AtomicU64,
    immature: AtomicU64,
    locked: AtomicU64,
}

impl Clone for WalletBalance {
    fn clone(&self) -> Self {
        Self {
            confirmed: AtomicU64::new(self.confirmed.load(Ordering::Relaxed)),
            unconfirmed: AtomicU64::new(self.unconfirmed.load(Ordering::Relaxed)),
            immature: AtomicU64::new(self.immature.load(Ordering::Relaxed)),
            locked: AtomicU64::new(self.locked.load(Ordering::Relaxed)),
        }
    }
}

impl Default for WalletBalance {
    fn default() -> Self {
        Self::new()
    }
}

impl WalletBalance {
    pub fn new() -> Self {
        Self {
            confirmed: AtomicU64::new(0),
            unconfirmed: AtomicU64::new(0),
            immature: AtomicU64::new(0),
            locked: AtomicU64::new(0),
        }
    }

    pub fn confirmed(&self) -> u64 {
        self.confirmed.load(Ordering::Relaxed)
    }

    pub fn unconfirmed(&self) -> u64 {
        self.unconfirmed.load(Ordering::Relaxed)
    }

    pub fn immature(&self) -> u64 {
        self.immature.load(Ordering::Relaxed)
    }

    pub fn locked(&self) -> u64 {
        self.locked.load(Ordering::Relaxed)
    }

    pub fn total(&self) -> u64 {
        self.confirmed() + self.unconfirmed() + self.immature() + self.locked()
    }

    /// Set from raw values (used by `BalanceUpdateHandler` from event data).
    pub(crate) fn set(&self, confirmed: u64, unconfirmed: u64, immature: u64, locked: u64) {
        self.confirmed.store(confirmed, Ordering::Relaxed);
        self.unconfirmed.store(unconfirmed, Ordering::Relaxed);
        self.immature.store(immature, Ordering::Relaxed);
        self.locked.store(locked, Ordering::Relaxed);
    }
}

//! What this process knows of the flushes of each wallet's records.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use crate::changeset::traits::{PersistenceError, PlatformWalletPersistence};
use crate::wallet::platform_wallet::WalletId;

/// What this process knows of the flushes of one wallet's records.
///
/// A backend buffers a wallet's records under its id, whoever stored them,
/// and a flush that fails for good drops that buffer, whoever flushed. So
/// what a flush did to the buffer is known only where every flush of the id
/// is counted: one ledger per wallet id, shared by the manager and by every
/// handle of a wallet under that id, a removed wallet's handles and its
/// same-id successor's alike ([`FlushLedgers`]). Every flush of the wallet's
/// records goes through [`Self::flush`].
#[derive(Debug, Default)]
pub struct FlushLedger {
    /// How many flushes are in progress: each is counted from before it calls
    /// the backend until its outcome is recorded in `lost`, so while none is,
    /// `lost` tells of every flush that failed for good.
    in_flight: AtomicUsize,
    /// How many flushes failed for good, the backend dropping what it
    /// buffered.
    lost: AtomicU64,
}

/// One flush in progress, counted in its [`FlushLedger`] until dropped.
struct InFlight<'a>(&'a AtomicUsize);

impl<'a> InFlight<'a> {
    fn enter(in_flight: &'a AtomicUsize) -> Self {
        in_flight.fetch_add(1, Ordering::SeqCst);
        Self(in_flight)
    }
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl FlushLedger {
    /// Flush `wallet_id`'s records on `backend`: in progress from before the
    /// backend is called until its outcome is recorded, and counted as lost
    /// when it fails for good (anything but a transient failure, which keeps
    /// the buffer).
    pub(crate) fn flush(
        &self,
        backend: &dyn PlatformWalletPersistence,
        wallet_id: WalletId,
    ) -> Result<(), PersistenceError> {
        let _in_flight = InFlight::enter(&self.in_flight);
        let flushed = backend.flush(wallet_id);
        if flushed.as_ref().is_err_and(|error| !error.is_transient()) {
            self.lost.fetch_add(1, Ordering::SeqCst);
        }
        flushed
    }

    /// How many flushes are in progress.
    pub(crate) fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }

    /// How many flushes failed for good so far.
    pub(crate) fn lost(&self) -> u64 {
        self.lost.load(Ordering::SeqCst)
    }
}

/// The [`FlushLedger`] of each wallet id, for the wallets of one manager. An
/// entry outlives its wallet, as the backend's buffer under the id does.
#[derive(Debug, Default)]
pub struct FlushLedgers(Mutex<BTreeMap<WalletId, Arc<FlushLedger>>>);

impl FlushLedgers {
    /// The ledger of `wallet_id`'s records, made on first use.
    pub fn of(&self, wallet_id: WalletId) -> Arc<FlushLedger> {
        Arc::clone(
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(wallet_id)
                .or_default(),
        )
    }
}

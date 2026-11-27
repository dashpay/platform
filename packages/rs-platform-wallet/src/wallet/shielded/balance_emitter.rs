//! Pushes each wallet's local shielded balance state to the host whenever it
//! changes.
//!
//! One emitter runs per [`NetworkShieldedCoordinator`]. It watches the
//! coordinator's balance change signal, which the store raises on every
//! mutation of its per-subwallet state and the coordinator raises when its
//! account registry or a wallet's hydration changes. On every wake it reads
//! [`NetworkShieldedCoordinator::local_balance_snapshot`] for every wallet
//! registered now or updated before, and hands the sink an update for each
//! wallet whose state differs from the one it last handed over. Emission
//! therefore does not depend on any operation announcing itself: a scan that
//! fails, a rebind, a partial Clear and a note reservation released by a
//! dropped spend all move the signal, and the comparison decides whether the
//! host hears about it.
//!
//! The read waits for the store rather than giving up: a writer (a scan's
//! append or commit, a spend) holds the store write lock, and the update
//! comes out once it releases it, whatever the writer's outcome. The read takes the locks in the
//! documented order (`lifecycle` -> `store`) and releases them before the sink
//! runs, so the emitter never holds a lock across a host callback and cannot
//! deadlock with Clear, unregister or a bind. Each pass reads the wallets that
//! are no longer registered first: their `Unbound` needs no store read, so it
//! is not held up behind a bound wallet's read while a writer holds the store.
//!
//! A ledger the read cannot sum (one account's stored notes past `u64`) is
//! delivered as [`ShieldedLocalBalanceState::RestoreIncomplete`]: not an
//! established ledger. The next successful read replaces it.
//!
//! An update carries the latest state the emitter read. Changes that land
//! between two reads coalesce into one update, or into none when they cancel
//! out (a reservation taken and released before the next read).
//!
//! Stopping: cancelling the token only requests the stop. The emitter checks
//! the token before each sink call, but the check and the call are not one
//! step, so a sink call can still start just after the token fired. The
//! barrier is a completed join of the task: only after it is the sink never
//! called again, and a host must not release what its callback uses on
//! cancellation alone.

use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::balance::{ShieldedBalanceUpdate, ShieldedLocalBalanceState};
use super::coordinator::NetworkShieldedCoordinator;
use crate::wallet::platform_wallet::WalletId;

/// Where the emitter delivers updates. Called on the emitter's task with no
/// lock held, once per update, in revision order per wallet.
pub(crate) type ShieldedBalanceSink = Arc<dyn Fn(&ShieldedBalanceUpdate) + Send + Sync>;

/// Spawn the emitter for `coordinator` on the current tokio runtime. It runs
/// until `cancel` fires; the returned handle joins it. Cancelling requests the
/// stop, and a sink call may still start right after it; once the join has
/// completed the sink is never called again (see the module docs).
pub(crate) fn spawn_balance_emitter(
    coordinator: Arc<NetworkShieldedCoordinator>,
    sink: ShieldedBalanceSink,
    cancel: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(run_balance_emitter(coordinator, sink, cancel))
}

/// The emitter's body (see the module docs). Returns when `cancel` fires.
pub(crate) async fn run_balance_emitter(
    coordinator: Arc<NetworkShieldedCoordinator>,
    sink: ShieldedBalanceSink,
    cancel: CancellationToken,
) {
    let mut changes = coordinator.subscribe_balance_changes();
    // The last update handed over for each wallet. Kept for the life of the
    // emitter, so a wallet that is unregistered and bound again continues
    // its revisions rather than starting over.
    let mut emitted: BTreeMap<WalletId, (u64, ShieldedLocalBalanceState)> = BTreeMap::new();
    loop {
        // Marked seen before the reads: a change that lands while they run
        // wakes the next pass.
        changes.borrow_and_update();
        let registered = tokio::select! {
            biased;
            _ = cancel.cancelled() => return,
            registered = coordinator.registered_wallets() => registered,
        };
        // Wallets no longer registered first (their read needs no store),
        // then the registered ones. Each wallet is read once per pass, so
        // its revisions still go out in order.
        let unregistered: Vec<WalletId> = emitted
            .keys()
            .filter(|wallet_id| !registered.contains(*wallet_id))
            .copied()
            .collect();
        for wallet_id in unregistered.into_iter().chain(registered) {
            let read = tokio::select! {
                biased;
                _ = cancel.cancelled() => return,
                read = coordinator.local_balance_snapshot(wallet_id) => read,
            };
            let state = match read {
                Ok(state) => state,
                Err(error) => {
                    // Only a stored balance past `u64` fails the read, which
                    // valid notes cannot reach. The ledger is then not
                    // established, so it is reported as such: a host showing
                    // an earlier balance must stop showing it as known. The
                    // next successful read replaces it with `Ready`.
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        %error,
                        "shielded balance not readable; reported as restore incomplete"
                    );
                    ShieldedLocalBalanceState::RestoreIncomplete
                }
            };
            let revision = match emitted.get(&wallet_id) {
                Some((_, last)) if *last == state => continue,
                Some((revision, _)) => match revision.checked_add(1) {
                    Some(next) => next,
                    None => {
                        tracing::warn!(
                            wallet_id = %hex::encode(wallet_id),
                            "shielded balance revisions exhausted; no update emitted"
                        );
                        continue;
                    }
                },
                None => 1,
            };
            if cancel.is_cancelled() {
                return;
            }
            let update = ShieldedBalanceUpdate {
                wallet_id,
                revision,
                state,
            };
            sink(&update);
            emitted.insert(wallet_id, (revision, update.state));
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return,
            changed = changes.changed() => {
                // The coordinator holds a sender for as long as it lives, and
                // this task holds the coordinator, so the channel cannot close.
                if changed.is_err() {
                    return;
                }
            }
        }
    }
}

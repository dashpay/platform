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
//! The read waits for the store rather than giving up: a scan holds the store
//! write lock for its whole download, and the update comes out once the scan
//! releases it, whatever the scan's outcome. The read takes the locks in the
//! documented order (`lifecycle` -> `store`) and releases them before the sink
//! runs, so the emitter never holds a lock across a host callback and cannot
//! deadlock with Clear, unregister or a bind.

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
/// until `cancel` fires; the returned handle joins it. No update is delivered
/// once `cancel` has fired, so cancelling and then joining the handle is the
/// barrier after which the sink is never called again.
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
        let mut wallets = registered;
        wallets.extend(emitted.keys().copied());
        for wallet_id in wallets {
            let read = tokio::select! {
                biased;
                _ = cancel.cancelled() => return,
                read = coordinator.local_balance_snapshot(wallet_id) => read,
            };
            let state = match read {
                Ok(state) => state,
                Err(error) => {
                    // Only a stored balance past `u64` fails the read, which
                    // valid notes cannot reach. The host keeps the state it
                    // last received; the next change reads again.
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        %error,
                        "shielded balance not readable; no update emitted"
                    );
                    continue;
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

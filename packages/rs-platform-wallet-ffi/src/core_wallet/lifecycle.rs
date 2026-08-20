//! The generation lifecycle gate, taken on behalf of an already-finalized
//! transaction.
//!
//! # Why this is one function and not three inline blocks
//!
//! Three entry points — `core_wallet_tx_builder_finalize`,
//! `core_wallet_signed_payment_finalize` and
//! `core_wallet_broadcast_signed_transaction` — reach a point where a
//! `SignedCoreTransaction` exists and **holds a UTXO reservation**, but no
//! handle or token has been published to the host yet (or, on the broadcast
//! path, the handle has already been consumed). Between that point and the
//! publish/send they take this wallet generation's lifecycle gate and check
//! that the generation is still live.
//!
//! Acquiring the gate is a guarded call, so it can now fail on its own account
//! (a caught panic, or an async runtime that would not build). When it does,
//! the entry point returns an error — and the reservation it is holding has
//! **nothing left that could ever release it**: the host got no token and no
//! handle, so the inputs stay reserved for the life of the process and the
//! wallet quietly loses spendable balance (`dashpay/platform#4424` review,
//! CodeRabbit + Codex, same finding).
//!
//! So the release lives here, next to the acquisition that can fail, rather
//! than being re-derived correctly at three call sites.
//!
//! # The release is deliberately best-effort
//!
//! `abandon_transaction` runs on the *swallowing* `block_on` (its `()` output
//! absorbs a boundary failure into the guard's `ERROR` log). That is the right
//! shape here and only here: this path is already returning an error of its
//! own, so a second failure has nothing to add and must not mask the first.
//! The release is also generation-bound, so on a genuine teardown it is a
//! logged no-op and on a re-create it correctly declines to touch the new
//! generation's inputs.

use platform_wallet::broadcaster::SpvBroadcaster;
use platform_wallet::{CoreWallet, SignedCoreTransaction};
use tokio::sync::RwLockReadGuard;

use crate::panic_guard::FfiBoundaryError;
use crate::runtime::runtime;

// Fault injection for the release-on-failure tests.
//
// The failure this guards against is a panic *inside* the guarded region,
// which is not otherwise reachable from a test: driving a real entry point
// from inside a runtime context would make the compensating `abandon` panic
// too, and so could never show that the reservation came back. Arming this
// makes only the gate acquisition panic, on the real code path, which is
// exactly the scenario.
//
// `#[cfg(test)]`, so it does not exist in the cdylib.
#[cfg(test)]
thread_local! {
    static PANIC_IN_LIFECYCLE_GATE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Arm a one-shot panic in the next [`lifecycle_gate_or_release`] on this
/// thread.
#[cfg(test)]
pub(crate) fn arm_lifecycle_gate_panic() {
    PANIC_IN_LIFECYCLE_GATE.with(|armed| armed.set(true));
}

/// Take `gate_on`'s generation lifecycle gate and read its liveness, releasing
/// `finalized`'s reservation through `release_on` if the *acquisition itself*
/// fails.
///
/// `gate_on` and `release_on` are separate because the broadcast path gates on
/// the caller-supplied wallet handle but acts through the transaction's own
/// originating wallet; the two finalize paths pass the same wallet twice.
///
/// Returns the held gate plus whether the generation is still registered. The
/// caller keeps the gate alive across its own publish/send so a teardown cannot
/// interleave between the check and the act.
///
/// A `false` liveness result is NOT a failure here — the caller has its own
/// (already correct) reconciliation and error message for that case, so this
/// function leaves it alone.
pub(crate) fn lifecycle_gate_or_release<'a>(
    gate_on: &'a CoreWallet<SpvBroadcaster>,
    release_on: &CoreWallet<SpvBroadcaster>,
    finalized: &SignedCoreTransaction,
) -> Result<(RwLockReadGuard<'a, ()>, bool), FfiBoundaryError> {
    let acquired = runtime().try_block_on(async {
        #[cfg(test)]
        if PANIC_IN_LIFECYCLE_GATE.with(std::cell::Cell::take) {
            panic!("injected lifecycle-gate panic");
        }
        let gate = gate_on.generation_payment_guard().await;
        let live = gate_on.is_current_generation().await;
        (gate, live)
    });

    match acquired {
        Ok(acquired) => Ok(acquired),
        Err(error) => {
            // Nothing was published and nothing was registered, so no token or
            // handle exists that could ever release this build's reservation.
            // Reconcile it here or it is held until the process exits.
            runtime().block_on(release_on.abandon_transaction(finalized));
            Err(error)
        }
    }
}

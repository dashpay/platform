//! Broadcast-side UTXO reservation cleanup.
//!
//! `TransactionBuilder::build_signed` reserves the selected UTXOs in each
//! contributing funding account's `ReservationSet` and leaves the reservation
//! held on success, expecting the transaction to be broadcast. When the
//! broadcast *fails* the reservation must be reconciled here: released for an
//! immediate retry when Core definitively rejected the transaction, kept (for
//! the reservation-TTL backstop or a later sync) when acceptance is unknown.
//! A pooled build reserves across several accounts under one owner token, so
//! the reconciliation takes the whole contributor list, not one account.
//!
//! Every build-then-broadcast path must go through
//! [`broadcast_releasing_on_rejection`] so the cleanup exists once instead of
//! per call site — except paths with rejection-specific cleanup of their own
//! that must run *before* the release (the asset-lock flow untracks its
//! `Built` row first); those call the broadcaster directly and then
//! [`release_reservation_after_rejected_broadcast`].

use std::time::Duration;

use dashcore::{Transaction, Txid};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::account::AccountType;
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

/// Maximum age, in `last_processed_height` blocks, of a held funding
/// reservation before an operation that would *consume* it (broadcast) is
/// refused. Shared by the two deferred/split core-send surfaces so they bound a
/// reservation's lifetime against the same TTL with one number:
///
/// * the deferred build → broadcast/release registry
///   ([`SignedPaymentRegistry`](crate::SignedPaymentRegistry)), and
/// * the atomic finalized-transaction handle path
///   (`core_wallet_tx_builder_finalize` →
///   `broadcast_finalized_transaction`).
///
/// Kept strictly below key-wallet's `RESERVATION_TTL_BLOCKS` (24, ~1h at the
/// mainnet block target): a `build_signed` / `finalize_transaction` reservation
/// is stamped at the wallet's `last_processed_height` (via `set_current_height`)
/// and swept by a later `reserve`/`reserved` call — itself stamped with the same
/// `last_processed_height` clock — once it is `RESERVATION_TTL_BLOCKS` old,
/// silently returning the outpoint to the selectable pool where an unrelated
/// build can re-select and re-reserve it. `ReservationSet::release` removes an
/// outpoint unconditionally, with no ownership/generation check, so acting on a
/// reservation that was already swept could free (or broadcast against) a newer,
/// unrelated one. Refusing at this lower bound guarantees the guard always trips
/// **before** the underlying reservation could have been swept, leaving a margin
/// for `last_processed_height` to lag a few blocks behind the true tip.
pub(crate) const RESERVATION_MAX_AGE_BLOCKS: u32 = 20;

/// The ORPHAN BACKSTOP for a broadcast input fence
/// ([`WalletGeneration::pin_in_broadcast`](crate::wallet::core::WalletGeneration::pin_in_broadcast)'s
/// pending-spend phase): how long an outpoint the wallet has *never observed
/// spent* stays fenced after its dispatch returned.
///
/// This is **not** the mechanism that makes the fence safe, and it carries no
/// evidence about the dispatched transaction. The fence is released by
/// [`WalletGeneration::observe_spent`](crate::wallet::core::WalletGeneration::observe_spent)
/// when the wallet actually observes the outpoint spent — by the dispatch's own
/// transaction or by a competing one. This constant only stops a fence whose
/// transaction is never observed at all from stranding those inputs for the
/// life of the process.
///
/// # Why a wall clock, and why `Instant`
///
/// Every height-anchored form of this bound is unsound, and the reason is not
/// where the anchor is sampled (`dashpay/platform#4309`, rounds 2-4 all moved
/// the sample and all failed). It is that `last_processed_height` is not a
/// clock at all during catch-up: the wallet can advance it by thousands of
/// blocks in seconds, and those blocks were **mined before the transaction was
/// submitted**. Elapsed height is therefore evidence about the chain's past,
/// never about whether a transaction submitted *now* has been seen or dropped —
/// so no `installed_height + N` bound, however carefully sampled or however
/// atomically installed, can survive an ordinary historical sync.
///
/// [`Instant`] is the only clock in this crate with no chain input whatsoever.
/// It is monotonic, cannot be moved by catch-up, by a re-org, by a peer feeding
/// historical headers, or by a system clock adjustment. That is exactly the
/// "expiry clock that historical catch-up cannot fast-forward" the fix requires,
/// and it is *readable from a synchronous `Drop`* — which is what lets the
/// bound be stamped at the instant the pending-spend phase begins, on every exit
/// path including cancellation and unwind, with no manager lock, no height
/// sample and therefore no sample-to-install window to make atomic.
///
/// # Why a fence past dispatch is needed at all
///
/// `SpvBroadcaster` injects the dispatched transaction into dash-spv's local
/// mempool pipeline, so on that path the wallet marks the inputs spent within
/// milliseconds of dispatch returning and they leave the selectable set on
/// their own. `DapiBroadcaster::broadcast` does no such injection — it awaits
/// `sdk.execute` and returns — so on the DAPI path an accepted response *and*
/// an ambiguous `MaybeSent` both return with the inputs still selectable here
/// while the transaction is in flight. Ending the fence at dispatch return
/// therefore reopens the sweep + re-select race on that path
/// (`dashpay/platform#4309`): key-wallet's `ReservationSet` TTL is stamped at
/// *build* time, so a handle that sat between `finalize` and broadcast can be
/// swept the instant the next selection runs. On BOTH paths the release is now
/// the same observation, so the DAPI path is no longer the odd one out — it
/// simply reaches the observation later, when SPV relays the transaction back
/// or a block carries it.
///
/// # Why one hour
///
/// The real-time analogue of the bound this replaces: key-wallet's
/// `RESERVATION_TTL_BLOCKS` is 24 blocks, ~1 h at the 2.5-minute mainnet block
/// target. Keeping the same magnitude means the residual exposure of an
/// *unobserved* transaction is the one key-wallet's reservation TTL already
/// accepts, and no larger — only the clock changed, not the budget. It is also
/// long enough that the observation path wins in every healthy flow (an
/// accepted transaction is relayed back in seconds), and short enough that a
/// genuinely dropped transaction's inputs come back inside one session rather
/// than only at process exit — the fence is in-memory and never persisted, so a
/// bound much longer than a session would make process restart the real
/// recovery path.
pub(crate) const IN_BROADCAST_FENCE_ORPHAN_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// Whether a reservation stamped at `registered_height` is too old to act on at
/// `current_height` (see [`RESERVATION_MAX_AGE_BLOCKS`]). The registration
/// height is mandatory on both surfaces — it is derived from the finalized
/// [`SignedCoreTransaction::reservation_height`](crate::SignedCoreTransaction)
/// (captured inside the funding critical section, before the potentially-slow
/// external signer ran), never sampled independently.
///
/// *Consuming* (broadcasting) a stale reservation is refused: once the outpoint
/// may already have been swept by key-wallet's TTL and re-reserved by an
/// unrelated build, broadcasting would spend against that newer reservation.
/// The guarded broadcasts
/// ([`broadcast_finalized_transaction`](crate::CoreWallet::broadcast_finalized_transaction)
/// and the registry's [`broadcast`](crate::SignedPaymentRegistry::broadcast))
/// refuse with their stale-reservation errors, reconciling the reservation on
/// the way out. Cleanup (abandon/free, and that refusal-path reconciliation)
/// distinguishes two cases by the build's owner token:
///
/// * **Owner token present** (every funded finalize): the release is
///   owner-guarded (`release_reservation_if_owner`) and therefore safe at ANY
///   age — it frees the inputs only while this build still owns them and no-ops
///   once a TTL sweep or re-reservation transferred ownership — so aged cleanup
///   still releases, letting an immediate rebuild reselect the inputs.
/// * **Token-less** (a build that reserved nothing): the only release primitive
///   is `ReservationSet::release`, which removes an outpoint unconditionally
///   with no ownership check, so past the bound the by-outpoint release is
///   skipped and the aged reservation is left for key-wallet's TTL to reclaim.
///
/// An unknown *current* height means the wallet is gone from the manager, which
/// disables the guard (`None` → not expired). That is safe only because every
/// caller establishes liveness first and so never reaches here with a removed
/// wallet: the registry's
/// [`broadcast`](crate::SignedPaymentRegistry::broadcast) refuses with
/// `SignedPaymentError::WalletRemoved` before sampling the height, its
/// `reconcile_removed_entry` release is itself generation-bound and no-ops on a
/// missing wallet, and the finalized-transaction handle path runs after the
/// FFI layer's generation-identity check. The earlier claim that "the
/// wallet-mismatch / account-lookup paths already reject those cases" was wrong
/// for the registry broadcast path — `is_same_generation` compares handles (a
/// removed generation matches itself) and that path performs no account lookup
/// at all (`dashpay/platform#4185`).
pub(crate) fn reservation_expired(registered_height: u32, current_height: Option<u32>) -> bool {
    match current_height {
        Some(current) => current.saturating_sub(registered_height) >= RESERVATION_MAX_AGE_BLOCKS,
        None => false,
    }
}

/// Broadcast `tx` and reconcile the funding account's UTXO reservation on
/// failure.
///
/// On [`BroadcastError::Rejected`] — Core definitively did not accept the
/// transaction — the inputs reserved by the preceding `build_signed` are
/// released so an immediate retry can reselect them instead of failing with
/// spurious insufficient funds until the reservation-TTL backstop. On
/// [`BroadcastError::MaybeSent`] the reservation is intentionally kept:
/// releasing inputs of a transaction that may already be propagating invites
/// a double-spend on retry.
///
/// `account_type`/`account_index` identify the funding account whose
/// `ReservationSet` holds the inputs — the same account handed to
/// `set_funding` when the transaction was built.
///
/// Returns the still-typed [`BroadcastError`]; `?` converts it into
/// [`PlatformWalletError`](crate::PlatformWalletError) at the call sites.
pub(crate) async fn broadcast_releasing_on_rejection<B: TransactionBroadcaster + ?Sized>(
    broadcaster: &B,
    wallet_manager: &RwLock<WalletManager<PlatformWalletInfo>>,
    wallet_id: &WalletId,
    account_type: StandardAccountType,
    account_index: u32,
    tx: &Transaction,
) -> Result<Txid, BroadcastError> {
    match broadcaster.broadcast(tx).await {
        Ok(txid) => Ok(txid),
        Err(e) => {
            if matches!(e, BroadcastError::Rejected { .. }) {
                release_reservation_after_rejected_broadcast(
                    wallet_manager,
                    wallet_id,
                    &[AccountType::Standard {
                        index: account_index,
                        standard_account_type: account_type,
                    }],
                    tx,
                    // The generic send path doesn't thread the build's
                    // reservation token yet; keep its historical
                    // unconditional release.
                    None,
                )
                .await;
            }
            Err(e)
        }
    }
}

/// Release the funding accounts' UTXO reservations for `tx` after its
/// broadcast came back [`BroadcastError::Rejected`].
///
/// `funding_accounts` are the accounts that contributed inputs to `tx` — the
/// accounts handed to `add_funding` when it was built. A build funded from
/// several of them (a pooled asset lock, a pooled send) reserves in **each**
/// account's own `ReservationSet` under the one token, so every one of them
/// must reconcile; releasing only the first would leave the rest of the inputs
/// held until the TTL backstop reclaims them.
///
/// Callers that pair the release with other rejection cleanup must order
/// that cleanup **before** this call when it removes state a concurrent
/// flow could act on — while the reservation is still held the inputs
/// cannot be re-selected by a new build, so the pre-release window is
/// safe.
pub(crate) async fn release_reservation_after_rejected_broadcast(
    wallet_manager: &RwLock<WalletManager<PlatformWalletInfo>>,
    wallet_id: &WalletId,
    funding_accounts: &[AccountType],
    tx: &Transaction,
    reservation_token: Option<key_wallet::ReservationToken>,
) {
    // `release_reservation` takes `&self` and the manager map is
    // untouched, so a read lock suffices — this cleanup does not
    // serialize concurrent sends.
    let wm = wallet_manager.read().await;
    let Some((_, info)) = wm.get_wallet_and_info(wallet_id) else {
        tracing::warn!(
            wallet_id = %hex::encode(wallet_id),
            ?funding_accounts,
            "could not release UTXO reservation after rejected broadcast: wallet not found"
        );
        return;
    };
    for funding_account in funding_accounts {
        match info.core_wallet.accounts.funds_account(funding_account) {
            // Owner-guarded when the build's `ReservationToken` is available:
            // this cleanup always runs after `.await`s (build → broadcast), so
            // the original reservation may have been swept and the same
            // outpoints re-reserved by a NEWER build — an unconditional release
            // would clobber that newer owner and make its inputs re-selectable
            // by a conflicting transaction. Callers without a token (paths that
            // predate token plumbing) keep the historical unconditional release.
            Some(account) => match reservation_token {
                Some(token) => account.release_reservation_if_owner(tx, token),
                None => account.release_reservation(tx),
            },
            None => tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                ?funding_account,
                "could not release UTXO reservation after rejected broadcast: \
                 funds account not found"
            ),
        }
    }
}

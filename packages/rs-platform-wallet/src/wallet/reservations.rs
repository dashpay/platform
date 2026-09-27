//! Broadcast-side UTXO reservation cleanup.
//!
//! `TransactionBuilder::build_signed` reserves the selected UTXOs in each
//! contributing funding account's `ReservationSet` and leaves the reservation
//! held on success, expecting the transaction to be broadcast. When the
//! broadcast *fails* the reservation must be reconciled here: released for an
//! immediate retry when Core definitively rejected the transaction, kept when
//! acceptance is unknown. A pooled build reserves across several accounts under
//! one owner token, so the reconciliation takes the whole contributor list, not
//! one account.
//!
//! "Kept" means kept until key-wallet's own `ReservationSet` TTL sweeps it, and
//! that sweep is NOT what makes the ambiguous case safe. The inputs of a
//! transaction that may be on the network are held by the generation's
//! pending-spend fence, which the TTL does not touch and which no elapsed
//! quantity retires — only an observed spend does.
//! Reservation cleanup here and fence settlement in
//! the caller are two separate obligations; see
//! [`release_reservation_after_rejected_broadcast`] for the order they must run
//! in.
//!
//! Every build-then-broadcast path must go through
//! [`broadcast_releasing_on_rejection`] so the cleanup exists once instead of
//! per call site — except paths with rejection-specific cleanup of their own
//! that must run *before* the release (the asset-lock flow untracks its
//! `Built` row first); those call the broadcaster directly and then
//! [`release_reservation_after_rejected_broadcast`].

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

// THERE IS DELIBERATELY NO TIMEOUT CONSTANT FOR THE BROADCAST INPUT FENCE.
//
// Neither a height-anchored bound nor a monotonic-clock deadline is sound
// here. A height bound can be fast-forwarded by catch-up. A monotonic clock
// cannot, but it shares the real defect: ELAPSED TIME IS NOT EVIDENCE. A
// signed transaction does not become
// invalid by getting older, and waiting does not prove no peer retained it: a
// withholding DAPI endpoint can accept the transaction while keeping it off the
// network, and a backgrounded mobile wallet can outlast any deadline worth
// setting. When the deadline lapsed and catch-up had also swept key-wallet's
// reservation, the next build pruned the fence and signed a CONFLICTING
// transaction over inputs the original might still spend.
//
// So the fence now ends on evidence only — `WalletGeneration::observe_spent`,
// or a definitive pre-send failure. Anything added here later must be a
// LIVENESS path that says something about the pending transaction itself
// (persist it and query or rebroadcast it; take an explicit caller-driven
// abandon), never a duration that runs out. See the `in_broadcast` field docs
// on `WalletGeneration` for the full contract.

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
/// FFI layer's generation-identity check. The wallet-mismatch and
/// account-lookup paths are not enough on their own for the registry broadcast
/// path: `is_same_generation` compares handles (a removed generation matches
/// itself) and that path performs no account lookup at all.
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
/// # Ordering contract for paired cleanup
///
/// This call `.await`s a manager lock, so anything a caller does around it is
/// separated from it by a scheduling point. Which side of the call a given
/// cleanup belongs on follows from one rule — *the inputs must never be
/// reusable while anything else can still act on them*:
///
/// * Cleanup that removes **resumability** runs **before**: the asset-lock
///   flow untracks its `Built` row first, so the row is gone before its inputs
///   become re-spendable. While the reservation is still held a new build
///   cannot take them, so the pre-release window is safe.
/// * Cleanup that removes **protection** runs **after**:
///   [`InBroadcastPin::settle_released`](crate::wallet::core::InBroadcastPin::settle_released)
///   comes down only once this call has returned. Releasing the fence first
///   would open a window in which the outpoint is neither fenced nor — once
///   catch-up has swept it — reserved; a build queued on the manager write
///   lock could reserve and sign it there, and the token-less form of this
///   call would then delete that newer reservation, freeing the input for a
///   second signer. With the fence held across the call, such a build meets
///   it and rolls back instead.
///
/// The fence coming down after this call must not mean the pin still carries
/// its pending-on-drop DEFAULT through it: this call awaits, and awaiting is
/// where cancellation strikes. Every caller that has already ESTABLISHED the
/// released verdict (a definitive rejection, an abort before the broadcaster)
/// records it on the pin —
/// [`InBroadcastPin::settle_released_on_drop`](crate::wallet::core::InBroadcastPin::settle_released_on_drop)
/// — synchronously before awaiting this call, so a cancellation inside it
/// settles the fence as released rather than opening a pending-spend fence no
/// observed spend could ever clear.
///
/// Both ordering halves are exercised end to end by
/// `payments::tests::the_contact_send_fence_outlives_its_rejected_broadcast_reservation_cleanup`,
/// and the cancellation half by
/// `payments::tests::cancelling_the_rejected_broadcast_cleanup_leaves_no_fence`.
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

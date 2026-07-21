//! Broadcast-side UTXO reservation cleanup.
//!
//! `TransactionBuilder::build_signed` reserves the selected UTXOs in the
//! funding account's `ReservationSet` and leaves the reservation held on
//! success, expecting the transaction to be broadcast. When the broadcast
//! *fails* the reservation must be reconciled here: released for an immediate
//! retry when Core definitively rejected the transaction, kept (for the
//! reservation-TTL backstop or a later sync) when acceptance is unknown.
//!
//! Every build-then-broadcast path must go through
//! [`broadcast_releasing_on_rejection`] so the cleanup exists once instead of
//! per call site — except paths with rejection-specific cleanup of their own
//! that must run *before* the release (the asset-lock flow untracks its
//! `Built` row first); those call the broadcaster directly and then
//! [`release_reservation_after_rejected_broadcast`].

use dashcore::{Transaction, Txid};
use key_wallet::account::account_type::StandardAccountType;
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
/// * the atomic V2 finalized-transaction handle path
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

/// Whether a reservation stamped at `registered_height` is too old to act on at
/// `current_height` (see [`RESERVATION_MAX_AGE_BLOCKS`]). The registration
/// height is mandatory on both surfaces — it is derived from the finalized
/// [`SignedCoreTransaction::reservation_height`](crate::SignedCoreTransaction)
/// (captured inside the funding critical section, before the potentially-slow
/// external signer ran), never sampled independently.
///
/// An unknown *current* height means the wallet is gone from the manager, which
/// disables the guard (`None` → not expired). That is safe only because every
/// caller establishes liveness first and so never reaches here with a removed
/// wallet: the registry's
/// [`broadcast`](crate::SignedPaymentRegistry::broadcast) refuses with
/// `SignedPaymentError::WalletRemoved` before sampling the height, its
/// `reconcile_removed_entry` release is itself generation-bound and no-ops on a
/// missing wallet, and the V2 finalized-transaction handle path runs after the
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
                    account_type,
                    account_index,
                    tx,
                )
                .await;
            }
            Err(e)
        }
    }
}

/// Release the funding account's UTXO reservation for `tx` after its
/// broadcast came back [`BroadcastError::Rejected`].
///
/// Callers that pair the release with other rejection cleanup must order
/// that cleanup **before** this call when it removes state a concurrent
/// flow could act on — while the reservation is still held the inputs
/// cannot be re-selected by a new build, so the pre-release window is
/// safe.
pub(crate) async fn release_reservation_after_rejected_broadcast(
    wallet_manager: &RwLock<WalletManager<PlatformWalletInfo>>,
    wallet_id: &WalletId,
    account_type: StandardAccountType,
    account_index: u32,
    tx: &Transaction,
) {
    // `release_reservation` takes `&self` and the manager map is
    // untouched, so a read lock suffices — this cleanup does not
    // serialize concurrent sends.
    let wm = wallet_manager.read().await;
    let account = wm
        .get_wallet_and_info(wallet_id)
        .and_then(|(_, info)| match account_type {
            StandardAccountType::BIP44Account => info
                .core_wallet
                .bip44_managed_account_at_index(account_index),
            StandardAccountType::BIP32Account => info
                .core_wallet
                .bip32_managed_account_at_index(account_index),
        });
    match account {
        Some(account) => account.release_reservation(tx),
        None => tracing::warn!(
            wallet_id = %hex::encode(wallet_id),
            ?account_type,
            account_index,
            "could not release UTXO reservation after rejected broadcast: \
             wallet or funds account not found"
        ),
    }
}

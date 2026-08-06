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
                    ReservedFundingAccount::Standard(account_type, account_index),
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

/// The funding account whose `ReservationSet` holds a transaction's inputs —
/// the same account handed to `set_funding` when the transaction was built.
/// Asset locks can be funded from a CoinJoin account (the whole-balance
/// drain flow), so the release path must address that family too.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ReservedFundingAccount {
    /// A standard (BIP44/BIP32) account at the given index.
    Standard(StandardAccountType, u32),
    /// A CoinJoin account at the given index.
    CoinJoin(u32),
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
    funding_account: ReservedFundingAccount,
    tx: &Transaction,
    reservation_token: Option<key_wallet::ReservationToken>,
) {
    // `release_reservation` takes `&self` and the manager map is
    // untouched, so a read lock suffices — this cleanup does not
    // serialize concurrent sends.
    let wm = wallet_manager.read().await;
    let account = wm
        .get_wallet_and_info(wallet_id)
        .and_then(|(_, info)| match funding_account {
            ReservedFundingAccount::Standard(StandardAccountType::BIP44Account, account_index) => {
                info.core_wallet
                    .bip44_managed_account_at_index(account_index)
            }
            ReservedFundingAccount::Standard(StandardAccountType::BIP32Account, account_index) => {
                info.core_wallet
                    .bip32_managed_account_at_index(account_index)
            }
            ReservedFundingAccount::CoinJoin(account_index) => info
                .core_wallet
                .accounts
                .coinjoin_accounts
                .get(&account_index),
        });
    match account {
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
             wallet or funds account not found"
        ),
    }
}

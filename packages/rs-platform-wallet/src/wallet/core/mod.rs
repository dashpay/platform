pub mod balance;
pub mod balance_handler;
mod broadcast;
pub(crate) mod broadcast_resolver;
pub mod generation;
// Inherent `CoreWallet::sign_message` only — no types to re-export.
mod sign_message;
pub mod spend_observer;
pub(crate) use sign_message::is_signable_funding_account;
mod transaction;
pub mod wallet;

pub use balance::WalletBalance;
pub use balance_handler::BalanceUpdateHandler;
pub use generation::WalletGeneration;
pub(crate) use generation::{InBroadcastFences, InBroadcastPin};
pub use spend_observer::SpendObservationHandler;
pub(crate) use transaction::{
    build_error_awaiting_network, is_shortfall, resolve_source_accounts, trial_with_waiting_coins,
};
pub use transaction::{
    check_fee_rate, input_awaiting_network, is_final, FinalizeOptions, SignedCoreTransaction,
    ASSET_LOCK_FUNDING_SOURCES, SEND_FUNDING_SOURCES,
};
pub use wallet::CoreWallet;

use key_wallet::managed_account::transaction_record::TransactionRecord;

/// Is this record owned by a contact's watch-only DashPay chain?
///
/// A `DashpayExternalAccount` derives its addresses from the
/// **contact's** xpub, so this wallet can observe those outputs but can
/// never sign for them. They are the contact's coins; this wallet only
/// ever pays into them.
///
/// dashpay/rust-dashcore#926 established exactly that policy at the
/// balance layer, dropping `dashpay_external_accounts` from
/// `ManagedAccountCollection::all_funding_accounts` (and `_mut`), which
/// covers `balance`, `account_balances`, `utxos` and
/// `get_spendable_utxos` in one place. `dashpay_receival_accounts` were
/// deliberately kept: those derive from *our* xpub, so a contact paying
/// into them really is money arriving.
///
/// The persistence seam is the same rule's second home. Upstream
/// `check_core_transaction` emits **one `TransactionRecord` per matched
/// account** (`key_wallet::transaction_checking::wallet_checker`), so a
/// payment to a contact produces two records sharing one txid:
///
/// | record's account          | `direction` | `net_amount`     |
/// |---------------------------|-------------|------------------|
/// | funding (BIP44/BIP32/…)   | `Outgoing`  | `change - spent` |
/// | `DashpayExternalAccount`  | `Incoming`  | `+paid`          |
///
/// The external account's record is not *wrong* about its own account —
/// that chain did receive an output. It is wrong as a description of the
/// **wallet**, and the persisted `transactions` row is keyed by txid
/// alone, with no per-account dimension to disambiguate (the
/// `transaction_account_involvements` table is only written for
/// provider-key accounts). Whichever record is stored last therefore
/// defines the row, and the watch-only one — emitted last, because
/// `all_accounts` visits the DashPay accounts after the standard ones —
/// wins: a payment *away* is persisted as an incoming credit, and its
/// output is written into `txos` as a wallet-owned coin no key of ours
/// can spend.
///
/// So external-account records are excluded from the persist-time
/// projection entirely, exactly as #926 excluded the accounts from
/// balance aggregation. What survives from the same event is everything
/// that is genuinely ours to remember: the address-used flips and
/// highest-used watermarks (so contact address rotation keeps working),
/// the derived-address rows, and `derive_spent_utxos` (so a contact
/// spending an output persisted *before* this fix still clears the
/// stale row).
///
/// The predicate itself is canonical upstream
/// (`AccountType::is_contact_owned`, dashpay/rust-dashcore#952): its
/// exhaustive match forces any future account type to declare whether
/// its coins are the wallet's or a contact's, so this seam and #926's
/// cannot drift apart.
pub(crate) fn is_contact_watch_only(record: &TransactionRecord) -> bool {
    record.account_type.is_contact_owned()
}

/// Whether `record` spends coins the wallet owns: it recorded input details
/// (entries keyed to inputs that spent the account's outpoints) and the
/// account is not a contact's watch-only chain ([`is_contact_watch_only`]),
/// whose input details record the *contact* spending. The persisted projection's input rule and the
/// broadcast resolver's "own send".
pub(crate) fn record_spends_own_coins(record: &TransactionRecord) -> bool {
    !record.input_details.is_empty() && !is_contact_watch_only(record)
}

//! Wallet-level accounting of a Core transaction record: the ownership
//! predicate ([`is_owned`]), the direction rule ([`wallet_direction`]) and
//! the net formula ([`wallet_accounting`]). The live projection
//! (`fold_same_txid_records`), the per-wallet rows handed to hosts
//! ([`record_accounting`]), the host-history backfill
//! ([`history_accounting`]) and the SQLite repair
//! (`platform-wallet-storage`'s `core_history`) all call them, so a row
//! cannot change accounting when storage repairs it; they differ only on
//! an `i64` overflow (live saturates, repair errors).
//!
//! Hosts (the Swift and Kotlin SDKs) persist [`WalletAccounting`] keyed by
//! `(txid, wallet id)` and display it; they never derive it themselves.
//!
//! Upstream `key-wallet` classifies each matched account on its own, and
//! its account-local views disagree for an asset lock: the funding
//! account reads a lock with no change as `Outgoing` (the OP_RETURN burn
//! is not an owned output), while the keys account holding the credit
//! keys reads it as `Internal`. From the wallet's side an asset lock
//! moves Core duffs into its own Platform credits, so only the fee
//! leaves the wallet.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::{OutPoint, ScriptBuf, Transaction, Txid};
use key_wallet::managed_account::managed_account_ref::ManagedAccountRef;
use key_wallet::managed_account::transaction_record::{
    OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::transaction_router::{TransactionRouter, TransactionType};
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

/// One transaction's Core accounting from one wallet's point of view: the
/// value a host persists keyed by `(txid, wallet id)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalletAccounting {
    pub txid: Txid,
    /// `Σ owned outputs − Σ owned inputs` in duffs, saturated to `i64`.
    pub net_amount: i64,
    pub direction: TransactionDirection,
    /// The transaction fee, when the wallet funded every input.
    pub fee: Option<u64>,
}

/// One persisted Core transaction a host hands back for
/// [`history_accounting`].
#[derive(Debug, Clone)]
pub struct StoredCoreTransaction {
    pub transaction: Transaction,
    /// The host holds no [`WalletAccounting`] for this transaction and
    /// wallet yet. Transactions without the flag still resolve the
    /// prevouts of the flagged ones.
    pub needs_accounting: bool,
}

/// Classify a transaction from the wallet's point of view.
///
/// - `spends_ours`: at least one input spends a wallet-owned output.
/// - `has_ours`: at least one output is wallet-owned (`Received`/`Change`).
/// - `has_external`: at least one output is neither wallet-owned nor an
///   OP_RETURN burn, so value leaves the wallet.
///
/// An asset lock is `Internal` whenever nothing leaves the wallet, even
/// with no change output: its OP_RETURN value becomes the wallet's own
/// Platform credits. The case table lives in `platform-wallet-storage`
/// (`should_classify_repaired_direction_like_the_live_projection`).
pub fn wallet_direction(
    transaction_type: TransactionType,
    spends_ours: bool,
    has_ours: bool,
    has_external: bool,
) -> TransactionDirection {
    if transaction_type == TransactionType::CoinJoin {
        TransactionDirection::CoinJoin
    } else if !spends_ours {
        TransactionDirection::Incoming
    } else if !has_external && (has_ours || transaction_type == TransactionType::AssetLock) {
        TransactionDirection::Internal
    } else {
        TransactionDirection::Outgoing
    }
}

/// Recompute a record's wallet-level `net_amount` and `direction` from
/// its input and output details, saturating a net that does not fit `i64`.
///
/// A record with no details carries no accounting evidence (a keys-account
/// marker that met no funding slice), so it is left as upstream emitted
/// it. The SQLite repair skips such records for the same reason.
pub(crate) fn apply_wallet_accounting(record: &mut TransactionRecord) {
    if record.input_details.is_empty() && record.output_details.is_empty() {
        return;
    }
    let (net, direction) = wallet_accounting(record);
    record.net_amount = saturating_net(net);
    record.direction = direction;
}

/// The wallet-level accounting `record` carries, or `None` when it carries
/// none.
///
/// A record with no input or output details is no evidence of what the
/// transaction did to the wallet: a keys-account marker, or a record the
/// load path rebuilt from persisted bytes alone (an asset lock's funding
/// transaction, a spender of its input) and re-emits when a chain lock
/// promotes it. Such a record's `net_amount` is a placeholder, so the host
/// keeps whatever accounting it already stored.
pub fn record_accounting(record: &TransactionRecord) -> Option<WalletAccounting> {
    if record.input_details.is_empty() && record.output_details.is_empty() {
        return None;
    }
    let (net, direction) = wallet_accounting(record);
    Some(WalletAccounting {
        txid: record.txid,
        net_amount: saturating_net(net),
        direction,
        fee: record.fee,
    })
}

/// Accounting for each flagged transaction of a host's stored history, from
/// the wallet's own view of which scripts it owns.
///
/// Rows a host stored before it kept per-wallet accounting carry no
/// trustworthy amount, and the restored wallet holds no records to re-emit.
/// So the host hands back the raw history and this recomputes what the live
/// projection would have emitted:
/// - an output is owned when its script belongs to one of the wallet's funds
///   accounts other than a contact's watch-only chain
///   ([`AccountType::is_contact_owned`](key_wallet::account::AccountType::is_contact_owned));
/// - an input is owned when its prevout is an owned output of the supplied
///   history or one of the wallet's restored UTXOs;
/// - type, net and direction come from [`TransactionRouter::classify_transaction`]
///   and [`wallet_accounting`]'s rule.
///
/// A flagged transaction that touches no owned script yields no row, like a
/// detail-less record in [`record_accounting`]. An owned input whose funding
/// transaction is missing from the history and already spent reads as
/// foreign, exactly as it would to the live projection.
pub fn history_accounting(
    wallet: &ManagedWalletInfo,
    history: &[StoredCoreTransaction],
) -> Vec<WalletAccounting> {
    let mut owned_scripts: BTreeSet<ScriptBuf> = BTreeSet::new();
    let mut prevouts: BTreeMap<OutPoint, u64> = BTreeMap::new();
    for account in wallet.accounts.all_accounts() {
        let ManagedAccountRef::Funds(funds) = account else {
            continue;
        };
        if account
            .managed_account_type()
            .to_account_type()
            .is_contact_owned()
        {
            continue;
        }
        owned_scripts.extend(account.all_script_pubkeys());
        for (outpoint, utxo) in &funds.utxos {
            prevouts.insert(*outpoint, utxo.txout.value);
        }
    }
    for stored in history {
        let txid = stored.transaction.txid();
        for (vout, output) in stored.transaction.output.iter().enumerate() {
            if owned_scripts.contains(&output.script_pubkey) {
                prevouts.insert(
                    OutPoint {
                        txid,
                        vout: vout as u32,
                    },
                    output.value,
                );
            }
        }
    }

    let mut rows = Vec::new();
    for stored in history.iter().filter(|stored| stored.needs_accounting) {
        let transaction = &stored.transaction;
        let spent: Vec<u64> = transaction
            .input
            .iter()
            .filter_map(|input| prevouts.get(&input.previous_output).copied())
            .collect();
        let owned: Vec<(usize, u64)> = transaction
            .output
            .iter()
            .enumerate()
            .filter(|(_, output)| owned_scripts.contains(&output.script_pubkey))
            .map(|(index, output)| (index, output.value))
            .collect();
        if spent.is_empty() && owned.is_empty() {
            continue;
        }
        let (net, direction) = accounting(
            transaction,
            TransactionRouter::classify_transaction(transaction),
            &spent,
            &owned,
            &BTreeSet::new(),
        );
        // Only a transaction funded entirely by the wallet has a fee the
        // wallet can know.
        let fee = (spent.len() == transaction.input.len())
            .then(|| {
                let funded = spent.iter().try_fold(0u64, |sum, v| sum.checked_add(*v))?;
                let paid = transaction
                    .output
                    .iter()
                    .try_fold(0u64, |sum, o| sum.checked_add(o.value))?;
                funded.checked_sub(paid)
            })
            .flatten();
        rows.push(WalletAccounting {
            txid: transaction.txid(),
            net_amount: saturating_net(net),
            direction,
            fee,
        });
    }
    rows
}

/// Unreachable for real amounts (the whole supply fits i64 many times over);
/// saturate rather than panic on a corrupt record.
fn saturating_net(net: i128) -> i64 {
    i64::try_from(net).unwrap_or(if net < 0 { i64::MIN } else { i64::MAX })
}

/// Wallet-level net amount and direction of `record`, from its details.
///
/// The net is `Σ owned outputs − Σ owned inputs`: the change in the
/// wallet's Core balance. A keys-account marker's `+credit` (Platform
/// credits, not Core funds) is therefore not part of it, so an asset lock
/// nets `−(credit + fee)`. The net is returned as `i128` so each caller
/// picks its own policy for a value outside `i64`.
pub fn wallet_accounting(record: &TransactionRecord) -> (i128, TransactionDirection) {
    let spent: Vec<u64> = record.input_details.iter().map(|d| d.value).collect();
    let owned: Vec<(usize, u64)> = record
        .output_details
        .iter()
        .filter(|d| is_owned(d.role))
        .map(|d| (d.index as usize, d.value))
        .collect();
    let unspendable: BTreeSet<usize> = record
        .output_details
        .iter()
        .filter(|d| d.role == OutputRole::Unspendable)
        .map(|d| d.index as usize)
        .collect();
    accounting(
        &record.transaction,
        record.transaction_type,
        &spent,
        &owned,
        &unspendable,
    )
}

/// The net formula and direction rule over a transaction's owned inputs
/// (`spent` values) and owned outputs (`(index, value)`); `unspendable`
/// outputs leave nothing the wallet could lose, like an OP_RETURN.
fn accounting(
    transaction: &Transaction,
    transaction_type: TransactionType,
    spent: &[u64],
    owned: &[(usize, u64)],
    unspendable: &BTreeSet<usize>,
) -> (i128, TransactionDirection) {
    let received: i128 = owned.iter().map(|(_, value)| i128::from(*value)).sum();
    let paid: i128 = spent.iter().map(|value| i128::from(*value)).sum();
    // Indexed once so the per-output check below stays linear.
    let accounted: BTreeSet<usize> = owned
        .iter()
        .map(|(index, _)| *index)
        .chain(unspendable.iter().copied())
        .collect();
    let has_external = transaction
        .output
        .iter()
        .enumerate()
        .any(|(index, output)| !output.script_pubkey.is_op_return() && !accounted.contains(&index));
    let direction = wallet_direction(
        transaction_type,
        !spent.is_empty(),
        !owned.is_empty(),
        has_external,
    );
    (received - paid, direction)
}

/// Whether an output with this role belongs to the wallet.
pub fn is_owned(role: OutputRole) -> bool {
    matches!(role, OutputRole::Received | OutputRole::Change)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::changeset::fold_same_txid_records;
    use crate::test_support::core_fixtures::{
        block, foreign_address, outside, pay, records_of, spend_of, wallet_with_receive_address,
    };
    use key_wallet::account::{Account, AccountType};
    use key_wallet::managed_account::managed_core_funds_account::ManagedCoreFundsAccount;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Wallet;

    const FUNDING: u64 = 100_000;
    const PAID: u64 = 30_000;
    const CHANGE: u64 = 69_000;
    const FEE: u64 = FUNDING - PAID - CHANGE;

    fn stored(transaction: &Transaction, needs_accounting: bool) -> StoredCoreTransaction {
        StoredCoreTransaction {
            transaction: transaction.clone(),
            needs_accounting,
        }
    }

    /// A payment from outside into the wallet, then a spend of it that pays
    /// a foreign address and returns change: what the live projection emits
    /// for each, and the history the host stored.
    async fn receive_then_spend() -> (
        ManagedWalletInfo,
        Vec<WalletAccounting>,
        Transaction,
        Transaction,
    ) {
        let (mut wallet, mut info, receive) = wallet_with_receive_address();
        let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let change = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .next_change_address(Some(&xpub), true)
            .unwrap();
        let funding = spend_of(&[outside(7)], vec![pay(FUNDING, &receive)]);
        let spend = spend_of(
            &[OutPoint::new(funding.txid(), 0)],
            vec![pay(PAID, &foreign_address(9)), pay(CHANGE, &change)],
        );
        let mut live = Vec::new();
        for (tx, height) in [(&funding, 100), (&spend, 101)] {
            let mut records = records_of(&mut info, &mut wallet, tx, block(height)).await;
            fold_same_txid_records(&mut records);
            live.extend(records.iter().filter_map(record_accounting));
        }
        (info, live, funding, spend)
    }

    #[tokio::test]
    async fn should_backfill_stored_history_with_the_live_projections_accounting() {
        let (info, live, funding, spend) = receive_then_spend().await;
        assert_eq!(
            live.iter().map(|row| row.net_amount).collect::<Vec<_>>(),
            vec![FUNDING as i64, CHANGE as i64 - FUNDING as i64],
            "live projection: receive, then a spend netting −(paid + fee)"
        );

        let backfilled = history_accounting(&info, &[stored(&funding, true), stored(&spend, true)]);

        assert_eq!(backfilled.len(), 2);
        for (row, expected) in backfilled.iter().zip(&live) {
            assert_eq!(row.txid, expected.txid);
            assert_eq!(row.net_amount, expected.net_amount);
            assert_eq!(row.direction, expected.direction);
        }
        assert_eq!(backfilled[1].direction, TransactionDirection::Outgoing);
        assert_eq!(
            backfilled[1].fee,
            Some(FEE),
            "the wallet funded every input"
        );
        assert_eq!(backfilled[0].fee, None, "a foreign input hides the fee");
    }

    /// The other stored transactions resolve a flagged spend's inputs, but
    /// only flagged ones get a row.
    #[tokio::test]
    async fn should_resolve_inputs_from_unflagged_history_and_answer_only_flagged_rows() {
        let (info, live, funding, spend) = receive_then_spend().await;

        let backfilled =
            history_accounting(&info, &[stored(&funding, false), stored(&spend, true)]);

        assert_eq!(backfilled.len(), 1);
        assert_eq!(backfilled[0].txid, spend.txid());
        assert_eq!(backfilled[0].net_amount, live[1].net_amount);
    }

    #[tokio::test]
    async fn should_yield_no_row_for_a_transaction_touching_no_owned_script() {
        let (info, _, _, _) = receive_then_spend().await;
        let unrelated = spend_of(&[outside(3)], vec![pay(5_000, &foreign_address(4))]);

        assert!(history_accounting(&info, &[stored(&unrelated, true)]).is_empty());
    }

    /// An output on a contact's watch-only chain is the contact's money,
    /// not the wallet's: paying it is outgoing, the same rule the live
    /// projection applies by dropping the contact account's slice.
    #[tokio::test]
    async fn should_not_count_a_contacts_watch_only_output_as_owned() {
        let (_, mut info, receive) = wallet_with_receive_address();
        let contact_wallet = Wallet::new_random(
            dashcore::Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let contact_xpub = contact_wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let account = Account::new(
            None,
            AccountType::DashpayExternalAccount {
                index: 0,
                user_identity_id: [1; 32],
                friend_identity_id: [2; 32],
            },
            contact_xpub,
            dashcore::Network::Testnet,
        )
        .unwrap();
        let contact_account = ManagedCoreFundsAccount::from_account(&account);
        let contact_address = ManagedAccountRef::Funds(&contact_account)
            .all_addresses()
            .into_iter()
            .next()
            .expect("the contact chain derives addresses");
        info.accounts
            .insert_funds_bearing_account(contact_account)
            .unwrap();
        let funding = spend_of(&[outside(7)], vec![pay(FUNDING, &receive)]);
        let to_contact = spend_of(
            &[OutPoint::new(funding.txid(), 0)],
            vec![pay(FUNDING - FEE, &contact_address)],
        );

        let rows = history_accounting(&info, &[stored(&funding, false), stored(&to_contact, true)]);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].net_amount, -(FUNDING as i64));
        assert_eq!(rows[0].direction, TransactionDirection::Outgoing);
    }

    /// A record rebuilt at load from bytes alone (an asset lock's funding
    /// transaction) is re-emitted on chain-lock promotion with a placeholder
    /// net of 0: it must reach the host as "no accounting", not as 0.
    #[test]
    fn should_report_no_accounting_for_a_record_without_details() {
        let tx = spend_of(&[outside(1)], vec![pay(5_000, &foreign_address(2))]);
        let record = TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
            },
            block(10),
            TransactionType::AssetLock,
            TransactionDirection::Internal,
            Vec::new(),
            Vec::new(),
            0,
        );

        assert_eq!(record_accounting(&record), None);
    }
}

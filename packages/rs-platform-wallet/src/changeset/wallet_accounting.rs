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

use dashcore::blockdata::script::Instruction;
use dashcore::hashes::Hash;
use dashcore::{Address, OutPoint, PubkeyHash, ScriptBuf, Transaction, TxIn, Txid};
use key_wallet::managed_account::managed_account_ref::ManagedAccountRef;
use key_wallet::managed_account::managed_core_funds_account::ManagedCoreFundsAccount;
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
    /// The transaction fee, when the wallet funded every input; otherwise
    /// the wallet cannot know it.
    pub fee: Option<u64>,
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
    let accounting = record_parts(record);
    Some(WalletAccounting {
        txid: record.txid,
        net_amount: saturating_net(accounting.net),
        direction: accounting.direction,
        fee: accounting.fee,
    })
}

/// The txids whose transactions [`history_accounting`] needs, beyond
/// `unaccounted` itself, to resolve the inputs of `unaccounted`: every
/// prevout that is neither one of those transactions' outputs nor one of the
/// wallet's restored UTXOs. Some of them are foreign and will not be found;
/// that is expected.
pub fn funding_txids(wallet: &ManagedWalletInfo, unaccounted: &[Transaction]) -> BTreeSet<Txid> {
    let own: BTreeSet<Txid> = unaccounted.iter().map(Transaction::txid).collect();
    let utxos: BTreeSet<OutPoint> = wallet_funds(wallet)
        .flat_map(|funds| funds.utxos.keys().copied())
        .collect();
    unaccounted
        .iter()
        .flat_map(|transaction| &transaction.input)
        .map(|input| input.previous_output)
        .filter(|prevout| {
            !prevout.is_null() && !own.contains(&prevout.txid) && !utxos.contains(prevout)
        })
        .map(|prevout| prevout.txid)
        .collect()
}

/// Accounting for each of `unaccounted`, a host's stored transactions that
/// have no [`WalletAccounting`] yet, from the wallet's own view of which
/// scripts it owns. The live projection's rule, recomputed for rows a host
/// stored before it kept per-wallet accounting:
/// - an output is owned when its script belongs to one of the wallet's funds
///   accounts other than a contact's watch-only chain
///   ([`AccountType::is_contact_owned`](key_wallet::account::AccountType::is_contact_owned));
///   an address-less, provably unspendable output is `Unspendable`, as
///   key-wallet classifies it;
/// - an input is owned when its prevout is an owned output of `unaccounted`,
///   of `funding` (the stored transactions [`funding_txids`] named), or one
///   of the wallet's restored UTXOs;
/// - type, net, direction and fee follow [`record_accounting`].
///
/// A transaction that touches no owned script yields no row, like a
/// detail-less record.
///
/// An input whose prevout none of the above resolves is either someone
/// else's coin (every input of a genuine receipt) or the wallet's own coin
/// whose funding transaction the host could not supply. The two are told
/// apart by the input's redeem: when its scriptSig ends with a public key
/// whose P2PKH script the wallet owns, the wallet signed it. Such a
/// transaction gets no row and is listed in
/// [`HistoryAccounting::unresolved`] instead: reading the input as foreign
/// would turn a payment with change into an `Incoming` receipt of the
/// change. An input redeemed any other way reads as foreign; the wallet's
/// funds accounts hold only P2PKH scripts, so its own coins are always
/// redeemed with a public key.
///
/// A transaction left without a row is not yet accounted, and the host
/// shows it as such until a later backfill (or the live projection) writes
/// it. Any subset of a host's unaccounted transactions is therefore safe to
/// pass.
pub fn history_accounting(
    wallet: &ManagedWalletInfo,
    unaccounted: &[Transaction],
    funding: &[Transaction],
) -> HistoryAccounting {
    let mut owned_scripts: BTreeSet<ScriptBuf> = BTreeSet::new();
    let mut prevouts: BTreeMap<OutPoint, u64> = BTreeMap::new();
    for funds in wallet_funds(wallet) {
        owned_scripts.extend(ManagedAccountRef::Funds(funds).all_script_pubkeys());
        for (outpoint, utxo) in &funds.utxos {
            prevouts.insert(*outpoint, utxo.txout.value);
        }
    }
    for transaction in unaccounted.iter().chain(funding) {
        let txid = transaction.txid();
        for (vout, output) in transaction.output.iter().enumerate() {
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

    let mut accounted = HistoryAccounting::default();
    for transaction in unaccounted {
        let mut spent: Vec<u64> = Vec::new();
        let mut owned_but_unresolved = false;
        for input in &transaction.input {
            match prevouts.get(&input.previous_output) {
                Some(value) => spent.push(*value),
                None if !input.previous_output.is_null() => {
                    owned_but_unresolved |= p2pkh_redeemed_script(input)
                        .is_some_and(|script| owned_scripts.contains(&script));
                }
                None => {}
            }
        }
        if owned_but_unresolved {
            accounted.unresolved.push(transaction.txid());
            continue;
        }
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
        let unspendable: BTreeSet<usize> = transaction
            .output
            .iter()
            .enumerate()
            .filter(|(_, output)| {
                Address::from_script(&output.script_pubkey, wallet.network).is_err()
                    && output.script_pubkey.is_provably_unspendable()
            })
            .map(|(index, _)| index)
            .collect();
        let accounting = accounting(
            transaction,
            TransactionRouter::classify_transaction(transaction),
            &spent,
            &owned,
            &unspendable,
        );
        accounted.rows.push(WalletAccounting {
            txid: transaction.txid(),
            net_amount: saturating_net(accounting.net),
            direction: accounting.direction,
            fee: accounting.fee,
        });
    }
    accounted
}

/// What [`history_accounting`] made of a host's unaccounted transactions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryAccounting {
    /// One row per transaction it could account for.
    pub rows: Vec<WalletAccounting>,
    /// Transactions left without a row because an input spends a coin the
    /// wallet signed for but could not resolve. A later backfill retries
    /// them.
    pub unresolved: Vec<Txid>,
}

impl HistoryAccounting {
    /// Whether every transaction that touches the wallet got its row, so the
    /// backfill may be recorded as complete.
    pub fn is_complete(&self) -> bool {
        self.unresolved.is_empty()
    }
}

/// The P2PKH script `input` redeems, when its scriptSig ends with a public
/// key push (`<sig> <pubkey>`, how a P2PKH output is spent). `None` for any
/// other redeem.
fn p2pkh_redeemed_script(input: &TxIn) -> Option<ScriptBuf> {
    let Instruction::PushBytes(pubkey) = input.script_sig.instructions().last()?.ok()? else {
        return None;
    };
    let pubkey = pubkey.as_bytes();
    matches!(pubkey.len(), 33 | 65).then(|| ScriptBuf::new_p2pkh(&PubkeyHash::hash(pubkey)))
}

/// The wallet's funds accounts, without contacts' watch-only chains.
fn wallet_funds(wallet: &ManagedWalletInfo) -> impl Iterator<Item = &ManagedCoreFundsAccount> + '_ {
    wallet
        .accounts
        .all_accounts()
        .into_iter()
        .filter_map(|account| match account {
            ManagedAccountRef::Funds(funds)
                if !account
                    .managed_account_type()
                    .to_account_type()
                    .is_contact_owned() =>
            {
                Some(funds)
            }
            _ => None,
        })
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
    let accounting = record_parts(record);
    (accounting.net, accounting.direction)
}

/// What [`accounting`] computes, before an `i64` policy is applied to `net`.
struct Accounting {
    net: i128,
    direction: TransactionDirection,
    fee: Option<u64>,
}

/// [`accounting`] over a record's details.
fn record_parts(record: &TransactionRecord) -> Accounting {
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

/// The net formula, direction rule and fee over a transaction's owned
/// inputs (`spent` values) and owned outputs (`(index, value)`);
/// `unspendable` outputs leave nothing the wallet could lose, like an
/// OP_RETURN. The fee is known only when every input is an owned one.
fn accounting(
    transaction: &Transaction,
    transaction_type: TransactionType,
    spent: &[u64],
    owned: &[(usize, u64)],
    unspendable: &BTreeSet<usize>,
) -> Accounting {
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
    let fee = (!spent.is_empty() && spent.len() == transaction.input.len())
        .then(|| {
            let funded = spent
                .iter()
                .try_fold(0u64, |sum, value| sum.checked_add(*value))?;
            let paid_out = transaction
                .output
                .iter()
                .try_fold(0u64, |sum, output| sum.checked_add(output.value))?;
            funded.checked_sub(paid_out)
        })
        .flatten();
    Accounting {
        net: received - paid,
        direction,
        fee,
    }
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
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::transaction::special_transaction::TransactionPayload;
    use dashcore::{Network, TxOut};
    use key_wallet::account::{Account, AccountType, StandardAccountType};
    use key_wallet::managed_account::address_pool::PublicKeyType;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Wallet;

    const FUNDING: u64 = 100_000;
    const PAID: u64 = 30_000;
    const CHANGE: u64 = 69_000;
    const FEE: u64 = FUNDING - PAID - CHANGE;
    /// A CoinJoin denomination (0.001 DASH + fee).
    const DENOMINATION: u64 = 100_001;

    /// A payment from outside into the wallet, then `spend` of that coin,
    /// both seen by the live projection. Returns the wallet as it stands
    /// afterwards, the live rows, and the two transactions.
    async fn live(
        spend: impl FnOnce(OutPoint, &Address) -> Transaction,
    ) -> (
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
        let spend = spend(OutPoint::new(funding.txid(), 0), &change);
        let mut rows = Vec::new();
        for (tx, height) in [(&funding, 100), (&spend, 101)] {
            let mut records = records_of(&mut info, &mut wallet, tx, block(height)).await;
            fold_same_txid_records(&mut records);
            rows.extend(records.iter().filter_map(record_accounting));
        }
        (info, rows, funding, spend)
    }

    fn payment_with_change(coin: OutPoint, change: &Address) -> Transaction {
        spend_of(
            &[coin],
            vec![pay(PAID, &foreign_address(9)), pay(CHANGE, change)],
        )
    }

    /// The backfill must write exactly what the live projection emitted for
    /// the same transactions: amount, direction AND fee.
    async fn assert_backfill_matches_live(
        spend: impl FnOnce(OutPoint, &Address) -> Transaction,
    ) -> Vec<WalletAccounting> {
        let (info, live, funding, spend) = live(spend).await;
        assert_eq!(live.len(), 2, "the live projection accounts for both");
        let backfilled = history_accounting(&info, &[funding, spend], &[]).rows;
        assert_eq!(backfilled, live);
        backfilled
    }

    #[tokio::test]
    async fn should_backfill_a_payment_with_change_exactly_like_the_live_projection() {
        let rows = assert_backfill_matches_live(payment_with_change).await;

        assert_eq!(rows[0].net_amount, FUNDING as i64);
        assert_eq!(rows[0].fee, None, "a foreign input hides the fee");
        assert_eq!(rows[1].net_amount, CHANGE as i64 - FUNDING as i64);
        assert_eq!(rows[1].direction, TransactionDirection::Outgoing);
        assert_eq!(rows[1].fee, Some(FEE), "the wallet funded every input");
    }

    /// An asset lock with no change: the OP_RETURN burn becomes the wallet's
    /// own Platform credits, so it is `Internal`, and the Core net is the
    /// whole coin.
    #[tokio::test]
    async fn should_backfill_a_no_change_asset_lock_exactly_like_the_live_projection() {
        let credit = FUNDING - FEE;
        let rows = assert_backfill_matches_live(|coin, _| {
            let mut lock = spend_of(
                &[coin],
                vec![TxOut {
                    value: credit,
                    script_pubkey: ScriptBuf::new_op_return(&[]),
                }],
            );
            lock.version = 3;
            lock.special_transaction_payload =
                Some(TransactionPayload::AssetLockPayloadType(AssetLockPayload {
                    version: 1,
                    credit_outputs: vec![pay(credit, &foreign_address(5))],
                }));
            lock
        })
        .await;

        assert_eq!(rows[1].net_amount, -(FUNDING as i64));
        assert_eq!(rows[1].direction, TransactionDirection::Internal);
        assert_eq!(rows[1].fee, Some(FEE));
    }

    /// An address-less, provably unspendable output that is not an OP_RETURN
    /// takes nothing out of the wallet either: a self-transfer stays
    /// `Internal`, as key-wallet's `Unspendable` role makes it live.
    #[tokio::test]
    async fn should_backfill_an_unspendable_output_exactly_like_the_live_projection() {
        let rows = assert_backfill_matches_live(|coin, change| {
            spend_of(
                &[coin],
                vec![
                    pay(CHANGE, change),
                    TxOut {
                        value: 0,
                        // OP_VERIF: an illegal opcode, so provably unspendable.
                        script_pubkey: ScriptBuf::from_bytes(vec![0x65]),
                    },
                ],
            )
        })
        .await;

        assert_eq!(rows[1].direction, TransactionDirection::Internal);
    }

    #[tokio::test]
    async fn should_backfill_a_coinjoin_exactly_like_the_live_projection() {
        let rows = assert_backfill_matches_live(|coin, change| {
            spend_of(
                &[coin, outside(1), outside(2)],
                vec![
                    pay(DENOMINATION, change),
                    pay(DENOMINATION, &foreign_address(3)),
                    pay(DENOMINATION, &foreign_address(4)),
                ],
            )
        })
        .await;

        assert_eq!(rows[1].direction, TransactionDirection::CoinJoin);
        assert_eq!(rows[1].fee, None, "other participants funded inputs");
    }

    /// A spend's coin resolves from a funding transaction passed in
    /// `funding`; only `unaccounted` transactions get a row.
    #[tokio::test]
    async fn should_resolve_inputs_from_funding_transactions_and_answer_only_unaccounted() {
        let (info, live, funding, spend) = live(payment_with_change).await;

        let backfilled = history_accounting(&info, std::slice::from_ref(&spend), &[funding]).rows;

        assert_eq!(backfilled, vec![live[1]]);
    }

    /// A spend whose funding transaction the host no longer has still
    /// resolves while its coin is a restored UTXO.
    #[tokio::test]
    async fn should_resolve_an_input_from_a_restored_utxo_alone() {
        let (mut wallet, mut info, receive) = wallet_with_receive_address();
        let funding = spend_of(&[outside(7)], vec![pay(FUNDING, &receive)]);
        records_of(&mut info, &mut wallet, &funding, block(100)).await;
        let spend = spend_of(
            &[OutPoint::new(funding.txid(), 0)],
            vec![pay(FUNDING - FEE, &foreign_address(9))],
        );

        let rows = history_accounting(&info, std::slice::from_ref(&spend), &[]).rows;

        assert_eq!(
            rows,
            vec![WalletAccounting {
                txid: spend.txid(),
                net_amount: -(FUNDING as i64),
                direction: TransactionDirection::Outgoing,
                fee: Some(FEE),
            }]
        );
    }

    /// The manager fetches only prevouts nothing else resolves: not the
    /// unaccounted transactions' own outputs, not restored UTXOs, not a
    /// coinbase's null prevout.
    #[tokio::test]
    async fn should_name_only_the_funding_transactions_nothing_else_resolves() {
        let (mut wallet, mut info, receive) = wallet_with_receive_address();
        let utxo_funding = spend_of(&[outside(7)], vec![pay(FUNDING, &receive)]);
        records_of(&mut info, &mut wallet, &utxo_funding, block(100)).await;
        let chained = spend_of(&[outside(8)], vec![pay(FUNDING, &foreign_address(1))]);
        let spend = spend_of(
            &[
                OutPoint::new(utxo_funding.txid(), 0),
                OutPoint::new(chained.txid(), 0),
                outside(6),
                OutPoint::null(),
            ],
            vec![pay(1, &foreign_address(2))],
        );

        let needed = funding_txids(&info, &[chained.clone(), spend]);

        assert_eq!(
            needed,
            BTreeSet::from([outside(6).txid, outside(8).txid]),
            "`chained` is itself unaccounted and the UTXO is restored"
        );
    }

    #[tokio::test]
    async fn should_yield_no_row_for_a_transaction_touching_no_owned_script() {
        let (info, _, _, _) = live(payment_with_change).await;
        let unrelated = spend_of(&[outside(3)], vec![pay(5_000, &foreign_address(4))]);

        assert_eq!(
            history_accounting(&info, &[unrelated], &[]),
            HistoryAccounting::default()
        );
    }

    /// An output on a contact's watch-only chain is the contact's money,
    /// not the wallet's: paying it is outgoing, the same rule the live
    /// projection applies by dropping the contact account's slice.
    #[tokio::test]
    async fn should_not_count_a_contacts_watch_only_output_as_owned() {
        let (_, mut info, receive) = wallet_with_receive_address();
        let contact_wallet =
            Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
        let contact_xpub = contact_wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let account = Account::new(
            None,
            AccountType::DashpayExternalAccount {
                index: 0,
                user_identity_id: [1; 32],
                friend_identity_id: [2; 32],
            },
            contact_xpub,
            Network::Testnet,
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

        let rows = history_accounting(&info, std::slice::from_ref(&to_contact), &[funding]).rows;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].net_amount, -(FUNDING as i64));
        assert_eq!(rows[0].direction, TransactionDirection::Outgoing);
    }

    /// `tx` with input `index` redeemed the P2PKH way, `<sig> <pubkey>`.
    fn signed_by(mut tx: Transaction, index: usize, pubkey: &[u8]) -> Transaction {
        let signature = [0x30; 71];
        tx.input[index].script_sig = dashcore::script::Builder::new()
            .push_slice(<&dashcore::script::PushBytes>::try_from(&signature[..]).unwrap())
            .push_slice(<&dashcore::script::PushBytes>::try_from(pubkey).unwrap())
            .into_script();
        tx
    }

    /// The compressed public key behind one of the wallet's own addresses.
    fn owned_pubkey(info: &ManagedWalletInfo, address: &Address) -> Vec<u8> {
        let account = &info.accounts.standard_bip44_accounts[&0];
        match ManagedAccountRef::Funds(account)
            .get_address_info(address)
            .and_then(|info| info.public_key)
        {
            Some(PublicKeyType::ECDSA(bytes)) => bytes,
            other => panic!("the wallet derived {address} from an ECDSA key, got {other:?}"),
        }
    }

    /// A spend signed by the wallet's own key whose coin the host no longer
    /// holds (funding transaction gone, no longer a UTXO) cannot be accounted:
    /// reading the input as foreign would turn a payment with change into
    /// an `Incoming` receipt of the change. It gets no row and is reported
    /// unresolved, and accounts normally once the funding body is passed.
    #[test]
    fn should_withhold_the_row_of_a_spend_whose_owned_input_cannot_be_resolved() {
        let (wallet, mut info, receive) = wallet_with_receive_address();
        let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let change = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .next_change_address(Some(&xpub), true)
            .unwrap();
        let funding = spend_of(&[outside(7)], vec![pay(FUNDING, &receive)]);
        let spend = signed_by(
            payment_with_change(OutPoint::new(funding.txid(), 0), &change),
            0,
            &owned_pubkey(&info, &receive),
        );

        let missing = history_accounting(&info, std::slice::from_ref(&spend), &[]);
        assert_eq!(
            missing,
            HistoryAccounting {
                rows: Vec::new(),
                unresolved: vec![spend.txid()],
            }
        );
        assert!(!missing.is_complete());

        let filled = history_accounting(&info, std::slice::from_ref(&spend), &[funding]);
        assert_eq!(
            filled.rows,
            vec![WalletAccounting {
                txid: spend.txid(),
                net_amount: CHANGE as i64 - FUNDING as i64,
                direction: TransactionDirection::Outgoing,
                fee: Some(FEE),
            }]
        );
        assert!(filled.is_complete());
    }

    /// A genuine receipt's inputs are someone else's and never resolve; that
    /// must not hold back its row.
    #[test]
    fn should_still_account_a_receipt_whose_foreign_inputs_cannot_be_resolved() {
        let (_, info, receive) = wallet_with_receive_address();
        let mut foreign_key = vec![0x02];
        foreign_key.extend([7u8; 32]);
        let receipt = signed_by(
            spend_of(&[outside(7)], vec![pay(FUNDING, &receive)]),
            0,
            &foreign_key,
        );

        let accounted = history_accounting(&info, std::slice::from_ref(&receipt), &[]);

        assert_eq!(
            accounted,
            HistoryAccounting {
                rows: vec![WalletAccounting {
                    txid: receipt.txid(),
                    net_amount: FUNDING as i64,
                    direction: TransactionDirection::Incoming,
                    fee: None,
                }],
                unresolved: Vec::new(),
            }
        );
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
                standard_account_type: StandardAccountType::BIP44Account,
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

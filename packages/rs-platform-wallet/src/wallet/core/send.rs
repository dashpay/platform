//! General Core L1 payment building.
//!
//! [`CoreWallet::build_signed_payment`] is the first-class "send" primitive:
//! it selects inputs across every **signable** funds account, builds and signs
//! a standard payment transaction, and returns the **signed serialized bytes**
//! plus the computed fee and change amount — WITHOUT broadcasting and WITHOUT
//! persisting a debit.
//!
//! ## Why build-only / no-broadcast
//!
//! During the dashj→SDK transition the Android app keeps its own transaction
//! bookkeeping (dashj's `maybeCommitTx` drives CrowdNode, memos, and confidence
//! listeners). The app therefore wants the SDK to *build + sign* a payment from
//! the bound wallet and hand back the raw bytes, then commit + broadcast them
//! through dashj itself. Post-transition a separate SDK-broadcast mode will own
//! broadcasting and the debit persistence that goes with it; this primitive is
//! the permanent, generally-useful "give me signed bytes" half of that split.
//!
//! ## Persistence semantics (deliberate)
//!
//! Building does **not** persist a debit and does not write UTXOs, balances, or
//! transaction records back to the wallet. The only in-memory mutation is the
//! key-wallet `ReservationSet` bookkeeping that `set_funding` +
//! `TransactionBuilder::build_signed` perform on the primary funding account:
//! the selected inputs are marked *reserved* so a concurrent SDK build does not
//! re-select the same coins. That reservation is in-memory only (never
//! serialized) and is released when the spend is later processed back into the
//! wallet by sync, or by the reservation-TTL backstop, or explicitly via
//! [`ManagedCoreFundsAccount::release_reservation`] for an abandoned build. No
//! balance is debited until the transaction actually confirms — exactly what
//! the transition flow needs, since dashj owns commit/broadcast.
//!
//! [`ManagedCoreFundsAccount::release_reservation`]:
//!     key_wallet::managed_account::ManagedCoreFundsAccount::release_reservation

use std::collections::{HashMap, HashSet};

use dashcore::{Address as DashAddress, OutPoint, Transaction};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::ManagedAccountType;
use key_wallet::signer::Signer;
use key_wallet::wallet::managed_wallet_info::coin_selection::{SelectionError, SelectionStrategy};
use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
use key_wallet::wallet::managed_wallet_info::transaction_builder::{BuilderError, TransactionBuilder};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::Utxo;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::core::CoreWallet;

/// key-wallet's default fee rate (duffs per kB). Matches the asset-lock
/// builder's `DEFAULT_FEE_PER_KB` and `FeeRate::normal()`.
const DEFAULT_FEE_PER_KB: u64 = 1000;

/// The BIP44 account that supplies the change output (and whose reservation
/// ledger gates concurrent primary-account builds). The union of every other
/// signable funds account is added as explicit inputs on top of it.
const PRIMARY_BIP44_ACCOUNT_INDEX: u32 = 0;

/// A built-and-signed Core L1 payment, ready to be committed/broadcast by the
/// caller (dashj during the transition, or a later SDK-broadcast mode).
#[derive(Debug, Clone)]
pub struct SignedCorePayment {
    /// The signed transaction. Serialize with
    /// [`consensus::serialize`](dashcore::consensus::serialize) for the raw
    /// wire bytes the caller hands to its broadcaster.
    pub transaction: Transaction,
    /// The fee paid, in duffs, computed from the encoded size of the *signed*
    /// transaction.
    pub fee: u64,
    /// Duffs returned to the wallet's change address (0 when the build produced
    /// no change output — an exact-match selection or a dust-only remainder
    /// folded into the fee).
    pub change_amount: u64,
}

/// True for funds accounts the bound wallet cannot sign for. Only
/// `DashpayExternalAccount`s are watch-only: they hold a *contact's* receiving
/// addresses (we keep the contact's xpub to build payments *to* them and to
/// watch that side), so their UTXOs must never be selected as spend inputs —
/// signing would fail because no private key of ours derives them. Every other
/// funds account (BIP44/BIP32/CoinJoin/DashPay receiving) is derived from our
/// own seed and is signable.
fn is_watch_only_funds_account(account: &ManagedCoreFundsAccount) -> bool {
    matches!(
        account.managed_account_type(),
        ManagedAccountType::DashpayExternalAccount { .. }
    )
}

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// Build and sign a standard Core L1 payment to `outputs`, funding it from
    /// the union of every **signable** funds account, and return the signed
    /// transaction plus its fee and change amount. Does **not** broadcast and
    /// does **not** persist a debit (see the module docs for the persistence
    /// contract).
    ///
    /// ## Coin selection — union of signable accounts
    ///
    /// Inputs are selected across BIP44 + BIP32 + CoinJoin + DashPay-receiving
    /// accounts (the wallet-wide spendable set), reusing the same union-funding
    /// machinery the shielded asset-lock path uses
    /// ([`AssetLockManager::build_asset_lock_tx_from_all_funding_accounts`]):
    /// BIP44 account 0 is the PRIMARY account (it supplies the change output and
    /// its reservation ledger gates concurrent primary-account builds), and the
    /// spendable UTXOs of every other signable account are added as explicit
    /// builder inputs. Watch-only `DashpayExternalAccount`s are excluded — their
    /// coins belong to a contact and cannot be signed by this wallet.
    ///
    /// `LargestFirst` selection is used deliberately (not the builder default
    /// `BranchAndBound`): a CoinJoin account can hold many small mixed
    /// denominations, and `BranchAndBound`'s exact-match subset-sum is
    /// exponential over them (the same hang the asset-lock union path avoids).
    /// `LargestFirst`'s linear greedy accumulator also minimizes the input
    /// count — fewer signer round-trips and a smaller tx/fee.
    ///
    /// ## Parameters
    ///
    /// * `outputs` — the recipient `(address, amount_duffs)` pairs. Must be
    ///   non-empty and every amount must be positive.
    /// * `fee_per_kb` — fee rate in duffs/kB, or `None` for the default
    ///   (`1000`).
    /// * `signer` — the ECDSA signer that produces each input's P2PKH signature
    ///   (the Keychain/Keystore-backed `MnemonicResolverCoreSigner` in
    ///   production). No private key crosses the boundary.
    ///
    /// [`AssetLockManager::build_asset_lock_tx_from_all_funding_accounts`]:
    ///     crate::wallet::asset_lock
    pub async fn build_signed_payment<S: Signer>(
        &self,
        outputs: Vec<(DashAddress, u64)>,
        fee_per_kb: Option<u64>,
        signer: &S,
    ) -> Result<SignedCorePayment, PlatformWalletError> {
        if outputs.is_empty() {
            return Err(PlatformWalletError::TransactionBuild(
                "at least one output is required".to_string(),
            ));
        }
        if outputs.iter().any(|(_, amount)| *amount == 0) {
            return Err(PlatformWalletError::TransactionBuild(
                "every output amount must be greater than zero".to_string(),
            ));
        }
        let outputs_total: u64 = outputs.iter().map(|(_, amount)| *amount).sum();

        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_and_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        let height = info.core_wallet.last_processed_height();
        let fee_rate = FeeRate::new(fee_per_kb.unwrap_or(DEFAULT_FEE_PER_KB));

        // The PRIMARY account (change destination). Clone the xpub-bearing
        // account so no immutable borrow of `wallet` is held across the mutable
        // `info` borrow / signer await below.
        let primary_account = wallet
            .get_bip44_account(PRIMARY_BIP44_ACCOUNT_INDEX)
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "BIP44 account {PRIMARY_BIP44_ACCOUNT_INDEX} not found for payment funding"
                ))
            })?
            .clone();

        // Snapshot the primary account's spendable outpoints so the union sweep
        // does not double-add them: `set_funding` already seeds them, and
        // `add_inputs` must contribute only the OTHER signable accounts.
        let primary_outpoints: HashSet<OutPoint> = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&PRIMARY_BIP44_ACCOUNT_INDEX)
            .map(|a| {
                a.spendable_utxos(height)
                    .into_iter()
                    .map(|u| u.outpoint)
                    .collect()
            })
            .unwrap_or_default();

        // Single immutable pass over every signable funds account, building:
        //   (a) an owned `Address -> DerivationPath` resolver spanning all
        //       signable inputs, so signing resolves a key for an input drawn
        //       from any account;
        //   (b) the explicit extra inputs (all signable non-primary accounts);
        //   (c) an `OutPoint -> value` map for the post-build change figure;
        //   (d) the total selectable value, for a typed shortfall error.
        let mut path_map: HashMap<DashAddress, key_wallet::bip32::DerivationPath> = HashMap::new();
        let mut input_value: HashMap<OutPoint, u64> = HashMap::new();
        let mut extra_inputs: Vec<Utxo> = Vec::new();
        let mut selectable_value: u64 = 0;
        for account in info.core_wallet.accounts.all_funding_accounts() {
            if is_watch_only_funds_account(account) {
                continue;
            }
            for utxo in account.spendable_utxos(height) {
                selectable_value = selectable_value.saturating_add(utxo.value());
                input_value.insert(utxo.outpoint, utxo.value());
                if let Some(path) = account.address_derivation_path(&utxo.address) {
                    path_map.insert(utxo.address.clone(), path);
                }
                if !primary_outpoints.contains(&utxo.outpoint) {
                    extra_inputs.push(utxo.clone());
                }
            }
        }

        // Seed the primary account (inputs + change address + reservations),
        // append the union of the other signable accounts' inputs, then add the
        // real recipient outputs. The `&mut` borrow of the primary account is
        // scoped to this block; the returned builder owns cloned inputs /
        // reservations / change address, so no account borrow is held across
        // the signer await below.
        let builder = {
            let primary_funds = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&PRIMARY_BIP44_ACCOUNT_INDEX)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(format!(
                        "managed BIP44 account {PRIMARY_BIP44_ACCOUNT_INDEX} not found for \
                         payment funding"
                    ))
                })?;
            let mut builder = TransactionBuilder::new()
                .set_fee_rate(fee_rate)
                .set_current_height(height)
                // See the doc-comment: LargestFirst, not the default
                // BranchAndBound, to keep CoinJoin's many small denominations
                // from blowing up the exact-match subset-sum search.
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_funding(primary_funds, &primary_account)
                .add_inputs(extra_inputs);
            for (address, amount) in &outputs {
                builder = builder.add_output(address, *amount);
            }
            builder
        };

        let (transaction, _estimated_fee) = builder
            .build_signed(signer, move |addr| path_map.get(&addr).cloned())
            .await
            .map_err(|e| map_send_builder_error(e, selectable_value, outputs_total))?;

        // Derive fee and change from the transaction itself — the ground truth
        // that is always self-consistent (`fee + outputs + change == inputs`).
        // We do NOT use `build_signed`'s returned fee: it recomputes the fee
        // from the *signed* size, but the change output was already sized with
        // the pre-sign estimate, and ECDSA signatures vary in encoded length —
        // so the recomputed figure can differ by a few duffs from the fee the
        // wallet actually pays (`inputs − outputs`).
        //
        // `total_out` is the sum of every output; the only non-recipient output
        // a plain payment (no special payload) can carry is the single change
        // output back to the primary account, so `change = total_out − outputs`.
        // Any selected input we somehow can't price (impossible — every
        // spendable UTXO was recorded above) counts as 0, so `fee` is over-
        // rather than under-reported.
        let selected_input_value: u64 = transaction
            .input
            .iter()
            .map(|txin| input_value.get(&txin.previous_output).copied().unwrap_or(0))
            .sum();
        let total_out: u64 = transaction.output.iter().map(|o| o.value).sum();
        let fee = selected_input_value.saturating_sub(total_out);
        let change_amount = total_out.saturating_sub(outputs_total);

        Ok(SignedCorePayment {
            transaction,
            fee,
            change_amount,
        })
    }
}

/// Map a key-wallet [`BuilderError`] to a [`PlatformWalletError`], promoting the
/// two shortfall shapes to the typed [`PlatformWalletError::PaymentInsufficientFunds`]
/// so the exact `available`/`required` duff amounts survive. The builder's own
/// `InsufficientFunds` figures cover only what the primary-account selector saw,
/// so we substitute the union-wide selectable total (`available`) and the
/// outputs-plus-fee-ish target — `required` is at least the outputs total; a
/// coin-selection error already carries the fee-inclusive figure, which we
/// prefer when present.
fn map_send_builder_error(
    error: BuilderError,
    union_available: u64,
    outputs_total: u64,
) -> PlatformWalletError {
    match error {
        BuilderError::InsufficientFunds { required, .. } => {
            PlatformWalletError::PaymentInsufficientFunds {
                available: union_available,
                required: required.max(outputs_total),
            }
        }
        BuilderError::CoinSelection(SelectionError::InsufficientFunds { required, .. }) => {
            PlatformWalletError::PaymentInsufficientFunds {
                available: union_available,
                required: required.max(outputs_total),
            }
        }
        BuilderError::CoinSelection(SelectionError::NoUtxosAvailable) => {
            PlatformWalletError::PaymentInsufficientFunds {
                available: union_available,
                required: outputs_total,
            }
        }
        other => PlatformWalletError::TransactionBuild(format!("payment build failed: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use dashcore::hashes::Hash;
    use dashcore::{Address as DashAddress, Network, OutPoint, TxOut, Txid};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::ManagedCoreFundsAccount;
    use key_wallet::Utxo;

    use crate::test_support::{
        funded_wallet_manager, split_funded_wallet_manager, AlwaysRejectedBroadcaster,
    };
    use crate::wallet::core::balance::WalletBalance;
    use crate::wallet::core::CoreWallet;
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletError;

    use super::SignedCorePayment;

    /// A `CoreWallet` over a manager fixture. The send path never broadcasts,
    /// so the broadcaster is irrelevant (and the balance handle is unused by
    /// build — a fresh one is fine for the split fixtures that don't return it).
    fn core_wallet(
        wallet_manager: Arc<tokio::sync::RwLock<key_wallet_manager::WalletManager<crate::wallet::platform_wallet::PlatformWalletInfo>>>,
        wallet_id: WalletId,
        balance: Arc<WalletBalance>,
    ) -> CoreWallet<AlwaysRejectedBroadcaster> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        CoreWallet::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(AlwaysRejectedBroadcaster),
            balance,
        )
    }

    fn recipient(seed: u8) -> DashAddress {
        DashAddress::dummy(Network::Testnet, seed as usize)
    }

    /// Every input of a signed tx must carry a non-empty scriptSig (proof each
    /// selected input was actually signed by the per-account resolver).
    fn assert_all_inputs_signed(payment: &SignedCorePayment) {
        for (i, txin) in payment.transaction.input.iter().enumerate() {
            assert!(
                !txin.script_sig.is_empty(),
                "input {i} was left unsigned (empty scriptSig)"
            );
        }
    }

    /// A single-account BIP44 payment: the recipient output is present with the
    /// exact value, a fee is charged, and the change amount is exactly
    /// selected_input − output − fee (here the whole 0.1 DASH rides on one
    /// input, so change ≈ 0.1 − amount − fee).
    #[tokio::test]
    async fn bip44_payment_has_correct_output_change_and_fee() {
        let (wm, wallet_id, balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let core = core_wallet(wm, wallet_id, balance);

        let to = recipient(42);
        let amount = 1_000_000u64;
        let payment = core
            .build_signed_payment(vec![(to.clone(), amount)], None, &signer)
            .await
            .expect("build should succeed with 0.1 DASH funded");

        // Recipient output present with the exact value.
        let recipient_out = payment
            .transaction
            .output
            .iter()
            .find(|o| o.script_pubkey == to.script_pubkey());
        assert_eq!(
            recipient_out.map(|o| o.value),
            Some(amount),
            "recipient output must carry the requested amount"
        );

        // A fee was charged and change is exactly input − output − fee.
        assert!(payment.fee > 0, "a non-zero fee should be charged");
        assert_eq!(
            payment.change_amount,
            10_000_000 - amount - payment.fee,
            "change must be the single input minus the output minus the fee"
        );
        // The change output pays the leftover back to the wallet.
        assert!(
            payment
                .transaction
                .output
                .iter()
                .any(|o| o.value == payment.change_amount),
            "a change output equal to change_amount should exist"
        );
        assert_all_inputs_signed(&payment);
    }

    /// Coin selection spans the UNION of signable funds accounts: a payment
    /// that exceeds either the BIP44 slice or the CoinJoin slice alone pulls
    /// inputs from BOTH, and every mixed-account input is signed.
    #[tokio::test]
    async fn payment_funds_from_bip44_and_coinjoin_union() {
        // 0.09 DASH on BIP44, 0.09 on CoinJoin; ask 0.15 → needs both.
        let (wm, wallet_id, signer) = split_funded_wallet_manager(9_000_000, 9_000_000).await;

        // Snapshot each account's outpoints before building.
        let (bip44_ops, coinjoin_ops): (HashSet<OutPoint>, HashSet<OutPoint>) = {
            let guard = wm.read().await;
            let (_, info) = guard.get_wallet_and_info(&wallet_id).expect("wallet present");
            let bip44 = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .map(|a| a.utxos.keys().copied().collect())
                .unwrap_or_default();
            let coinjoin = info
                .core_wallet
                .accounts
                .coinjoin_accounts
                .get(&0)
                .map(|a| a.utxos.keys().copied().collect())
                .unwrap_or_default();
            (bip44, coinjoin)
        };

        let core = core_wallet(wm, wallet_id, Arc::new(WalletBalance::new()));
        let payment = core
            .build_signed_payment(vec![(recipient(7), 15_000_000)], None, &signer)
            .await
            .expect("0.15 DASH must be fundable from the 0.18 DASH union");

        let spent: HashSet<OutPoint> = payment
            .transaction
            .input
            .iter()
            .map(|i| i.previous_output)
            .collect();
        assert!(
            spent.iter().any(|op| bip44_ops.contains(op)),
            "at least one BIP44 input should be selected"
        );
        assert!(
            spent.iter().any(|op| coinjoin_ops.contains(op)),
            "at least one CoinJoin input should be selected"
        );
        assert_all_inputs_signed(&payment);
    }

    /// A shortfall across the whole signable union surfaces as the typed
    /// [`PlatformWalletError::PaymentInsufficientFunds`], with `available`
    /// reflecting the union total (not just the primary BIP44 slice).
    #[tokio::test]
    async fn union_shortfall_is_typed() {
        let (wm, wallet_id, signer) = split_funded_wallet_manager(9_000_000, 9_000_000).await;
        let core = core_wallet(wm, wallet_id, Arc::new(WalletBalance::new()));

        let result = core
            .build_signed_payment(vec![(recipient(7), 100_000_000)], None, &signer)
            .await;

        match result {
            Err(PlatformWalletError::PaymentInsufficientFunds {
                available,
                required,
            }) => {
                assert!(
                    (9_000_000..=18_000_000).contains(&available),
                    "available {available} should reflect the union (9M<..<=18M)"
                );
                assert!(
                    required >= 100_000_000,
                    "required {required} should be at least the requested amount"
                );
            }
            other => panic!("expected PaymentInsufficientFunds, got {other:?}"),
        }
    }

    /// A watch-only `DashpayExternalAccount` (a contact's addresses, which this
    /// wallet cannot sign) is EXCLUDED from coin selection: its UTXO is never
    /// spent, and its value is not counted toward the selectable total.
    #[tokio::test]
    async fn watch_only_external_account_is_excluded() {
        // BIP44 holds 0.1 DASH; a watch-only external account holds 1.0 DASH.
        let (wm, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;

        let watch_only_outpoint = OutPoint {
            txid: Txid::from_byte_array([0x9au8; 32]),
            vout: 0,
        };
        {
            let mut guard = wm.write().await;
            let (wallet, info) = guard
                .get_wallet_mut_and_info_mut(&wallet_id)
                .expect("wallet present");

            // Reuse the wallet's own BIP44 xpub as a stand-in "contact xpub":
            // the exclusion happens before any address derivation, so any valid
            // xpub suffices to construct the funds-bearing external account.
            let contact_xpub = wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .expect("bip44 account 0")
                .account_xpub;
            let account_type = AccountType::DashpayExternalAccount {
                index: 0,
                user_identity_id: [1u8; 32],
                friend_identity_id: [2u8; 32],
            };
            let account = key_wallet::Account {
                parent_wallet_id: Some(wallet_id),
                account_type,
                network: Network::Testnet,
                account_xpub: contact_xpub,
                is_watch_only: true,
            };
            let mut managed = ManagedCoreFundsAccount::from_account(&account);

            // Insert a large spendable UTXO directly (arbitrary address — the
            // account is skipped before its addresses are ever consulted).
            let addr = recipient(200);
            let utxo = Utxo {
                outpoint: watch_only_outpoint,
                txout: TxOut {
                    value: 100_000_000,
                    script_pubkey: addr.script_pubkey(),
                },
                address: addr,
                height: 1,
                is_coinbase: false,
                is_confirmed: true,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            };
            managed.utxos.insert(utxo.outpoint, utxo);
            info.core_wallet
                .accounts
                .insert_funds_bearing_account(managed)
                .expect("insert watch-only external account");
        }

        let core = core_wallet(wm, wallet_id, Arc::new(WalletBalance::new()));

        // Ask for 0.5 DASH: covered only if the 1.0-DASH watch-only UTXO were
        // spendable. Since it is excluded, the build must fail — and the
        // reported `available` must be just the 0.1-DASH BIP44 slice.
        let result = core
            .build_signed_payment(vec![(recipient(7), 50_000_000)], None, &signer)
            .await;
        match result {
            Err(PlatformWalletError::PaymentInsufficientFunds { available, .. }) => {
                assert_eq!(
                    available, 10_000_000,
                    "watch-only value must be excluded from the selectable total"
                );
            }
            other => panic!("expected PaymentInsufficientFunds, got {other:?}"),
        }

        // And a payment that the 0.1-DASH BIP44 slice CAN cover must never spend
        // the watch-only outpoint.
        let payment = core
            .build_signed_payment(vec![(recipient(7), 1_000_000)], None, &signer)
            .await
            .expect("0.01 DASH is fundable from the BIP44 slice alone");
        assert!(
            payment
                .transaction
                .input
                .iter()
                .all(|i| i.previous_output != watch_only_outpoint),
            "the watch-only UTXO must never be selected as an input"
        );
        assert_all_inputs_signed(&payment);
    }

    /// Input validation: empty outputs and zero-amount outputs are rejected
    /// before any wallet work.
    #[tokio::test]
    async fn rejects_empty_and_zero_outputs() {
        let (wm, wallet_id, balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let core = core_wallet(wm, wallet_id, balance);

        let empty = core.build_signed_payment(vec![], None, &signer).await;
        assert!(matches!(empty, Err(PlatformWalletError::TransactionBuild(_))));

        let zero = core
            .build_signed_payment(vec![(recipient(7), 0)], None, &signer)
            .await;
        assert!(matches!(zero, Err(PlatformWalletError::TransactionBuild(_))));
    }
}

//! General Core L1 payment building.
//!
//! [`CoreWallet::build_signed_payment`] is the first-class "send" primitive:
//! it selects inputs from **one** caller-named funds account, builds and signs
//! a standard payment transaction, and returns the **signed serialized bytes**
//! plus the computed fee and change amount — WITHOUT broadcasting and WITHOUT
//! persisting a debit.
//!
//! ## Funding-domain isolation
//!
//! Selection is confined to a single funding account, defaulting to the unmixed
//! BIP44 account — never a union across accounts. See
//! [`crate::wallet::funding_privacy`] for the invariant, the
//! dashpay/platform#4073 → #4184 history behind it, and the guardrail that
//! enforces it.
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
//! `TransactionBuilder::build_signed` perform on the **selected** funding
//! account: the selected inputs are marked *reserved* so a concurrent SDK build
//! does not re-select the same coins. Because selection is confined to one
//! account, every selected input is reserved in the ledger that all funding
//! paths consult for that account — there are no unreserved "secondary-account"
//! inputs (dashpay/platform#4247 review finding, now structurally impossible).
//!
//! That reservation is in-memory only (never
//! serialized) and is released when the spend is later processed back into the
//! wallet by sync, or by the reservation-TTL backstop, or explicitly via
//! [`ManagedCoreFundsAccount::release_reservation`] for an abandoned build. No
//! balance is debited until the transaction actually confirms — exactly what
//! the transition flow needs, since dashj owns commit/broadcast.
//!
//! [`ManagedCoreFundsAccount::release_reservation`]:
//!     key_wallet::managed_account::ManagedCoreFundsAccount::release_reservation

use std::collections::HashMap;

use dashcore::{Address as DashAddress, OutPoint, Transaction};
use key_wallet::bip32::DerivationPath;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::signer::Signer;
use key_wallet::wallet::managed_wallet_info::coin_selection::{SelectionError, SelectionStrategy};
use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    BuilderError, TransactionBuilder,
};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::core::CoreWallet;
use crate::wallet::funding_privacy::is_signable_funding_account;

/// key-wallet's default fee rate (duffs per kB). Matches the asset-lock
/// builder's `DEFAULT_FEE_PER_KB` and `FeeRate::normal()`.
const DEFAULT_FEE_PER_KB: u64 = 1000;

/// Consensus cap on any single amount this primitive will accept or aggregate.
const MAX_MONEY: u64 = dashcore::blockdata::constants::MAX_MONEY;

/// Dash's standard-transaction size limit, in bytes. A transaction above this
/// is non-standard and will not relay, so building one is never useful.
///
/// Derived from `dashcore::policy::MAX_STANDARD_TX_WEIGHT` (400_000 weight
/// units) rather than hard-coded: Dash has no segwit, so weight is exactly
/// 4× size and the byte limit is `MAX_STANDARD_TX_WEIGHT / 4` = 100_000.
const MAX_STANDARD_TX_SIZE: usize = (dashcore::policy::MAX_STANDARD_TX_WEIGHT / 4) as usize;

/// Encoded size of one P2PKH output, matching key-wallet's `TX_OUTPUT_SIZE`.
const TX_OUTPUT_SIZE: usize = 34;

/// Encoded size of one signed P2PKH input, matching the `148` key-wallet passes
/// to `select_coins_with_size`.
const TX_INPUT_SIZE: usize = 148;

/// Largest a Bitcoin/Dash varint can encode to. Used instead of the exact
/// varint width so the size estimate never comes in under key-wallet's.
const MAX_VARINT_SIZE: usize = 9;

/// Upper bound on the caller-supplied fee rate, in duffs/kB.
///
/// `FeeRate::calculate_fee` computes `sat_per_kb * size_bytes` with **unchecked**
/// `u64` multiplication (key-wallet `managed_wallet_info/fee.rs`), and the public
/// Kotlin/FFI APIs accept any non-negative `Long` — so an unbounded rate panics
/// in an overflow-checking Android build, or wraps in release, silently turning
/// an astronomical requested rate into a tiny fee.
///
/// The bound is derived so the product cannot overflow **for any transaction
/// size expressible in a `u32`** (~4.3 GB): with `sat_per_kb ≤ u64::MAX /
/// u32::MAX`, `sat_per_kb * size_bytes ≤ u64::MAX` whenever
/// `size_bytes ≤ u32::MAX`. That deliberately does NOT depend on the input
/// count. An earlier `MAX_MONEY / 100` bound assumed the transaction stayed
/// under [`MAX_STANDARD_TX_SIZE`], which this method never enforced — leaving
/// the product to overflow at ~878 kB, reachable both by an oversized recipient
/// list and by a CoinJoin account with a few thousand small denominations
/// (dashpay/platform#4247 and #4256 review). Since size is bounded by `u32`
/// long before it is bounded by policy, tying the bound to `u32::MAX` closes
/// the overflow unconditionally.
///
/// ~4.29e9 duffs/kB is ~43 DASH/kB — three orders of magnitude above any
/// legitimate rate (the default is 1_000), so nothing real is rejected. The
/// maximum fee this permits on a standard-size transaction is
/// `MAX_FEE_PER_KB * 100` ≈ 4_295 DASH, still far below [`MAX_MONEY`].
const MAX_FEE_PER_KB: u64 = u64::MAX / u32::MAX as u64;

/// The unmixed BIP44 account this primitive is pinned to, in both of its roles:
///
/// * the **default funding account** — where `funding_path: None` selects from;
/// * the **change sink** — key-wallet derives change addresses only for
///   *Standard* accounts, so a payment funded from an explicitly-named
///   non-Standard account (CoinJoin / DashPay-receiving) must route change
///   here. See [`crate::wallet::funding_privacy`].
///
/// Account 0 rather than a caller-chosen index: the transition-era send path
/// has exactly one BIP44 account, and the asset-lock builder's `account_index`
/// serves the same pinned role there.
const BIP44_ACCOUNT_INDEX: u32 = 0;

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

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// Build and sign a standard Core L1 payment to `outputs`, funding it from
    /// the **single** funds account named by `funding_path`, and return the
    /// signed transaction plus its fee and change amount. Does **not** broadcast
    /// and does **not** persist a debit (see the module docs for the persistence
    /// contract).
    ///
    /// ## Coin selection — one account, never a union
    ///
    /// Inputs come from exactly one funds account: `None` (the default) funds
    /// from the unmixed BIP44 account at [`BIP44_ACCOUNT_INDEX`], and
    /// `Some(path)` funds strictly from the one funds account whose
    /// account-level derivation path equals `path` (e.g. the DIP-9 CoinJoin
    /// account, to spend previously-mixed coins deliberately). There is **no
    /// union across accounts and no privacy-domain consent gate** — the caller
    /// names exactly one funding source, so there is nothing to consent to. If
    /// that account cannot cover the payment (+ fee) the build fails with
    /// [`PlatformWalletError::PaymentInsufficientFunds`] rather than silently
    /// topping up from another account; that failure is the point, not a
    /// limitation. See [`crate::wallet::funding_privacy`] for why
    /// (dashpay/platform#4073, blocked and re-scoped by #4184).
    ///
    /// Watch-only `DashpayExternalAccount`s can never fund a payment — their
    /// coins belong to a contact and the local mnemonic holds no key for
    /// them — so naming one explicitly is refused rather than silently ignored.
    ///
    /// Change routes to the BIP44 account at [`BIP44_ACCOUNT_INDEX`], which for
    /// the default funding path is the funding account itself. When an explicit
    /// non-Standard account (CoinJoin / DashPay-receiving) funds the payment,
    /// key-wallet cannot derive change on it at all, so the BIP44 sink is
    /// structural — the same change model the asset-lock builder uses.
    ///
    /// `LargestFirst` selection is used deliberately (not the builder default
    /// `BranchAndBound`): a CoinJoin account can hold many small mixed
    /// denominations, and `BranchAndBound`'s exact-match subset-sum is
    /// exponential over them. `LargestFirst`'s linear greedy accumulator also
    /// minimizes the input count — fewer signer round-trips and a smaller
    /// tx/fee.
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
    /// * `funding_path` — the account-level derivation path of the SINGLE funds
    ///   account whose UTXOs fund the payment. `None` (the default) funds from
    ///   the unmixed BIP44 account (dashpay/platform#4184).
    pub async fn build_signed_payment<S: Signer>(
        &self,
        outputs: Vec<(DashAddress, u64)>,
        fee_per_kb: Option<u64>,
        signer: &S,
        funding_path: Option<DerivationPath>,
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

        // Bound the recipient count so the transaction stays relayable AND so
        // key-wallet's unchecked `sat_per_kb * size_bytes` fee arithmetic cannot
        // be driven to overflow from the output side. `outputs.len()` is the one
        // caller-controlled size dimension (~25.8k recipients still fits in a
        // practical JNI blob); the input count is wallet-owned and key-wallet
        // caps it separately.
        //
        // Mirrors key-wallet's own base-size formula so the estimate is the one
        // the builder will actually use: 8 bytes of version/type/locktime, a
        // 1-byte input-count varint, the output-count varint (≤ 9, taken at its
        // maximum so this never under-estimates), 34 bytes per P2PKH output,
        // and 34 for the change output. Every step is checked —
        // `outputs.len() * 34` is an unchecked `usize` multiply inside
        // key-wallet. Room for at least one 148-byte input is required, since a
        // transaction with no inputs cannot be funded.
        let outputs_count = outputs.len();
        let base_size = outputs_count
            .checked_mul(TX_OUTPUT_SIZE)
            .and_then(|s| s.checked_add(8 + 1 + MAX_VARINT_SIZE + TX_OUTPUT_SIZE))
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "{outputs_count} recipients overflow the transaction size calculation"
                ))
            })?;
        if base_size.saturating_add(TX_INPUT_SIZE) > MAX_STANDARD_TX_SIZE {
            return Err(PlatformWalletError::TransactionBuild(format!(
                "{outputs_count} recipients need {base_size} bytes of outputs, leaving no \
                 room for inputs within the {MAX_STANDARD_TX_SIZE}-byte standard \
                 transaction limit"
            )));
        }

        // Reject below-dust recipients. `TransactionBuilder::add_output` applies
        // no relay policy at all — it copies the requested amount straight into
        // the `TxOut` — so without this a one-duff recipient produced a fully
        // signed transaction that every standard node rejects as nonstandard,
        // from a primitive documented as building a *standard* payment for
        // later broadcast (dashpay/platform#4247 review). Checked per output
        // against its OWN destination script, not a shared constant: the
        // threshold is script-shaped (546 duffs for P2PKH, less for P2SH),
        // which is also why key-wallet's hard-coded 546 change-dust literal is
        // not reusable here.
        //
        // After the count bound so an absurd recipient list is rejected before
        // this loop runs a script serialization per output, and before the
        // wallet lock is taken, before any input is reserved, and before the
        // signer is called — a request that can never relay must not tie up
        // coins or prompt the user for a keystore signature.
        for (address, amount) in &outputs {
            let dust = address.script_pubkey().dust_value().to_sat();
            if *amount < dust {
                return Err(PlatformWalletError::TransactionBuild(format!(
                    "output {amount} duffs to {address} is below the {dust}-duff dust \
                     threshold for its script type; such a transaction cannot be relayed"
                )));
            }
        }

        // Checked aggregation, bounded by MAX_MONEY. key-wallet sums the same
        // amounts with unchecked `u64` arithmetic while building, so an
        // unchecked total here would wrap in release builds (four outputs of
        // `1 << 62` sum to exactly 2^64) and let selection fund only the fee
        // while retaining four enormous outputs — a signed transaction
        // consensus rejects, with meaningless fee/change metadata. In an
        // overflow-checking build the same input panics inside the `extern "C"`
        // FFI frame, where the JNI guard cannot recover it.
        let outputs_total = outputs
            .iter()
            .try_fold(0u64, |total, (_, amount)| total.checked_add(*amount))
            .filter(|total| *total <= MAX_MONEY)
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "output amounts overflow or exceed MAX_MONEY ({MAX_MONEY} duffs)"
                ))
            })?;

        // Bound the caller-supplied fee rate for the same reason: key-wallet's
        // `FeeRate::calculate_fee` computes `sat_per_kb * size_bytes` with
        // unchecked `u64` multiplication, so a rate near `u64::MAX` (the public
        // Kotlin/FFI APIs accept any non-negative `Long`) panics in an
        // overflow-checking Android build, or wraps in release — turning an
        // astronomical requested rate into a tiny fee.
        let fee_per_kb = fee_per_kb.unwrap_or(DEFAULT_FEE_PER_KB);
        if fee_per_kb > MAX_FEE_PER_KB {
            return Err(PlatformWalletError::TransactionBuild(format!(
                "fee rate {fee_per_kb} duffs/kB exceeds the maximum {MAX_FEE_PER_KB}"
            )));
        }

        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_and_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        let height = info.core_wallet.last_processed_height();
        let network = info.core_wallet.network();
        let fee_rate = FeeRate::new(fee_per_kb);

        // ------------------------------------------------------------------
        // REGRESSION NOTE (dashpay/platform#4073 → #4184 → #4247)
        //
        // This selection block previously unioned every signable funds account
        // and ran LargestFirst over the combined set, with BIP44 change — which
        // irreversibly links ordinary, CoinJoin, and DashPay-receiving coins in
        // one on-chain transaction. Reviewer shumkov blocked exactly that on
        // PR #4184 (2026-07-21); the single-selected-account redesign (commit
        // 4d3e1322bc) was signed off 2026-07-23.
        //
        // The send-raw-tx code was written on an older integration line BEFORE
        // that re-scope, and shipped the blocked union behavior into the general
        // send path — with a test asserting the union as correct behavior. A
        // compile-clean, review-passed change is NOT sufficient evidence of
        // correctness here.
        //
        // INVARIANT: single selected account; never union funding accounts;
        // default unmixed BIP44. See `crate::wallet::funding_privacy` and its
        // guardrail tests.
        // ------------------------------------------------------------------

        // Resolve the account-level path of the unmixed BIP44 account: both the
        // default funding source and the change sink.
        let bip44_path = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&BIP44_ACCOUNT_INDEX)
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "BIP44 account {BIP44_ACCOUNT_INDEX} not found for payment funding"
                ))
            })?
            .managed_account_type()
            .to_account_type()
            .derivation_path(network)
            .map_err(|e| {
                PlatformWalletError::TransactionBuild(format!(
                    "failed to derive the unmixed BIP44 account-level path: {e}"
                ))
            })?;
        let funding_path = funding_path.unwrap_or_else(|| bip44_path.clone());
        let funds_from_change_account = funding_path == bip44_path;

        // The xpub-bearing BIP44 account: the change sink, and the fallback
        // signing-side account. Cloned so no immutable borrow of `wallet` is
        // held across the mutable `info` borrow below.
        let bip44_acc = wallet
            .get_bip44_account(BIP44_ACCOUNT_INDEX)
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "BIP44 account {BIP44_ACCOUNT_INDEX} not found for payment change routing"
                ))
            })?
            .clone();

        // Derive an explicit BIP44 change address ONLY when the funding account
        // is not the BIP44 sink itself: `set_funding` already derives change on
        // the funding account, which is correct (and consumes no extra pool
        // index) in the default case, but fails and is swallowed to `None` for a
        // non-Standard CoinJoin / DashPay account. Taken before the funding
        // account's `&mut` below — two accounts of the same collection cannot
        // both be borrowed mutably at once.
        let change_addr: Option<DashAddress> = if funds_from_change_account {
            None
        } else {
            let change_acc = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&BIP44_ACCOUNT_INDEX)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(format!(
                        "managed BIP44 account {BIP44_ACCOUNT_INDEX} not found for payment \
                         change routing"
                    ))
                })?;
            Some(
                change_acc
                    .next_change_address(Some(&bip44_acc.account_xpub), true)
                    .map_err(|e| {
                        PlatformWalletError::TransactionBuild(format!(
                            "failed to derive change address on BIP44 account \
                             {BIP44_ACCOUNT_INDEX}: {e}"
                        ))
                    })?,
            )
        };

        // The funding account's OWN wallet-level `Account`. `set_funding` calls
        // `funds_acc.next_change_address(Some(&acc.account_xpub))` before the
        // `set_change_address` override, so `acc` must be the funding account —
        // passing the BIP44 xpub for an explicitly-selected BIP32 account would
        // record a change entry derived from the wrong xpub into that account's
        // pool (dashpay/platform#4184 review).
        //
        // FAILS CLOSED. This previously fell back to `bip44_acc` when no
        // wallet-level account matched, which is the same silent-fallback shape
        // #4184 removed from the selector: the managed-account lookup below can
        // still resolve a CoinJoin or DashPay receival account, so the fallback
        // would hand `set_funding` another account's xpub and record a change
        // entry derived from it into the funding account's pool. Refusing is the
        // only safe answer — the two lookups disagreeing is a wallet-state bug,
        // not something to paper over with BIP44 (dashpay/platform#4247 and
        // #4256 review).
        //
        // Verified not to narrow any real path: `all_accounts()` does enumerate
        // CoinJoin and DashPay receiving-funds accounts, so every send test —
        // including the explicit-CoinJoin one — passes with the fallback
        // removed. It was dead code on every exercised path.
        let funding_wallet_acc = wallet
            .all_accounts()
            .into_iter()
            .find(|a| {
                a.derivation_path()
                    .map(|p| p == funding_path)
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "no wallet-level account matches funding derivation path                      {funding_path}; refusing to fund with another account's xpub"
                ))
            })?;

        // Locate the ONE managed funds account whose account-level path equals
        // `funding_path`, MUTABLY, so `set_funding` reserves the selected inputs
        // in that account's OWN reservation ledger. Watch-only
        // `DashpayExternalAccount`s are never fundable (the local mnemonic
        // cannot sign them) — refuse even when named explicitly.
        //
        // PRIVACY-DOMAIN-OK: this iterates funds accounts only to LOOK ONE UP by
        // derivation path. Exactly one account is selected and it alone funds
        // the transaction; nothing is accumulated across accounts.
        let mut selected: Option<&mut ManagedCoreFundsAccount> = None;
        for acc in info.core_wallet.accounts.all_funding_accounts_mut() {
            let acc_path = acc
                .managed_account_type()
                .to_account_type()
                .derivation_path(network)
                .map_err(|e| {
                    PlatformWalletError::TransactionBuild(format!(
                        "failed to derive account-level path for a funds account: {e}"
                    ))
                })?;
            if acc_path != funding_path {
                continue;
            }
            if !is_signable_funding_account(acc.managed_account_type()) {
                return Err(PlatformWalletError::TransactionBuild(format!(
                    "funding derivation path {funding_path} names a watch-only account whose \
                     coins the local wallet cannot sign; choose a signable funds account"
                )));
            }
            selected = Some(acc);
            break;
        }
        let selected = selected.ok_or_else(|| {
            PlatformWalletError::TransactionBuild(format!(
                "no spendable funds account matches funding derivation path {funding_path}"
            ))
        })?;

        // One immutable pass over the SELECTED account, building:
        //   (a) an owned `Address -> DerivationPath` resolver, so signing can
        //       resolve a key for every selected input without holding an
        //       account borrow across the signer await;
        //   (b) an `OutPoint -> value` map for the post-build fee/change figures;
        //   (c) the account's selectable total, for a typed shortfall error.
        let mut path_map: HashMap<DashAddress, DerivationPath> = HashMap::new();
        let mut input_value: HashMap<OutPoint, u64> = HashMap::new();
        let mut selectable_value: u64 = 0;
        for utxo in selected.spendable_utxos(height) {
            selectable_value = selectable_value.saturating_add(utxo.value());
            input_value.insert(utxo.outpoint, utxo.value());
            if let Some(path) = selected.address_derivation_path(&utxo.address) {
                path_map.insert(utxo.address.clone(), path);
            }
        }

        // Seed the selected account (inputs + reservations + its own change
        // address), override the change sink when the funding account cannot
        // derive change, then add the recipient outputs. The `&mut` borrow ends
        // with `set_funding`; the returned builder owns cloned inputs /
        // reservations / change address, so no account borrow is held across the
        // signer await below.
        let builder = {
            let mut builder = TransactionBuilder::new()
                .set_fee_rate(fee_rate)
                .set_current_height(height)
                // See the doc-comment: LargestFirst, not the default
                // BranchAndBound, to keep CoinJoin's many small denominations
                // from blowing up the exact-match subset-sum search.
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_funding(selected, funding_wallet_acc);
            if let Some(addr) = change_addr {
                builder = builder.set_change_address(addr);
            }
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
        // output back to the BIP44 sink, so `change = total_out − outputs`.
        // Any selected input we somehow can't price (impossible — every
        // spendable UTXO was recorded above) counts as 0, which LOWERS
        // `selected_input_value` and therefore lowers the `saturating_sub`
        // result: `fee` would be UNDER-reported, not over-.
        let selected_input_value: u64 = transaction
            .input
            .iter()
            .map(|txin| input_value.get(&txin.previous_output).copied().unwrap_or(0))
            .sum();
        let total_out: u64 = transaction.output.iter().map(|o| o.value).sum();
        let fee = selected_input_value.saturating_sub(total_out);
        let change_amount = total_out.saturating_sub(outputs_total);

        // Belt-and-braces: the pre-build check bounded only the output side,
        // because the input count is not knowable until coin selection has run.
        // Measure the transaction we actually built and refuse to hand back
        // bytes that cannot relay. In practice this fires only for a request
        // whose recipient list already passed the output-side bound but whose
        // funding account then contributed enough small inputs to push the
        // whole transaction over the limit.
        let signed_size = transaction.size();
        if signed_size > MAX_STANDARD_TX_SIZE {
            return Err(PlatformWalletError::TransactionBuild(format!(
                "the signed transaction is {signed_size} bytes, over the \
                 {MAX_STANDARD_TX_SIZE}-byte standard transaction limit; it would not relay. \
                 Send a smaller amount (fewer inputs) or fewer recipients"
            )));
        }

        Ok(SignedCorePayment {
            transaction,
            fee,
            change_amount,
        })
    }
}

/// Map a key-wallet [`BuilderError`] to a [`PlatformWalletError`], promoting the
/// two shortfall shapes to the typed [`PlatformWalletError::PaymentInsufficientFunds`]
/// so the exact `available`/`required` duff amounts survive.
///
/// `available` is the **selected account's** spendable total, deliberately —
/// never a wallet-wide figure. Reporting a wallet-wide "available" against a
/// single-account shortfall would invite the caller to retry with a larger
/// amount that can only succeed by crossing privacy domains, which this
/// primitive will not do (see [`crate::wallet::funding_privacy`]). `required` is
/// at least the outputs total; a coin-selection error already carries the
/// fee-inclusive figure, which we prefer when present.
fn map_send_builder_error(
    error: BuilderError,
    available_in_account: u64,
    outputs_total: u64,
) -> PlatformWalletError {
    match error {
        BuilderError::InsufficientFunds { required, .. } => {
            PlatformWalletError::PaymentInsufficientFunds {
                available: available_in_account,
                required: required.max(outputs_total),
            }
        }
        BuilderError::CoinSelection(SelectionError::InsufficientFunds { required, .. }) => {
            PlatformWalletError::PaymentInsufficientFunds {
                available: available_in_account,
                required: required.max(outputs_total),
            }
        }
        BuilderError::CoinSelection(SelectionError::NoUtxosAvailable) => {
            PlatformWalletError::PaymentInsufficientFunds {
                available: available_in_account,
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
    use key_wallet::bip32::DerivationPath;
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
        wallet_manager: Arc<
            tokio::sync::RwLock<
                key_wallet_manager::WalletManager<
                    crate::wallet::platform_wallet::PlatformWalletInfo,
                >,
            >,
        >,
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

    /// Snapshot the BIP44 and CoinJoin outpoints of a split fixture, plus the
    /// CoinJoin account's account-level derivation path (the `funding_path` a
    /// caller passes to spend previously-mixed coins deliberately).
    async fn split_account_outpoints_and_coinjoin_path(
        wm: &Arc<
            tokio::sync::RwLock<
                key_wallet_manager::WalletManager<
                    crate::wallet::platform_wallet::PlatformWalletInfo,
                >,
            >,
        >,
        wallet_id: &WalletId,
    ) -> (HashSet<OutPoint>, HashSet<OutPoint>, DerivationPath) {
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

        let guard = wm.read().await;
        let (_, info) = guard
            .get_wallet_and_info(wallet_id)
            .expect("wallet present");
        let network = info.core_wallet.network();
        let bip44 = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .map(|a| a.utxos.keys().copied().collect())
            .unwrap_or_default();
        let coinjoin_acc = info
            .core_wallet
            .accounts
            .coinjoin_accounts
            .get(&0)
            .expect("coinjoin account 0 present");
        let coinjoin = coinjoin_acc.utxos.keys().copied().collect();
        let path = coinjoin_acc
            .managed_account_type()
            .to_account_type()
            .derivation_path(network)
            .expect("coinjoin account-level path");
        (bip44, coinjoin, path)
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
            .build_signed_payment(vec![(to.clone(), amount)], None, &signer, None)
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

    /// **Replaces `payment_funds_from_bip44_and_coinjoin_union`**, which asserted
    /// the blocked union behavior as correct (dashpay/platform#4247; see the
    /// regression note in `build_signed_payment`).
    ///
    /// The DEFAULT funding path must never select CoinJoin (or any other
    /// non-BIP44 domain) coins, even when BIP44 alone cannot cover the payment.
    /// Failing is the correct outcome: a shortfall is reported as a typed error
    /// rather than silently satisfied by crossing a privacy domain, because the
    /// cross-domain link would be irreversible while the failure is merely
    /// retryable with an explicit `funding_path`.
    #[tokio::test]
    async fn default_funding_never_selects_other_domains() {
        // 0.09 DASH on BIP44, 0.09 on CoinJoin; ask 0.15 → only a union covers it.
        let (wm, wallet_id, signer) = split_funded_wallet_manager(9_000_000, 9_000_000).await;
        let core = core_wallet(wm, wallet_id, Arc::new(WalletBalance::new()));

        let result = core
            .build_signed_payment(vec![(recipient(7), 15_000_000)], None, &signer, None)
            .await;

        match result {
            Err(PlatformWalletError::PaymentInsufficientFunds {
                available,
                required,
            }) => {
                assert_eq!(
                    available, 9_000_000,
                    "available must reflect ONLY the BIP44 account, never the \
                     wallet-wide union"
                );
                assert!(
                    required >= 15_000_000,
                    "required {required} should be at least the requested amount"
                );
            }
            other => panic!(
                "the default path must not union BIP44 with CoinJoin — expected \
                 PaymentInsufficientFunds, got {other:?}"
            ),
        }
    }

    /// The default path funds happily from BIP44 when BIP44 alone suffices, and
    /// still leaves the CoinJoin coins untouched.
    #[tokio::test]
    async fn default_funding_selects_strictly_within_bip44() {
        // 0.2 DASH on BIP44, 0.09 on CoinJoin; ask 0.15 → BIP44 alone covers it.
        let (wm, wallet_id, signer) = split_funded_wallet_manager(20_000_000, 9_000_000).await;
        let (bip44_ops, coinjoin_ops, _) =
            split_account_outpoints_and_coinjoin_path(&wm, &wallet_id).await;

        let core = core_wallet(wm, wallet_id, Arc::new(WalletBalance::new()));
        let payment = core
            .build_signed_payment(vec![(recipient(7), 15_000_000)], None, &signer, None)
            .await
            .expect("0.15 DASH is fundable from the 0.2 DASH BIP44 account");

        let spent: HashSet<OutPoint> = payment
            .transaction
            .input
            .iter()
            .map(|i| i.previous_output)
            .collect();
        assert!(
            spent.iter().all(|op| bip44_ops.contains(op)),
            "every input must come from BIP44, spent {spent:?}"
        );
        assert!(
            !spent.iter().any(|op| coinjoin_ops.contains(op)),
            "the default path must never reach CoinJoin coins, spent {spent:?}"
        );
        assert_all_inputs_signed(&payment);
    }

    /// An explicitly-passed CoinJoin path selects strictly from that account and
    /// nothing else — the caller-consented, single-domain half of the #4184
    /// contract. Change still lands on BIP44 because key-wallet cannot derive a
    /// change address on a non-Standard account; that is structural, not a
    /// co-spend.
    #[tokio::test]
    async fn explicit_coinjoin_path_selects_only_coinjoin() {
        // 0.09 DASH on BIP44 (short), 0.2 on CoinJoin; take 0.15 from CoinJoin.
        let (wm, wallet_id, signer) = split_funded_wallet_manager(9_000_000, 20_000_000).await;
        let (bip44_ops, coinjoin_ops, coinjoin_path) =
            split_account_outpoints_and_coinjoin_path(&wm, &wallet_id).await;

        let core = core_wallet(wm, wallet_id, Arc::new(WalletBalance::new()));
        let payment = core
            .build_signed_payment(
                vec![(recipient(7), 15_000_000)],
                None,
                &signer,
                Some(coinjoin_path),
            )
            .await
            .expect("the named CoinJoin account covers 0.15 DASH");

        let spent: HashSet<OutPoint> = payment
            .transaction
            .input
            .iter()
            .map(|i| i.previous_output)
            .collect();
        assert!(!spent.is_empty(), "the payment must have selected inputs");
        assert!(
            spent.iter().all(|op| coinjoin_ops.contains(op)),
            "every input must come from the named CoinJoin account, spent {spent:?}"
        );
        assert!(
            !spent.iter().any(|op| bip44_ops.contains(op)),
            "an explicit CoinJoin path must not pull BIP44 inputs, spent {spent:?}"
        );
        // Change is returned to the transparent BIP44 sink.
        assert!(
            payment.change_amount > 0,
            "spending a 0.2 DASH UTXO for 0.15 DASH must leave change"
        );
        assert_all_inputs_signed(&payment);
    }

    /// A shortfall inside the SELECTED account surfaces as the typed
    /// [`PlatformWalletError::PaymentInsufficientFunds`], with `available`
    /// reflecting only that account — never a wallet-wide union total, which
    /// would invite a retry that can only succeed by crossing domains.
    #[tokio::test]
    async fn selected_account_shortfall_is_typed() {
        let (wm, wallet_id, signer) = split_funded_wallet_manager(9_000_000, 9_000_000).await;
        let (_, _, coinjoin_path) =
            split_account_outpoints_and_coinjoin_path(&wm, &wallet_id).await;
        let core = core_wallet(wm, wallet_id, Arc::new(WalletBalance::new()));

        let result = core
            .build_signed_payment(
                vec![(recipient(7), 100_000_000)],
                None,
                &signer,
                Some(coinjoin_path),
            )
            .await;

        match result {
            Err(PlatformWalletError::PaymentInsufficientFunds {
                available,
                required,
            }) => {
                assert_eq!(
                    available, 9_000_000,
                    "available must reflect only the named CoinJoin account"
                );
                assert!(
                    required >= 100_000_000,
                    "required {required} should be at least the requested amount"
                );
            }
            other => panic!("expected PaymentInsufficientFunds, got {other:?}"),
        }
    }

    /// A `funding_path` that names no funds account is a hard error — never a
    /// silent fallback to the default account, which would fund the payment
    /// from coins the caller did not choose.
    #[tokio::test]
    async fn unknown_funding_path_is_rejected() {
        use std::str::FromStr;

        let (wm, wallet_id, balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let core = core_wallet(wm, wallet_id, balance);

        let nowhere = DerivationPath::from_str("m/44'/5'/77'").expect("valid path");
        let result = core
            .build_signed_payment(
                vec![(recipient(7), 1_000_000)],
                None,
                &signer,
                Some(nowhere),
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBuild(_))),
            "an unmatched funding path must fail, got {result:?}"
        );
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
        // spendable. It is on a different domain from the default BIP44 funding
        // path, so the default send can never reach it — the build must fail
        // with the 0.1-DASH BIP44 slice as `available`.
        let result = core
            .build_signed_payment(vec![(recipient(7), 50_000_000)], None, &signer, None)
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
            .build_signed_payment(vec![(recipient(7), 1_000_000)], None, &signer, None)
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

        let empty = core.build_signed_payment(vec![], None, &signer, None).await;
        assert!(matches!(
            empty,
            Err(PlatformWalletError::TransactionBuild(_))
        ));

        let zero = core
            .build_signed_payment(vec![(recipient(7), 0)], None, &signer, None)
            .await;
        assert!(matches!(
            zero,
            Err(PlatformWalletError::TransactionBuild(_))
        ));
    }

    /// A positive-but-below-dust recipient must be refused. `add_output` applies
    /// no relay policy, so before this check the primitive happily returned
    /// fully signed bytes for a transaction every standard node rejects as
    /// nonstandard (dashpay/platform#4247 review). 546 duffs is the P2PKH
    /// threshold `Script::dust_value()` computes.
    #[tokio::test]
    async fn below_dust_outputs_are_rejected() {
        let (wm, wallet_id, balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let core = core_wallet(wm, wallet_id, balance);

        let to = recipient(42);
        let dust = to.script_pubkey().dust_value().to_sat();
        assert_eq!(dust, 546, "P2PKH dust threshold");

        for amount in [1u64, dust - 1] {
            let result = core
                .build_signed_payment(vec![(to.clone(), amount)], None, &signer, None)
                .await;
            match result {
                Err(PlatformWalletError::TransactionBuild(m)) => assert!(
                    m.contains("dust"),
                    "the rejection must name dust as the cause, got {m:?}"
                ),
                other => panic!("{amount} duffs is below dust and must be refused, got {other:?}"),
            }
        }

        // A dust-sized output hidden among valid ones is caught too — the check
        // is per output, not just on the first.
        let mixed = core
            .build_signed_payment(
                vec![
                    (recipient(1), 1_000_000),
                    (recipient(2), 5),
                    (recipient(3), 1_000_000),
                ],
                None,
                &signer,
                None,
            )
            .await;
        assert!(
            matches!(mixed, Err(PlatformWalletError::TransactionBuild(ref m)) if m.contains("dust")),
            "a below-dust output among valid ones must still be refused, got {mixed:?}"
        );

        // Exactly at the threshold is valid and still builds.
        let at_threshold = core
            .build_signed_payment(vec![(to, dust)], None, &signer, None)
            .await
            .expect("an output exactly at the dust threshold is standard");
        assert_all_inputs_signed(&at_threshold);
    }

    /// Rejecting a below-dust request must not cost the caller anything: it
    /// happens before the wallet lock, so no input is reserved and the very
    /// next legitimate build still finds the account's coins selectable.
    #[tokio::test]
    async fn a_rejected_dust_request_reserves_nothing() {
        let (wm, wallet_id, balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let core = core_wallet(wm, wallet_id, balance);

        for _ in 0..3 {
            assert!(core
                .build_signed_payment(vec![(recipient(9), 100)], None, &signer, None)
                .await
                .is_err());
        }

        let payment = core
            .build_signed_payment(vec![(recipient(9), 1_000_000)], None, &signer, None)
            .await
            .expect("refused dust requests must not have reserved the account's UTXOs");
        assert_all_inputs_signed(&payment);
    }

    /// The output total is aggregated with checked arithmetic and bounded by
    /// `MAX_MONEY`. Four outputs of `1 << 62` sum to exactly 2^64: unchecked,
    /// that wraps to zero in release builds and lets selection fund only the
    /// fee while retaining four enormous outputs — a signed transaction
    /// consensus rejects, with meaningless fee/change metadata.
    #[tokio::test]
    async fn output_total_overflow_and_max_money_are_rejected() {
        let (wm, wallet_id, balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let core = core_wallet(wm, wallet_id, balance);

        let wrapping = vec![
            (recipient(1), 1u64 << 62),
            (recipient(2), 1u64 << 62),
            (recipient(3), 1u64 << 62),
            (recipient(4), 1u64 << 62),
        ];
        match core
            .build_signed_payment(wrapping, None, &signer, None)
            .await
        {
            Err(PlatformWalletError::TransactionBuild(m)) => assert!(
                m.contains("MAX_MONEY"),
                "a wrapping total must be refused as a monetary-bound breach, got {m:?}"
            ),
            other => panic!("4 × (1 << 62) wraps to zero and must be refused, got {other:?}"),
        }

        // A single in-range-but-over-MAX_MONEY amount is refused as well.
        let over = core
            .build_signed_payment(
                vec![(recipient(1), super::MAX_MONEY + 1)],
                None,
                &signer,
                None,
            )
            .await;
        assert!(
            matches!(over, Err(PlatformWalletError::TransactionBuild(ref m)) if m.contains("MAX_MONEY")),
            "an amount over MAX_MONEY must be refused, got {over:?}"
        );

        // MAX_MONEY itself is within bounds, so it passes validation and fails
        // later on funds — proving the bound is inclusive, not off by one.
        let at_max = core
            .build_signed_payment(vec![(recipient(1), super::MAX_MONEY)], None, &signer, None)
            .await;
        assert!(
            matches!(
                at_max,
                Err(PlatformWalletError::PaymentInsufficientFunds { .. })
            ),
            "MAX_MONEY exactly must pass the bound and fail on funds, got {at_max:?}"
        );
    }

    /// The fee rate is bounded before it reaches key-wallet, whose
    /// `calculate_fee` multiplies `sat_per_kb * size_bytes` unchecked — a rate
    /// near `u64::MAX` (the Kotlin/FFI APIs accept any non-negative `Long`)
    /// panics in an overflow-checking build or wraps in release.
    #[tokio::test]
    async fn excessive_fee_rates_are_rejected() {
        let (wm, wallet_id, balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let core = core_wallet(wm, wallet_id, balance);

        for rate in [u64::MAX, u64::MAX / 2, super::MAX_FEE_PER_KB + 1] {
            let result = core
                .build_signed_payment(vec![(recipient(7), 1_000_000)], Some(rate), &signer, None)
                .await;
            match result {
                Err(PlatformWalletError::TransactionBuild(m)) => assert!(
                    m.contains("fee rate"),
                    "the rejection must name the fee rate, got {m:?}"
                ),
                other => panic!("fee rate {rate} must be refused, got {other:?}"),
            }
        }

        // A sane rate still works, so the bound isn't rejecting real traffic.
        let ok = core
            .build_signed_payment(vec![(recipient(7), 1_000_000)], Some(5_000), &signer, None)
            .await
            .expect("5000 duffs/kB is an ordinary rate");
        assert!(ok.fee > 0);
    }

    /// The fee-rate bound must make key-wallet's unchecked
    /// `sat_per_kb * size_bytes` product unrepresentable-free for ANY
    /// transaction size a `u32` can express — which is the point of deriving it
    /// from `u32::MAX` rather than from the standard size limit. A cleanup that
    /// loosened it back to `MAX_MONEY / 100` would overflow at ~878 kB, which a
    /// funding account with a few thousand small denominations can reach.
    #[test]
    fn max_fee_rate_cannot_overflow_key_wallets_fee_product() {
        for size in [super::MAX_STANDARD_TX_SIZE as u64, 878_434, u32::MAX as u64] {
            assert!(
                super::MAX_FEE_PER_KB.checked_mul(size).is_some(),
                "MAX_FEE_PER_KB * {size} must not overflow u64"
            );
        }
        // And it stays permissive enough to be irrelevant in practice.
        assert!(
            super::MAX_FEE_PER_KB > 1_000_000,
            "the bound must sit far above any legitimate duffs/kB rate"
        );
    }

    /// An oversized recipient list is refused before any wallet work. ~25.8k
    /// recipients fit in a practical JNI blob and would drive key-wallet's
    /// estimated size past the point where the fee product overflows, as well
    /// as producing a transaction far too large to relay.
    #[tokio::test]
    async fn oversized_recipient_lists_are_rejected() {
        let (wm, wallet_id, balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let core = core_wallet(wm, wallet_id, balance);

        // Smallest count whose outputs alone leave no room for a single input
        // within the 100 kB standard limit.
        let over = (super::MAX_STANDARD_TX_SIZE - super::TX_INPUT_SIZE) / super::TX_OUTPUT_SIZE;
        let outputs: Vec<_> = (0..over)
            .map(|i| (recipient((i % 250) as u8), 1_000u64))
            .collect();
        let result = core
            .build_signed_payment(outputs, None, &signer, None)
            .await;
        match result {
            Err(PlatformWalletError::TransactionBuild(m)) => assert!(
                m.contains("standard") && m.contains("recipients"),
                "the rejection must cite the standard size limit, got {m:?}"
            ),
            other => panic!("{over} recipients must be refused, got {other:?}"),
        }

        // The 25.8k figure from the review is refused by the same bound.
        let huge: Vec<_> = (0..25_835)
            .map(|i| (recipient((i % 250) as u8), 1_000u64))
            .collect();
        assert!(
            matches!(
                core.build_signed_payment(huge, Some(super::MAX_FEE_PER_KB), &signer, None)
                    .await,
                Err(PlatformWalletError::TransactionBuild(_))
            ),
            "the review's 25,835-recipient overflow case must be refused"
        );
    }
}

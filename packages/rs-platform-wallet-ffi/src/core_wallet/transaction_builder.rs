use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::{
    Handle, CORE_SIGNED_TRANSACTION_STORAGE, CORE_WALLET_STORAGE, PLATFORM_WALLET_STORAGE,
};
use crate::runtime::runtime;
use crate::types::{FFINetwork, Network};
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::hashes::Hash;
use dashcore::{Address as DashAddress, OutPoint, TxOut, Txid};
use key_wallet::account::ManagedAccountCollection;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    TransactionBuilder, MAX_STANDARD_OP_RETURN_BYTES,
};
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use platform_wallet::{
    check_fee_rate, input_awaiting_network, is_final, ChosenInputProblem, FinalizeOptions,
    PlatformWalletError,
};
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::str::FromStr;

/// Opaque, C-compatible transaction builder. `inner` is a heap-boxed
/// [`BuilderState`] (the recorded configuration and the inputs chosen by
/// outpoint); `network` is the wallet network output and change addresses
/// are validated against.
///
/// NOT thread-safe: a single builder must be used from one thread at a time.
/// The setters mutate `*inner` in place without synchronization.
#[repr(C)]
pub struct FFITransactionBuilder {
    inner: *mut c_void,
    network: FFINetwork,
    /// Set by `core_wallet_tx_builder_use_only_added_inputs`. key-wallet takes
    /// this per funding call, which the finalizers make internally, so the
    /// intent has to be carried here and read when they run.
    reservation_only: bool,
}

/// What `FFITransactionBuilder::inner` points at — Rust-only, so the C
/// layout stays as it was.
#[derive(Default)]
struct BuilderState {
    /// The setters' calls, in order. The finalizers make the key-wallet
    /// builder from them, and make it again for the waiting-coins trial on a
    /// shortfall (`CoreWallet::finalize_transaction_from`).
    recipe: Vec<Step>,
    /// Set by `core_wallet_tx_builder_add_inputs_from_outpoints`: the
    /// finalizers seed the wallet's current copy of each, so a coin's finality
    /// is judged when the build is finalized, not when it was chosen.
    inputs: Vec<OutPoint>,
}

/// One validated setter call, replayed onto a fresh `TransactionBuilder`.
enum Step {
    AddOutput(DashAddress, u64),
    AddOpReturn(Vec<u8>),
    SetChangeAddress(DashAddress),
    PreserveOutputOrder,
    ChangeToFirstInput,
    SetFeeRate(FeeRate),
    SetSelectionStrategy(SelectionStrategy),
    SetSpecialPayload(TransactionPayload),
}

impl BuilderState {
    /// A fresh key-wallet builder with every recorded call applied. The
    /// setters validated each call before recording it (an OP_RETURN within
    /// the standard size, a fee rate that passes `check_fee_rate`), so the one
    /// fallible step cannot fail here; it is still an error, not a panic.
    fn make(&self) -> Result<TransactionBuilder, PlatformWalletError> {
        self.recipe
            .iter()
            .try_fold(TransactionBuilder::new(), |builder, step| {
                Ok(match step {
                    Step::AddOutput(address, amount) => builder.add_output(address, *amount),
                    Step::AddOpReturn(data) => builder
                        .add_op_return(data)
                        .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?,
                    Step::SetChangeAddress(address) => builder.set_change_address(address.clone()),
                    Step::PreserveOutputOrder => builder.preserve_output_order(),
                    Step::ChangeToFirstInput => builder.change_to_first_input(),
                    Step::SetFeeRate(rate) => builder.set_fee_rate(*rate),
                    Step::SetSelectionStrategy(strategy) => {
                        builder.set_selection_strategy(*strategy)
                    }
                    Step::SetSpecialPayload(payload) => {
                        builder.set_special_payload(payload.clone())
                    }
                })
            })
    }
}

impl FFITransactionBuilder {
    /// Reclaim both heap boxes of a builder a finalizer consumes: the network
    /// it was made for, its recorded configuration and inputs, and how to fund
    /// it.
    ///
    /// # Safety
    /// `builder` must be a live pointer from `core_wallet_tx_builder_new`; it
    /// is consumed.
    unsafe fn into_build(builder: *mut FFITransactionBuilder) -> (FFINetwork, BuildPlan) {
        let ffi = Box::from_raw(builder);
        let mut state = *Box::from_raw(ffi.inner as *mut BuilderState);
        let options = FinalizeOptions::default()
            .with_inputs(std::mem::take(&mut state.inputs))
            .with_reservation_only(ffi.reservation_only);
        (ffi.network, BuildPlan { state, options })
    }

    /// The inner state.
    ///
    /// # Safety
    /// `self.inner` must point at a live `BuilderState`.
    unsafe fn state(&mut self) -> &mut BuilderState {
        &mut *(self.inner as *mut BuilderState)
    }

    /// Record one validated setter call.
    ///
    /// # Safety
    /// `self.inner` must point at a live `BuilderState`.
    unsafe fn record(&mut self, step: Step) {
        self.state().recipe.push(step);
    }
}

/// A reclaimed builder, ready for `CoreWallet::finalize_transaction_from`.
struct BuildPlan {
    state: BuilderState,
    options: FinalizeOptions,
}

/// Owned signed-transaction bytes handed across the C ABI as the `out_tx`
/// of `core_wallet_signed_payment_finalize`; release it with
/// `core_wallet_transaction_free`.
#[repr(C)]
pub struct FFICoreTransaction {
    tx_bytes: *mut u8,
    tx_len: usize,
    // Part of the C ABI (the Swift host reads `FFICoreTransaction.fee`); the
    // Rust side only writes it, so silence the never-read lint.
    #[allow(dead_code)]
    fee: u64,
}

/// Internal value behind the opaque numeric handle. Keeping the originating
/// CoreWallet with the signed transaction lets `free` perform the same safe
/// reservation release as explicit abandon, even after the host discarded its
/// transient CoreWallet handle.
pub struct FFICoreSignedTransaction {
    pub(crate) wallet: platform_wallet::CoreWallet<platform_wallet::broadcaster::SpvBroadcaster>,
    pub(crate) transaction: platform_wallet::SignedCoreTransaction,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub enum CoreAccountTypeFFI {
    BIP44,
    BIP32,
    CoinJoin,
    /// Pool every spendable transparent source: BIP44 + BIP32 + all DashPay
    /// contact-receiving accounts (`platform_wallet::SEND_FUNDING_SOURCES`).
    /// Change returns to BIP44 (the first pooled source). CoinJoin stays out
    /// (separate privacy domain), as do a contact's watch-only external
    /// coins. The default selector for a plain send.
    AllSpendable,
}

impl CoreAccountTypeFFI {
    /// The single account family this selector names, or `None` for the
    /// pooled [`AllSpendable`](Self::AllSpendable) — used by APIs that address
    /// exactly one account (gap limits, per-account UTXO listing), which must
    /// reject the pooled selector with a typed parameter error.
    pub(crate) fn single_preference(self) -> Option<AccountTypePreference> {
        match self {
            CoreAccountTypeFFI::BIP44 => Some(AccountTypePreference::BIP44),
            CoreAccountTypeFFI::BIP32 => Some(AccountTypePreference::BIP32),
            CoreAccountTypeFFI::CoinJoin => Some(AccountTypePreference::CoinJoin),
            CoreAccountTypeFFI::AllSpendable => None,
        }
    }

    /// The funding sources this selector pools, in funding order — handed to
    /// [`CoreWallet::finalize_transaction_from`]'s multi-source API, whose first
    /// source supplies the change address. A single-family selector yields a
    /// one-element list, which keeps that API's strict one-account semantics.
    pub(crate) fn funding_sources(self) -> &'static [AccountTypePreference] {
        match self {
            CoreAccountTypeFFI::BIP44 => &[AccountTypePreference::BIP44],
            CoreAccountTypeFFI::BIP32 => &[AccountTypePreference::BIP32],
            CoreAccountTypeFFI::CoinJoin => &[AccountTypePreference::CoinJoin],
            CoreAccountTypeFFI::AllSpendable => &platform_wallet::SEND_FUNDING_SOURCES,
        }
    }
}

/// Atomically fund, reserve and sign a configured builder.
///
/// Selection and insertion into the account ReservationSet happen under one
/// wallet-manager lock, so they cannot interleave with a competing finalizer.
/// The wallet-manager lock is dropped before the host mnemonic resolver is
/// invoked. This function consumes `builder` on every path after its pointer
/// is accepted.
///
/// On success `out_transaction_handle` receives an opaque finalized-transaction handle. Consume
/// it with `core_wallet_broadcast_signed_transaction` or
/// `core_wallet_abandon_signed_transaction`.
///
/// If the host removes (or re-creates) this wallet while the external signer is
/// running, no handle is published: the build's reservation is reconciled and
/// this returns `NotFound` (98), the same code the deferred-token sibling
/// `core_wallet_signed_payment_finalize` uses for that case.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_tx_builder_finalize(
    builder: *mut FFITransactionBuilder,
    wallet: Handle,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_transaction_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    check_ptr!(core_signer_handle);
    check_ptr!(out_transaction_handle);
    *out_transaction_handle = 0;

    let (network, BuildPlan { state, options }) = FFITransactionBuilder::into_build(builder);
    let wallet = unwrap_option_or_return!(PLATFORM_WALLET_STORAGE.with_item(wallet, |w| w.clone()));

    let builder_network: Network = network.into();
    if builder_network != wallet.network() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "builder network does not match wallet network".to_string(),
        );
    }

    let signer =
        MnemonicResolverCoreSigner::new(core_signer_handle, wallet.wallet_id(), wallet.network());
    let make = move || state.make();
    let finalized = runtime().block_on(wallet.core().finalize_transaction_from(
        make,
        options,
        account_type.funding_sources(),
        account_index,
        &signer,
    ));
    let finalized = unwrap_result_or_return!(finalized);

    // Publishing the finalized handle is gated exactly like the deferred-token sibling
    // below (`core_wallet_signed_payment_finalize`). `finalize_transaction_from` drops
    // the wallet-manager write lock before awaiting the (external, possibly slow)
    // signer, so the host can have removed this wallet while we were signing —
    // and that removal's finalized-handle sweep has then ALREADY run. Inserting now
    // would publish a live handle for a removed generation that no later sweep
    // catches, and `core_wallet_broadcast_signed_transaction` would happily
    // push it to the network: its `is_same_generation` check compares two
    // handles, and a removed generation matches itself (`dashpay/platform#4185`).
    //
    // Hold THIS generation's lifecycle gate across BOTH the liveness check and
    // the insert, so a teardown cannot interleave between them. Acquired AFTER
    // the signer await, never around it: holding it across an open signing prompt
    // would stall this wallet's teardown for as long as the user takes, and the
    // check makes that unnecessary.
    let (_lifecycle, wallet_is_live) = runtime().block_on(async {
        let gate = wallet.core().generation_payment_guard().await;
        let live = wallet.core().is_current_generation().await;
        (gate, live)
    });
    if !wallet_is_live {
        // No handle was published, so nothing would ever release this build's
        // reservation. Reconcile it here: the release is generation-bound, so on
        // a genuine removal it is a logged no-op (the `ReservationSet` died with
        // the generation), and on a re-create it correctly declines to touch the
        // new generation's inputs.
        runtime().block_on(wallet.core().abandon_transaction(&finalized));
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "wallet is no longer registered in the manager (removed or re-created while the \
             transaction was being signed); no transaction handle was published and its \
             reservation was reconciled"
                .to_string(),
        );
    }

    *out_transaction_handle = CORE_SIGNED_TRANSACTION_STORAGE.insert(FFICoreSignedTransaction {
        wallet: wallet.core().clone(),
        transaction: finalized,
    });
    PlatformWalletFFIResult::ok()
}

/// Value of the sole non-OP_RETURN output: what a broadcast of this
/// transaction actually pays out.
///
/// Returns 0 when there is no single such output. A multi-recipient build has
/// no one deliverable amount, and an OP_RETURN-only build pays no one — hosts
/// read the 0 as "not applicable" rather than "pays nothing", so the two cases
/// need not be told apart here.
///
/// Output ORDER is deliberately irrelevant: a MAYAChain deposit carries its
/// memo at VOUT1, while other layouts put the data carrier first.
fn sole_deliverable_value(outputs: &[TxOut]) -> u64 {
    let mut carriers = outputs
        .iter()
        .filter(|out| !out.script_pubkey.is_op_return());
    match (carriers.next(), carriers.next()) {
        (Some(only), None) => only.value,
        _ => 0,
    }
}

/// Atomically fund, reserve, and sign a configured builder for DEFERRED
/// (BIP70/BIP270) submission, then register the built transaction — holding its
/// UTXO reservation — in one native operation.
///
/// This is the deferred counterpart to `core_wallet_tx_builder_finalize`: it
/// runs the same atomic `finalize_transaction_from` (the recorded configuration
/// is replayed for the build, and again for the waiting-coins trial on a
/// shortfall), where selection and insertion
/// into the account `ReservationSet` commit as a single unit under the
/// wallet-manager lock (signing happens after the lock is dropped). Routing the
/// deferred build through it closes the double-selection window the former
/// split fund-then-sign sequence reopened once the Kotlin per-wallet send mutex
/// was removed: two concurrent deferred builds, or a deferred build racing an
/// immediate send, can no longer select the same UTXO. Consumes `builder` on
/// every path after its pointer is accepted.
///
/// Writes `out_token` (the reservation token for a later
/// `core_wallet_signed_payment_broadcast` / `core_wallet_signed_payment_release`),
/// `out_fee` (the build's fee in duffs), `out_txid` (a heap C string freed with
/// `core_wallet_free_address`), and `out_tx` (an owned `FFICoreTransaction`
/// carrying the consensus-serialized bytes, freed with
/// `core_wallet_transaction_free`). `out_bytes_ptr`/`out_bytes_len` borrow
/// `out_tx`'s buffer — copy them out before freeing `out_tx`.
///
/// Also writes `out_deliverable_duffs`: the value of the sole non-OP_RETURN
/// output of the REGISTERED transaction — what a later broadcast actually
/// pays out. Hosts need it for a drain (`SelectionStrategy::All`), where the
/// engine, not the caller, sets that output to `total inputs - fee`; reading it
/// from the registered transaction here keeps a quote and its payment from
/// disagreeing. Writes 0 when there is no single such output (multi-recipient,
/// or an OP_RETURN-only build) — "not applicable", not "pays nothing".
///
/// This is the CURRENT entry point. `core_wallet_signed_payment_finalize` is
/// the pre-existing eleven-argument symbol, kept so already-compiled callers
/// keep linking; it forwards here and discards the amount.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer; `wallet` a valid
/// platform-wallet handle; `core_signer_handle` a valid resolver handle; every
/// out-pointer must be writable. `out_tx` must point at writable storage for one
/// `FFICoreTransaction` (typically zeroed).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_signed_payment_finalize_with_deliverable(
    builder: *mut FFITransactionBuilder,
    wallet: Handle,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_token: *mut u64,
    out_fee: *mut u64,
    out_txid: *mut *mut c_char,
    out_tx: *mut FFICoreTransaction,
    out_bytes_ptr: *mut *const u8,
    out_bytes_len: *mut usize,
    out_deliverable_duffs: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    check_ptr!(core_signer_handle);
    check_ptr!(out_token);
    check_ptr!(out_fee);
    check_ptr!(out_txid);
    check_ptr!(out_tx);
    check_ptr!(out_bytes_ptr);
    check_ptr!(out_bytes_len);
    check_ptr!(out_deliverable_duffs);
    // Publish sentinels into EVERY output before any fallible step (wallet
    // resolution, network validation, signing, registration), so an error
    // return never leaves caller-supplied garbage in an out param that a host
    // could misread as a token, fee, txid, or transaction buffer.
    *out_token = 0;
    *out_fee = 0;
    *out_txid = std::ptr::null_mut();
    *out_tx = FFICoreTransaction {
        tx_bytes: std::ptr::null_mut(),
        tx_len: 0,
        fee: 0,
    };
    *out_bytes_ptr = std::ptr::null();
    *out_bytes_len = 0;
    *out_deliverable_duffs = 0;

    // The finalizer consumes the builder: reclaim both heap boxes up front so
    // they are freed on every return path below. The recorded configuration
    // is replayed by `finalize_transaction_from`, possibly twice.
    let (network, BuildPlan { state, options }) = FFITransactionBuilder::into_build(builder);

    let wallet = unwrap_option_or_return!(PLATFORM_WALLET_STORAGE.with_item(wallet, |w| w.clone()));

    let builder_network: Network = network.into();
    if builder_network != wallet.network() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "builder network does not match wallet network".to_string(),
        );
    }

    let signer =
        MnemonicResolverCoreSigner::new(core_signer_handle, wallet.wallet_id(), wallet.network());

    // Atomic select + reserve + sign in one wallet-manager critical section.
    // `options` (the `reservation_only` flag, the inputs chosen by outpoint and
    // the recorded configuration) is read off the reclaimed boxes by `into_build`
    // (never through `builder`, whose provenance ends at `Box::from_raw`) and
    // threaded through here for the same reason as in the immediate sibling:
    // a host that called
    // `core_wallet_tx_builder_use_only_added_inputs` and then finalized a
    // DEFERRED payment would otherwise have the restriction silently discarded,
    // and the account's UTXOs would be offered to selection after all.
    let make = move || state.make();
    let finalized = runtime().block_on(wallet.core().finalize_transaction_from(
        make,
        options,
        account_type.funding_sources(),
        account_index,
        &signer,
    ));
    let finalized = unwrap_result_or_return!(finalized);

    // `finalize_transaction_from` drops the wallet-manager write lock before awaiting
    // the (external, possibly slow) signer, so the host can have removed this
    // wallet while we were signing — and that removal's registry sweep has then
    // ALREADY run. Registering now would insert a live token for a removed
    // generation, which no later sweep would catch, defeating the teardown
    // invariant that dropping tokens makes stale handles inert
    // (`dashpay/platform#4185`).
    //
    // Take THIS wallet generation's lifecycle gate (shared — concurrent payments
    // are unaffected) and hold it across BOTH the liveness check and the
    // synchronous `register`, so a teardown cannot interleave between them.
    // Deliberately acquired AFTER the signer await rather than around it: holding
    // it across an open signing prompt would stall this wallet's teardown for as
    // long as the user takes, and the check below makes that unnecessary.
    let (_lifecycle, wallet_is_live) = runtime().block_on(async {
        let gate = wallet.core().generation_payment_guard().await;
        let live = wallet.core().is_current_generation().await;
        (gate, live)
    });
    if !wallet_is_live {
        // Nothing was registered, so no token would ever release this build's
        // reservation. Reconcile it here: the release is generation-bound, so on
        // a genuine removal it is a logged no-op (the `ReservationSet` died with
        // the generation), and on a re-create it correctly declines to touch the
        // new generation's inputs.
        runtime().block_on(wallet.core().abandon_transaction(&finalized));
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "wallet is no longer registered in the manager (removed or re-created while the \
             payment was being signed); the payment was not registered and its reservation was \
             reconciled"
                .to_string(),
        );
    }

    let txid = finalized.transaction().txid();
    let fee = finalized.fee();

    // Do the one fallible marshalling step BEFORE the registry insert: that
    // insert mints a token and keeps the funding reservation held, so a later
    // failure would orphan the reservation with no token to release it. txid hex
    // never contains a NUL, but handle the impossible case anyway.
    let c_txid = match CString::new(txid.to_string()) {
        Ok(s) => s,
        Err(_) => {
            // Nothing registered yet — release the reservation finalize took.
            runtime().block_on(wallet.core().abandon_transaction(&finalized));
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                "txid string contained an interior NUL".to_string(),
            );
        }
    };

    // The deliverable amount, taken from the transaction that will actually be
    // broadcast — not re-derived by the host from a copy of the bytes. Under a
    // drain the ENGINE sets this output (total inputs - fee), so the caller
    // never supplied it and has no other authoritative source; a host that
    // re-parsed its own byte array could quote a value the broadcast does not
    // pay. Defined only for a single-destination payment: exactly one output
    // that is not an OP_RETURN data carrier. Anything else reports 0, which the
    // host reads as "not applicable" rather than "pays nothing".
    let deliverable_duffs = sole_deliverable_value(&finalized.transaction().output);
    unsafe { *out_deliverable_duffs = deliverable_duffs };

    let serialized = dashcore::consensus::serialize(finalized.transaction());
    let len = serialized.len();

    // Register the reserved+signed tx for deferred submission. `finalize` already
    // committed the reservation; `register` CONSUMES the `SignedCoreTransaction`
    // ownership object (deriving its transaction, funding account, reservation
    // height, and owner-guard token internally) and binds the token to the wallet
    // whose `ReservationSet` holds the inputs. Because the object is consumed
    // exactly once, this finalize can yield at most one token — no second token
    // can ever name the same reservation (`dashpay/platform#4185`, blocker 1).
    //
    // `register` is SYNCHRONOUS: its reservation-owning insert runs inline with
    // no future that could be dropped before its first poll and silently strand
    // the consumed reservation (`dashpay/platform#4185`). It also validates that
    // this wallet is the exact generation `finalize` bound the payment to; that
    // always holds here (we register through the very wallet that finalized), but
    // on the impossible mismatch it hands the finalized payment back so we
    // release its reservation (owner-guarded) rather than leaking it.
    let token = match crate::core_wallet::signed_payment::SIGNED_PAYMENT_REGISTRY
        .register(wallet.core().clone(), finalized)
    {
        Ok(token) => token,
        Err(err) => {
            runtime().block_on(wallet.core().abandon_transaction(&err.signed));
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorReservationWalletMismatch,
                "deferred payment was finalized against a different wallet generation".to_string(),
            );
        }
    };

    *out_tx = FFICoreTransaction {
        tx_bytes: Box::into_raw(serialized.into_boxed_slice()) as *mut u8,
        tx_len: len,
        fee,
    };
    *out_token = token.as_u64();
    *out_fee = fee;
    *out_txid = c_txid.into_raw();
    // Borrowed view into the just-written `out_tx` buffer; the caller copies the
    // bytes out before freeing `out_tx` with `core_wallet_transaction_free`.
    *out_bytes_ptr = (*out_tx).tx_bytes as *const u8;
    *out_bytes_len = len;
    PlatformWalletFFIResult::ok()
}

/// The pre-existing ELEVEN-argument finalize, preserved byte-for-byte in its
/// C signature. Forwards to
/// [`core_wallet_signed_payment_finalize_with_deliverable`] and discards the
/// deliverable amount; behaviour is otherwise identical.
///
/// Kept because this symbol is exported across a BINARY boundary: the Swift SDK
/// consumes `DashSDKFFI.xcframework` as a `binaryTarget`, so a host's compiled
/// Swift and this library are built and shipped separately and can meet at
/// different versions. Adding the twelfth out-parameter to this symbol in place
/// would make the callee write eight bytes through a pointer an eleven-argument
/// caller never passed — reading whatever occupied that argument slot and
/// treating it as an address. That corrupts silently rather than failing, so the
/// old shape stays, and callers that want the amount move to the new symbol.
///
/// Do not "simplify" this away by deleting it and updating the in-tree callers:
/// the callers that matter here are already-compiled binaries, which no
/// source-tree edit can reach.
///
/// # Safety
/// Identical to [`core_wallet_signed_payment_finalize_with_deliverable`], minus
/// `out_deliverable_duffs` (supplied internally).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_signed_payment_finalize(
    builder: *mut FFITransactionBuilder,
    wallet: Handle,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_token: *mut u64,
    out_fee: *mut u64,
    out_txid: *mut *mut c_char,
    out_tx: *mut FFICoreTransaction,
    out_bytes_ptr: *mut *const u8,
    out_bytes_len: *mut usize,
) -> PlatformWalletFFIResult {
    // A real local, never null: the callee null-checks every out-pointer and
    // would reject the call outright.
    let mut discarded_deliverable_duffs: u64 = 0;
    core_wallet_signed_payment_finalize_with_deliverable(
        builder,
        wallet,
        account_type,
        account_index,
        core_signer_handle,
        out_token,
        out_fee,
        out_txid,
        out_tx,
        out_bytes_ptr,
        out_bytes_len,
        &mut discarded_deliverable_duffs,
    )
}

#[repr(C)]
pub enum CoreSelectionStrategyFFI {
    SmallestFirst,
    LargestFirst,
    BranchAndBound,
    OptimalConsolidation,
    Random,
    All,
}

impl From<CoreSelectionStrategyFFI> for SelectionStrategy {
    fn from(value: CoreSelectionStrategyFFI) -> Self {
        match value {
            CoreSelectionStrategyFFI::SmallestFirst => SelectionStrategy::SmallestFirst,
            CoreSelectionStrategyFFI::LargestFirst => SelectionStrategy::LargestFirst,
            CoreSelectionStrategyFFI::BranchAndBound => SelectionStrategy::BranchAndBound,
            CoreSelectionStrategyFFI::OptimalConsolidation => {
                SelectionStrategy::OptimalConsolidation
            }
            CoreSelectionStrategyFFI::Random => SelectionStrategy::Random,
            CoreSelectionStrategyFFI::All => SelectionStrategy::All,
        }
    }
}

fn managed_account(
    accounts: &ManagedAccountCollection,
    source: AccountTypePreference,
    account_index: u32,
) -> Option<&ManagedCoreFundsAccount> {
    source
        .account_type(account_index)
        .and_then(|at| accounts.funds_account(&at))
}

/// Create a new transaction builder for `network`. Free with
/// `core_wallet_tx_builder_destroy` (or the consuming finalizers
/// `core_wallet_tx_builder_finalize` / `core_wallet_signed_payment_finalize`).
///
/// # Safety
/// The returned pointer is owned by the caller.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_new(
    network: FFINetwork,
) -> *mut FFITransactionBuilder {
    let inner = Box::into_raw(Box::<BuilderState>::default()) as *mut c_void;
    Box::into_raw(Box::new(FFITransactionBuilder {
        inner,
        network,
        reservation_only: false,
    }))
}

/// # Safety
/// `builder` must be a valid, non-destroyed pointer; `address` a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_add_output(
    builder: *mut FFITransactionBuilder,
    address: *const c_char,
    amount: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    check_ptr!(address);

    let addr_str = unwrap_result_or_return!(std::ffi::CStr::from_ptr(address).to_str());
    let network: Network = (*builder).network.into();
    let parsed = unwrap_result_or_return!(DashAddress::from_str(addr_str));
    let address = match parsed.require_network(network) {
        Ok(a) => a,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("output address network mismatch: {e}"),
            );
        }
    };

    (*builder).record(Step::AddOutput(address, amount));

    PlatformWalletFFIResult::ok()
}

/// Add a zero-value OP_RETURN output carrying `data`.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer; `data` must reference a
/// readable buffer of `data_len` bytes when `data_len > 0`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_add_op_return(
    builder: *mut FFITransactionBuilder,
    data: *const u8,
    data_len: usize,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    if data_len > 0 {
        check_ptr!(data);
    }

    let bytes = if data_len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(data, data_len)
    };

    // The one thing `add_op_return` refuses, by the same policy constant:
    // checked here, so a recorded step always replays.
    if data_len > MAX_STANDARD_OP_RETURN_BYTES {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "OP_RETURN payload too large: {data_len} bytes (max {MAX_STANDARD_OP_RETURN_BYTES})"
            ),
        );
    }

    (*builder).record(Step::AddOpReturn(bytes.to_vec()));

    PlatformWalletFFIResult::ok()
}

/// # Safety
/// `builder` must be a valid, non-destroyed pointer; `address` a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_set_change_address(
    builder: *mut FFITransactionBuilder,
    address: *const c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    check_ptr!(address);

    let addr_str = unwrap_result_or_return!(std::ffi::CStr::from_ptr(address).to_str());
    let network: Network = (*builder).network.into();
    let parsed = unwrap_result_or_return!(DashAddress::from_str(addr_str));
    let address = match parsed.require_network(network) {
        Ok(a) => a,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("change address network mismatch: {e}"),
            );
        }
    };

    (*builder).record(Step::SetChangeAddress(address));

    PlatformWalletFFIResult::ok()
}

/// Preserve outputs in the order they were added instead of applying BIP-69 sorting.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_preserve_output_order(
    builder: *mut FFITransactionBuilder,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);

    (*builder).record(Step::PreserveOutputOrder);

    PlatformWalletFFIResult::ok()
}

/// Route change to the address of the first selected input (VIN0).
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_change_to_first_input(
    builder: *mut FFITransactionBuilder,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);

    (*builder).record(Step::ChangeToFirstInput);

    PlatformWalletFFIResult::ok()
}

/// Set the fee rate in duffs per kB. Refused with `ErrorInvalidParameter` when
/// the rate's fee arithmetic would overflow (above about 42.9 DASH per kB —
/// key-wallet multiplies rate by size unchecked); the builder is unchanged.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_set_fee_rate(
    builder: *mut FFITransactionBuilder,
    sat_per_kb: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);

    // key-wallet prices with the rate unchecked: refuse one whose fee
    // arithmetic overflows here, where the host set it.
    let rate = FeeRate::new(sat_per_kb);
    if let Err(error) = check_fee_rate(rate) {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            error.to_string(),
        );
    }
    (*builder).record(Step::SetFeeRate(rate));

    PlatformWalletFFIResult::ok()
}

/// Fund the build from the inputs `core_wallet_tx_builder_add_inputs_from_outpoints`
/// supplied, and nothing else.
///
/// Without this, the wallet-aware finalizers offer every unreserved UTXO of the
/// funding account alongside the seeded ones, so seeding a subset does not
/// restrict what gets selected. A caller draining an account in batches that
/// each stay under the standard-transaction input limit needs this, or every
/// batch sees the whole account and fails with a too-many-inputs error.
///
/// Honoured by BOTH finalizers — `core_wallet_tx_builder_finalize` and the
/// deferred `core_wallet_signed_payment_finalize` — so the restriction cannot
/// be lost by picking one submission path over the other. It only removes
/// candidates: the account still takes on the build's reservation bookkeeping
/// and still supplies its change address.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_use_only_added_inputs(
    builder: *mut FFITransactionBuilder,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    (*builder).reservation_only = true;
    PlatformWalletFFIResult::ok()
}

/// The balance a build funded by `account_type` could actually select from — the
/// same accounts `core_wallet_tx_builder_finalize` would fund from, counting
/// only UTXOs coin selection accepts.
///
/// Gate amount entry on this rather than on `core_wallet_get_balance`, which
/// sums every funding account the wallet has — CoinJoin included — and so
/// reports money a build then refuses.
///
/// Reservations are not subtracted; see `CoreWallet::pooled_spendable_balance`.
///
/// `core_wallet` is the handle `platform_wallet_get_core` returns — the same
/// one `core_wallet_get_balance` takes — NOT the platform-wallet handle the
/// `core_wallet_tx_builder_*` entry points above take. Handles are drawn from
/// one global counter, so a core handle looked up in the platform table (or
/// vice versa) is always `NotFound`, never a wrong wallet.
///
/// # Safety
/// `out_balance` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_pooled_spendable_balance(
    core_wallet: Handle,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    out_balance: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_balance);
    *out_balance = 0;

    let core = unwrap_option_or_return!(CORE_WALLET_STORAGE.with_item(core_wallet, |w| w.clone()));
    let balance = unwrap_result_or_return!(runtime()
        .block_on(core.pooled_spendable_balance(account_type.funding_sources(), account_index)));

    *out_balance = balance;
    PlatformWalletFFIResult::ok()
}

/// The largest amount a build funded by `account_type` could actually pay out,
/// net of the fee spending it costs — what a "send max" control must use.
///
/// `core_wallet_pooled_spendable_balance` is the gross figure: entering it
/// verbatim as an amount fails, because a build needs `amount + fee`. This
/// prices the fee off the inputs that spending everything would take, at
/// `fee_rate_sat_per_kb` — pass 0 for the same default `TransactionBuilder`
/// starts from, or the rate the host sets on its builders.
///
/// `core_wallet` is the core-wallet handle, as for
/// `core_wallet_pooled_spendable_balance`.
///
/// # Safety
/// `out_max_sendable` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_pooled_max_sendable(
    core_wallet: Handle,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    fee_rate_sat_per_kb: u64,
    out_max_sendable: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_max_sendable);
    *out_max_sendable = 0;

    let fee_rate = (fee_rate_sat_per_kb != 0).then(|| FeeRate::new(fee_rate_sat_per_kb));
    let core = unwrap_option_or_return!(CORE_WALLET_STORAGE.with_item(core_wallet, |w| w.clone()));
    let max_sendable = unwrap_result_or_return!(runtime().block_on(core.pooled_max_sendable(
        account_type.funding_sources(),
        account_index,
        fee_rate
    )));

    *out_max_sendable = max_sendable;
    PlatformWalletFFIResult::ok()
}

/// # Safety
/// `builder` must be a valid, non-destroyed pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_set_selection_strategy(
    builder: *mut FFITransactionBuilder,
    strategy: CoreSelectionStrategyFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);

    (*builder).record(Step::SetSelectionStrategy(strategy.into()));

    PlatformWalletFFIResult::ok()
}

/// Set the block height coin selection treats as the chain tip (used for
/// coinbase maturity and locktime).
///
/// Accepted and ignored: the finalizers (the only way this builder is
/// built) always build at the wallet's last processed height, so a height
/// set here never reached a build.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_set_current_height(
    builder: *mut FFITransactionBuilder,
    height: u32,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    let _ = height;

    PlatformWalletFFIResult::ok()
}

/// `payload_bytes` is a bincode-encoded `TransactionPayload`.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer; `payload_bytes` a readable buffer of
/// `payload_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_set_special_payload(
    builder: *mut FFITransactionBuilder,
    payload_bytes: *const u8,
    payload_len: usize,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    check_ptr!(payload_bytes);

    let bytes = std::slice::from_raw_parts(payload_bytes, payload_len);
    // Upstream TransactionPayload does not implement DecodeUntrusted yet.
    // Its derived decoders can reserve collections from their length headers,
    // so use Core's existing decode limit until those decoders opt in.
    let config =
        bincode::config::standard().with_limit::<{ dashcore::consensus::encode::MAX_VEC_SIZE }>();
    let payload: TransactionPayload = match bincode::decode_from_slice(bytes, config) {
        Ok((p, consumed)) => {
            if consumed != payload_len {
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorDeserialization,
                    format!(
                        "trailing bytes after payload: decoded {consumed} of {payload_len} bytes"
                    ),
                );
            }
            p
        }
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorDeserialization,
                format!("invalid special payload: {e}"),
            );
        }
    };

    (*builder).record(Step::SetSpecialPayload(payload));

    PlatformWalletFFIResult::ok()
}

/// Add a caller-chosen subset of the account's UTXOs as inputs. `outpoints`
/// are selected from the account's own UTXO set (the same ones
/// `platform_wallet_account_utxos` returns). An outpoint not owned by the
/// account is an error, and so is one not final yet (code 59, naming it).
///
/// The finalizers look each recorded outpoint up again, in the accounts THEY
/// fund from (their own account type and index), and use the wallet's copy as
/// it is then. A chosen coin is never dropped silently: one those accounts
/// don't hold (chosen from another account, or spent since) fails the build
/// by name, with or without `core_wallet_tx_builder_use_only_added_inputs`;
/// one that lost its final status fails with code 59, one an in-flight
/// broadcast pins with an input-mid-broadcast error. The one exception: a coin
/// another in-flight build holds reserved (a pending deferred payment, say)
/// is left out by the reservation filter, as reservations are not readable
/// here.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer; `wallet` a valid platform-wallet handle;
/// `outpoints` a readable array of `outpoints_len` elements.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_add_inputs_from_outpoints(
    builder: *mut FFITransactionBuilder,
    wallet: Handle,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    outpoints: *const OutPointFFI,
    outpoints_len: usize,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);

    let wallet = unwrap_option_or_return!(PLATFORM_WALLET_STORAGE.with_item(wallet, |w| w.clone()));

    // Reject a builder created for a different network than the wallet, matching
    // the wallet-aware finalizers so every wallet-aware entry point fails fast
    // instead of mutating a foreign-network builder.
    let builder_network: Network = (*builder).network.into();
    if builder_network != wallet.network() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "builder network does not match wallet network".to_string(),
        );
    }

    let wallet_id = wallet.wallet_id();
    let Some(source) = account_type.single_preference() else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "AllSpendable pools multiple accounts; this API addresses exactly one".to_string(),
        );
    };

    let requested: Vec<OutPoint> = if outpoints_len == 0 {
        Vec::new()
    } else {
        check_ptr!(outpoints);
        std::slice::from_raw_parts(outpoints, outpoints_len)
            .iter()
            .map(|op| OutPoint {
                txid: Txid::from_byte_array(op.txid),
                vout: op.vout,
            })
            .collect()
    };

    let result = runtime().block_on(async {
        let wm = wallet.wallet_manager().read().await;
        let info = wm
            .get_wallet_info(&wallet_id)
            .ok_or_else(|| SeedError::Other("wallet not found".to_string()))?;

        let managed = managed_account(&info.core_wallet.accounts, source, account_index)
            .ok_or_else(|| {
                SeedError::Other(format!(
                    "managed account {source:?} #{account_index} not found"
                ))
            })?;

        let height = info.core_wallet.last_processed_height();
        let mut selected = Vec::with_capacity(requested.len());
        for op in &requested {
            let utxo = managed.utxos.get(op).ok_or_else(|| {
                SeedError::Other(format!("outpoint {}:{} not in account", op.txid, op.vout))
            })?;
            // In the native finalizer's order: an unspendable coin (an
            // immature coinbase output, a locked coin) first — confirmation
            // would not make it usable, so code 59 would mislead.
            if !utxo.is_spendable(height) {
                return Err(SeedError::Refused(
                    PlatformWalletError::ChosenInputUnavailable {
                        outpoint: *op,
                        problem: ChosenInputProblem::NotSpendable,
                    },
                ));
            }
            // Builds spend only final coins (InstantSend-locked or mined):
            // refuse one that is not, by name, as soon as it is named. The
            // finalizers check again against the coin as it is then.
            if !is_final(utxo) {
                return Err(SeedError::Refused(input_awaiting_network(utxo)));
            }
            selected.push(*op);
        }

        // Validation succeeded — only now record them.
        (*builder).state().inputs.extend(selected);
        Ok::<_, SeedError>(())
    });

    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(SeedError::Refused(error)) => PlatformWalletFFIResult::from(error),
        Err(SeedError::Other(e)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("add_inputs_from_outpoints failed: {e}"),
        ),
    }
}

/// Why `core_wallet_tx_builder_add_inputs_from_outpoints` refused its outpoints.
enum SeedError {
    /// A named coin is refused with a typed error: unspendable
    /// (`ChosenInputUnavailable`, `ErrorInvalidParameter`) or not final yet
    /// (code 59).
    Refused(PlatformWalletError),
    Other(String),
}

/// Destroy a transaction builder created by `core_wallet_tx_builder_new`.
///
/// # Safety
/// `builder` must not have already been destroyed or built (or null).
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_destroy(builder: *mut FFITransactionBuilder) {
    if builder.is_null() {
        return;
    }

    let b = Box::from_raw(builder);
    drop(Box::from_raw(b.inner as *mut BuilderState));
}

/// Free a transaction written by `core_wallet_signed_payment_finalize`.
/// Idempotent: the fields are nulled, so a second call is a no-op.
///
/// # Safety
/// `tx` must be a valid pointer to an `FFICoreTransaction` from
/// `core_wallet_signed_payment_finalize` (or null).
#[no_mangle]
pub unsafe extern "C" fn core_wallet_transaction_free(tx: *mut FFICoreTransaction) {
    if tx.is_null() {
        return;
    }

    let tx = &mut *tx;
    if !tx.tx_bytes.is_null() && tx.tx_len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(tx.tx_bytes, tx.tx_len));
    }

    tx.tx_bytes = std::ptr::null_mut();
    tx.tx_len = 0;
}

#[cfg(test)]
mod payload_decode_tests {
    use super::*;
    use dashcore::blockdata::transaction::special_transaction::asset_lock::AssetLockPayload;
    use std::ffi::CStr;

    #[test]
    fn should_reject_unbounded_payload_collection_and_keep_builder_usable() {
        let payload = TransactionPayload::AssetLockPayloadType(AssetLockPayload::new(vec![]));
        let valid = bincode::encode_to_vec(payload, bincode::config::standard())
            .expect("encode an asset lock payload");
        let mut malformed = valid.clone();
        // Replace the empty credit_outputs count with an oversized declaration,
        // without supplying any output bytes.
        assert_eq!(malformed.pop(), Some(0));
        malformed.extend(
            bincode::encode_to_vec(u64::MAX, bincode::config::standard())
                .expect("encode a collection length"),
        );

        unsafe {
            let builder = core_wallet_tx_builder_new(FFINetwork::Testnet);
            let rejected = core_wallet_tx_builder_set_special_payload(
                builder,
                malformed.as_ptr(),
                malformed.len(),
            );
            let accepted =
                core_wallet_tx_builder_set_special_payload(builder, valid.as_ptr(), valid.len());
            core_wallet_tx_builder_destroy(builder);

            assert_eq!(
                rejected.code,
                PlatformWalletFFIResultCode::ErrorDeserialization
            );
            assert!(CStr::from_ptr(rejected.message)
                .to_str()
                .expect("UTF-8 error message")
                .contains("LimitExceeded"));
            assert_eq!(accepted.code, PlatformWalletFFIResultCode::Success);
        }
    }
}

#[cfg(test)]
mod pooled_balance_handle_tests {
    //! The two pooled-balance entry points are exposed on the *core* wallet
    //! (`ManagedCoreWallet` in the Swift SDK), so they must resolve the handle
    //! `platform_wallet_get_core` hands out — the `CORE_WALLET_STORAGE` one —
    //! not the platform-wallet handle their `core_wallet_tx_builder_*`
    //! neighbours take. Looking a core handle up in the platform table failed
    //! every call with `ErrorInvalidHandle`; the Swift caller swallowed it and
    //! published a permanent 0, which zeroed Max and blocked every send on the
    //! 2026-09-03 QA build (dashwallet-ios#1107 / platform#4582).

    use dashcore::blockdata::transaction::special_transaction::asset_lock::AssetLockPayload;
    use key_wallet::account::account_type::StandardAccountType;
    use platform_wallet::test_support::{
        funded_spv_core_wallet, funded_spv_core_wallet_with_outputs, standard_account_coins,
    };
    use platform_wallet::SEND_FUNDING_SOURCES;

    use super::*;

    #[test]
    fn pooled_spendable_balance_resolves_the_core_wallet_handle() {
        let (core, _signer) =
            runtime().block_on(funded_spv_core_wallet(StandardAccountType::BIP44Account));
        let expected = runtime()
            .block_on(core.pooled_spendable_balance(&SEND_FUNDING_SOURCES, 0))
            .expect("direct pooled balance");
        assert!(
            expected > 0,
            "the helper funds BIP44, so the pool is non-empty"
        );

        let core_handle = CORE_WALLET_STORAGE.insert(core.clone());
        let mut out: u64 = 0;
        let result = unsafe {
            core_wallet_pooled_spendable_balance(
                core_handle,
                CoreAccountTypeFFI::AllSpendable,
                0,
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out, expected);
        CORE_WALLET_STORAGE.remove(core_handle);
    }

    #[test]
    fn pooled_max_sendable_resolves_the_core_wallet_handle() {
        let (core, _signer) =
            runtime().block_on(funded_spv_core_wallet(StandardAccountType::BIP44Account));
        let expected = runtime()
            .block_on(core.pooled_max_sendable(&SEND_FUNDING_SOURCES, 0, None))
            .expect("direct pooled max");
        assert!(expected > 0);

        let core_handle = CORE_WALLET_STORAGE.insert(core.clone());
        let mut out: u64 = 0;
        let result = unsafe {
            core_wallet_pooled_max_sendable(
                core_handle,
                CoreAccountTypeFFI::AllSpendable,
                0,
                0,
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out, expected);
        CORE_WALLET_STORAGE.remove(core_handle);
    }

    /// A handle that was never issued for a core wallet is refused, and the
    /// out-parameter is left at 0 rather than at a stale value.
    #[test]
    fn unknown_handle_is_refused_with_zero_out() {
        let mut out: u64 = 7;
        let result = unsafe {
            core_wallet_pooled_spendable_balance(
                Handle::MAX,
                CoreAccountTypeFFI::AllSpendable,
                0,
                &mut out,
            )
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out, 0);
    }

    /// Finalize what the FFI setters built, the way the finalizers do: from
    /// the recorded configuration, so a shortfall gets the waiting-coins trial.
    fn finalize_built(
        builder: *mut FFITransactionBuilder,
        core: &platform_wallet::CoreWallet<platform_wallet::broadcaster::SpvBroadcaster>,
        signer: &platform_wallet::test_support::WalletSigner,
    ) -> Result<platform_wallet::SignedCoreTransaction, PlatformWalletError> {
        let (_, BuildPlan { state, options }) =
            unsafe { FFITransactionBuilder::into_build(builder) };
        let make = move || state.make();
        runtime().block_on(core.finalize_transaction_from(
            make,
            options,
            CoreAccountTypeFFI::AllSpendable.funding_sources(),
            0,
            signer,
        ))
    }

    fn payment(fee_rate: Option<u64>, amount: u64) -> *mut FFITransactionBuilder {
        let builder = unsafe { core_wallet_tx_builder_new(FFINetwork::Testnet) };
        if let Some(rate) = fee_rate {
            let set = unsafe { core_wallet_tx_builder_set_fee_rate(builder, rate) };
            assert_eq!(set.code, PlatformWalletFFIResultCode::Success);
        }
        let address =
            CString::new(DashAddress::dummy(Network::Testnet, 90).to_string()).expect("address");
        let added = unsafe { core_wallet_tx_builder_add_output(builder, address.as_ptr(), amount) };
        assert_eq!(added.code, PlatformWalletFFIResultCode::Success);
        builder
    }

    /// The app's Send right after a send whose broadcast outcome is unknown:
    /// the wallet's only coin is that send's change, not final yet. A second
    /// payment built the way the app builds it — one `add_output`, default
    /// rate, the pooled `AllSpendable` sources — is refused with code 59 and
    /// builds nothing.
    #[test]
    fn should_refuse_a_second_payment_on_an_unknown_sends_change_with_code_59() {
        let change = 8_999_774;
        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[change],
            &[change],
        ));
        let result = finalize_built(payment(None, 1_000_000), &core, &signer);
        assert!(
            matches!(
                &result,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork {
                    available: Some(0),
                    waiting,
                    outpoint: None,
                    ..
                }) if *waiting == change
            ),
            "got {result:?}"
        );
        let error = result.expect_err("refused");
        assert_eq!(
            PlatformWalletFFIResult::from(error).code,
            PlatformWalletFFIResultCode::ErrorCoreFundsAwaitingNetwork
        );
    }

    /// 900,000 final + 101,000 waiting against a 1,000,000 payment. At the
    /// rate the builder set, 10,000 duffs/kB, the waiting coin nets too little
    /// once its input and the fee are paid: insufficient funds, not code 59.
    /// The same build at the default rate would be covered by waiting.
    #[test]
    fn should_price_a_waiting_shortfall_at_the_fee_rate_the_builder_set() {
        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[900_000, 101_000],
            &[101_000],
        ));

        let high = finalize_built(payment(Some(10_000), 1_000_000), &core, &signer);
        assert!(
            matches!(
                high,
                Err(PlatformWalletError::CoreInsufficientFunds { .. }
                    | PlatformWalletError::CorePooledInsufficientFunds { .. })
            ),
            "got {high:?}"
        );

        let default = finalize_built(payment(None, 1_000_000), &core, &signer);
        assert!(
            matches!(
                default,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ),
            "got {default:?}"
        );
    }

    /// The shortfall is priced at the transaction's serialized shape, every
    /// explicit output counted. 900,000 final + 100,400 waiting against four
    /// 250,000 outputs: once confirmed, the two inputs and four outputs need a
    /// 442-duff fee with no change and only 400 are left, so this is
    /// insufficient funds. The same 1,000,000 in one output is covered.
    #[test]
    fn should_price_a_waiting_shortfall_at_every_output_the_builder_added() {
        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[900_000, 100_400],
            &[100_400],
        ));

        let four = payment(None, 250_000);
        for tag in 91..94 {
            let address = CString::new(DashAddress::dummy(Network::Testnet, tag).to_string())
                .expect("address");
            let added =
                unsafe { core_wallet_tx_builder_add_output(four, address.as_ptr(), 250_000) };
            assert_eq!(added.code, PlatformWalletFFIResultCode::Success);
        }
        let four = finalize_built(four, &core, &signer);
        assert!(
            matches!(
                four,
                Err(PlatformWalletError::CoreInsufficientFunds { .. }
                    | PlatformWalletError::CorePooledInsufficientFunds { .. })
            ),
            "got {four:?}"
        );

        let one = finalize_built(payment(None, 1_000_000), &core, &signer);
        assert!(
            matches!(
                one,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ),
            "got {one:?}"
        );
    }

    fn is_insufficient(
        result: &Result<platform_wallet::SignedCoreTransaction, PlatformWalletError>,
    ) -> bool {
        matches!(
            result,
            Err(PlatformWalletError::CoreInsufficientFunds { .. }
                | PlatformWalletError::CorePooledInsufficientFunds { .. })
        )
    }

    /// A drain of a lone coin that is not final, at 10,000 duffs/kB (a
    /// 1,920-duff fee): 1,600 duffs do not even pay the fee, and 2,400 pay it
    /// but leave 480 — under the 546-duff dust floor key-wallet's drain
    /// requires — so both are insufficient funds even once confirmed. 100,000
    /// duffs drain fine once confirmed: waiting.
    #[test]
    fn should_judge_a_drain_shortfall_by_its_fee_and_dust() {
        for (coin, waiting) in [(1_600, false), (2_400, false), (100_000, true)] {
            let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
                StandardAccountType::BIP44Account,
                &[coin],
                &[coin],
            ));
            let drain = payment(Some(10_000), 0);
            let set = unsafe {
                core_wallet_tx_builder_set_selection_strategy(drain, CoreSelectionStrategyFFI::All)
            };
            assert_eq!(set.code, PlatformWalletFFIResultCode::Success);
            let result = finalize_built(drain, &core, &signer);
            if waiting {
                assert!(
                    matches!(
                        result,
                        Err(PlatformWalletError::CoreFundsAwaitingNetwork { waiting, .. })
                            if waiting == coin
                    ),
                    "coin {coin}: got {result:?}"
                );
            } else {
                assert!(
                    !matches!(
                        result,
                        Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
                    ) && result.is_err(),
                    "coin {coin}: got {result:?}"
                );
            }
        }
    }

    /// A drain cannot leave an uneconomic coin out, so its feasibility is the
    /// gross value of every coin against the fee of spending all of them.
    /// Two non-final outputs of one transaction, 2,467 and 600 duffs, drained
    /// at 10,000 duffs/kB: once they confirm (together) the two-input
    /// transaction's 3,400-duff fee alone exceeds their 3,067 duffs —
    /// insufficient funds, not code 59. Coin by coin, net of each input's fee
    /// (987 + 0 against a 986-duff one-output reserve), it looked covered.
    #[test]
    fn should_judge_a_drain_by_the_gross_value_of_every_coin() {
        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[2_467, 600],
            &[2_467, 600],
        ));
        let drain = payment(Some(10_000), 0);
        let set = unsafe {
            core_wallet_tx_builder_set_selection_strategy(drain, CoreSelectionStrategyFFI::All)
        };
        assert_eq!(set.code, PlatformWalletFFIResultCode::Success);

        let result = finalize_built(drain, &core, &signer);
        assert!(is_insufficient(&result), "got {result:?}");
    }

    /// key-wallet checks a drain's inputs against its placeholder amount
    /// before draining: a 1,000,000-duff placeholder over one non-final
    /// 100,000-duff coin is insufficient funds, not code 59.
    #[test]
    fn should_hold_a_drain_to_its_placeholder_amount() {
        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[100_000],
            &[100_000],
        ));
        let drain = payment(None, 1_000_000);
        let set = unsafe {
            core_wallet_tx_builder_set_selection_strategy(drain, CoreSelectionStrategyFFI::All)
        };
        assert_eq!(set.code, PlatformWalletFFIResultCode::Success);

        let result = finalize_built(drain, &core, &signer);
        assert!(is_insufficient(&result), "got {result:?}");
    }

    /// A drain key-wallet would refuse on its shape (two value outputs) is not
    /// called waiting, however much is unconfirmed.
    #[test]
    fn should_not_call_a_misshapen_drain_waiting() {
        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[100_000],
            &[100_000],
        ));
        let drain = payment(None, 0);
        let address =
            CString::new(DashAddress::dummy(Network::Testnet, 96).to_string()).expect("address");
        let added = unsafe { core_wallet_tx_builder_add_output(drain, address.as_ptr(), 0) };
        assert_eq!(added.code, PlatformWalletFFIResultCode::Success);
        let set = unsafe {
            core_wallet_tx_builder_set_selection_strategy(drain, CoreSelectionStrategyFFI::All)
        };
        assert_eq!(set.code, PlatformWalletFFIResultCode::Success);

        let result = finalize_built(drain, &core, &signer);
        assert!(
            !matches!(
                result,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ) && result.is_err(),
            "got {result:?}"
        );
    }

    /// An asset lock spends what its credit outputs burn, set through the
    /// payload setter with no explicit output. With every coin not final
    /// (500,000 duffs), a burn beyond them is insufficient funds, not code 59;
    /// one they cover is code 59.
    #[test]
    fn should_size_an_asset_lock_shortfall_by_its_burn() {
        for (burn, waiting) in [(5_000_000, false), (100_000, true)] {
            let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
                StandardAccountType::BIP44Account,
                &[500_000],
                &[500_000],
            ));
            let payload =
                TransactionPayload::AssetLockPayloadType(AssetLockPayload::new(vec![TxOut {
                    value: burn,
                    script_pubkey: DashAddress::dummy(Network::Testnet, 95).script_pubkey(),
                }]));
            let bytes = bincode::encode_to_vec(payload, bincode::config::standard())
                .expect("encode an asset lock payload");
            let lock = unsafe { core_wallet_tx_builder_new(FFINetwork::Testnet) };
            let set = unsafe {
                core_wallet_tx_builder_set_special_payload(lock, bytes.as_ptr(), bytes.len())
            };
            assert_eq!(set.code, PlatformWalletFFIResultCode::Success);

            let result = finalize_built(lock, &core, &signer);
            if waiting {
                assert!(
                    matches!(
                        result,
                        Err(PlatformWalletError::CoreFundsAwaitingNetwork {
                            waiting: 500_000,
                            ..
                        })
                    ),
                    "burn {burn}: got {result:?}"
                );
            } else {
                assert!(is_insufficient(&result), "burn {burn}: got {result:?}");
            }
        }
    }

    /// An input recorded on the builder is finalized from the wallet's copy as
    /// it is then: one not final by that time is refused by name (code 59),
    /// with and without `use_only_added_inputs`.
    #[test]
    fn should_refuse_a_recorded_input_that_is_not_final_when_finalized() {
        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[900_000, 1_500_000],
            &[1_500_000],
        ));
        let coin = runtime()
            .block_on(standard_account_coins(
                &core,
                StandardAccountType::BIP44Account,
            ))
            .into_iter()
            .find(|utxo| utxo.value() == 1_500_000)
            .expect("the non-final coin")
            .outpoint;
        for only_added in [false, true] {
            let builder = payment(None, 100_000);
            unsafe { (*builder).state().inputs.push(coin) };
            if only_added {
                let set = unsafe { core_wallet_tx_builder_use_only_added_inputs(builder) };
                assert_eq!(set.code, PlatformWalletFFIResultCode::Success);
            }
            let result = finalize_built(builder, &core, &signer);
            assert!(
                matches!(
                    result,
                    Err(PlatformWalletError::CoreFundsAwaitingNetwork {
                        outpoint: Some(outpoint),
                        ..
                    }) if outpoint == coin
                ),
                "only_added={only_added}: got {result:?}"
            );
        }
    }

    /// An explicit zero rate is the rate the build pays, not "unset": 900,000
    /// final + 100,100 waiting fund a 1,000,000 payment for free once the
    /// waiting coin confirms — code 59. Left unset, the default rate makes the
    /// same build short.
    #[test]
    fn should_price_an_explicit_zero_fee_rate_as_zero() {
        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[900_000, 100_100],
            &[100_100],
        ));

        let zero = finalize_built(payment(Some(0), 1_000_000), &core, &signer);
        assert!(
            matches!(
                zero,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ),
            "got {zero:?}"
        );

        let unset = finalize_built(payment(None, 1_000_000), &core, &signer);
        assert!(is_insufficient(&unset), "got {unset:?}");
    }

    /// A special payload is bytes the fee pays for too. 900,000 final + 100,400
    /// waiting against a 1,000,000 payment is covered without one, but a
    /// ProUpRevTx payload (132 bytes) leaves too little: insufficient funds.
    #[test]
    fn should_price_a_waiting_shortfall_with_the_special_payload() {
        use dashcore::blockdata::transaction::special_transaction::provider_update_revocation::ProviderUpdateRevocationPayload;
        use dashcore::bls_sig_utils::BLSSignature;
        use dashcore::hash_types::InputsHash;
        use dashcore::hashes::Hash;

        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[900_000, 100_400],
            &[100_400],
        ));
        let payload = TransactionPayload::ProviderUpdateRevocationPayloadType(
            ProviderUpdateRevocationPayload::new(
                dashcore::Txid::all_zeros(),
                0,
                InputsHash::all_zeros(),
                BLSSignature::from([0u8; 96]),
            ),
        );
        let bytes = bincode::encode_to_vec(payload, bincode::config::standard())
            .expect("encode a revocation payload");

        let with_payload = payment(None, 1_000_000);
        let set = unsafe {
            core_wallet_tx_builder_set_special_payload(with_payload, bytes.as_ptr(), bytes.len())
        };
        assert_eq!(set.code, PlatformWalletFFIResultCode::Success);
        let with_payload = finalize_built(with_payload, &core, &signer);
        assert!(is_insufficient(&with_payload), "got {with_payload:?}");

        let without = finalize_built(payment(None, 1_000_000), &core, &signer);
        assert!(
            matches!(
                without,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ),
            "got {without:?}"
        );
    }

    /// A fee rate whose fee arithmetic overflows is refused where the host
    /// sets it — a typed error, not a panic or a wrapped fee at finalize —
    /// and the builder stays usable: a sane rate set after it builds.
    #[test]
    fn should_refuse_an_overflowing_fee_rate_when_it_is_set() {
        let (core, signer) =
            runtime().block_on(funded_spv_core_wallet(StandardAccountType::BIP44Account));
        let builder = payment(None, 100_000);
        let refused = unsafe { core_wallet_tx_builder_set_fee_rate(builder, u64::MAX) };
        assert_eq!(
            refused.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        let set = unsafe { core_wallet_tx_builder_set_fee_rate(builder, 1_000) };
        assert_eq!(set.code, PlatformWalletFFIResultCode::Success);
        let built = finalize_built(builder, &core, &signer).expect("a sane rate builds");
        runtime().block_on(core.abandon_transaction(&built));
    }

    /// With no coin final, the trial decides: a payment the waiting coins
    /// could never cover is insufficient funds, one they cover is code 59.
    #[test]
    fn should_call_a_payment_beyond_every_coin_insufficient_with_no_final_coin() {
        let (core, signer) = runtime().block_on(funded_spv_core_wallet_with_outputs(
            StandardAccountType::BIP44Account,
            &[500_000],
            &[500_000],
        ));

        let beyond = finalize_built(payment(None, 5_000_000), &core, &signer);
        assert!(
            !matches!(
                beyond,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ),
            "got {beyond:?}"
        );
        assert!(beyond.is_err());

        let within = finalize_built(payment(None, 100_000), &core, &signer);
        assert!(
            matches!(
                within,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ),
            "got {within:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{sole_deliverable_value, CoreAccountTypeFFI};
    use dashcore::blockdata::script::ScriptBuf;
    use dashcore::TxOut;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;

    /// What a DRAIN spans, per selector. `SelectionStrategy::All` takes every
    /// UTXO each named source offers, so this list IS the sweep scope — the
    /// claim the Kotlin `SelectionStrategy.ALL` doc makes to callers.
    ///
    /// The pooled selector is the DEFAULT for a send, so a caller who asks for
    /// a drain without naming an account type sweeps all three families at
    /// once, contact-receiving funds included. Pinned here so that widening
    /// cannot happen silently: anything added to `SEND_FUNDING_SOURCES`
    /// enlarges every defaulted drain, and this test is where that shows up.
    #[test]
    fn a_drains_scope_is_whatever_the_selector_names() {
        assert_eq!(
            CoreAccountTypeFFI::BIP44.funding_sources(),
            &[AccountTypePreference::BIP44],
            "a single-family selector drains exactly one account"
        );
        assert_eq!(
            CoreAccountTypeFFI::CoinJoin.funding_sources(),
            &[AccountTypePreference::CoinJoin],
            "CoinJoin stays its own privacy domain, never pooled"
        );
        assert_eq!(
            CoreAccountTypeFFI::AllSpendable.funding_sources(),
            &[
                AccountTypePreference::BIP44,
                AccountTypePreference::BIP32,
                AccountTypePreference::AllDashpayReceivingFunds,
            ],
            "the DEFAULT selector drains BIP44 + BIP32 + every DashPay \
             receiving account; BIP44 must stay first, as it supplies change"
        );
    }

    /// A spendable output. The script only has to NOT be an OP_RETURN.
    fn destination(value: u64) -> TxOut {
        TxOut {
            value,
            script_pubkey: ScriptBuf::from(vec![0x76, 0xa9, 0x14]),
        }
    }

    fn op_return(payload: &[u8]) -> TxOut {
        let data = dashcore::script::PushBytesBuf::try_from(payload.to_vec())
            .expect("test payload is within push limits");
        TxOut {
            value: 0,
            script_pubkey: ScriptBuf::new_op_return(&data),
        }
    }

    #[test]
    fn a_lone_destination_is_the_deliverable_amount() {
        assert_eq!(
            sole_deliverable_value(&[destination(27_442_985)]),
            27_442_985
        );
    }

    /// The MAYAChain shape: vault output plus a zero-value memo. The memo must
    /// not be mistaken for a second recipient, in EITHER order — Maya puts the
    /// memo at VOUT1, but nothing in the calculation may depend on that.
    #[test]
    fn a_data_carrier_beside_the_destination_is_ignored_in_both_orders() {
        let memo = op_return(b"=:MAYA.CACAO:maya1abc");
        assert_eq!(
            sole_deliverable_value(&[destination(27_442_985), memo.clone()]),
            27_442_985,
            "memo after the destination (the Maya layout)"
        );
        assert_eq!(
            sole_deliverable_value(&[memo, destination(27_442_985)]),
            27_442_985,
            "memo before the destination"
        );
    }

    /// Two recipients have no single deliverable amount. Reporting either one
    /// would let a host quote a number the payment does not pay.
    #[test]
    fn two_spendable_outputs_report_zero() {
        assert_eq!(
            sole_deliverable_value(&[destination(1_000), destination(2_000)]),
            0
        );
    }

    #[test]
    fn two_spendable_outputs_report_zero_even_beside_a_data_carrier() {
        assert_eq!(
            sole_deliverable_value(&[destination(1_000), op_return(b"x"), destination(2_000)]),
            0
        );
    }

    /// An OP_RETURN-only build pays no one; so does an empty output set.
    #[test]
    fn a_transaction_with_no_spendable_output_reports_zero() {
        assert_eq!(sole_deliverable_value(&[op_return(b"data only")]), 0);
        assert_eq!(sole_deliverable_value(&[]), 0);
    }

    /// An asset lock's single output IS an OP_RETURN, so it reports 0 rather
    /// than its burn value. That is the intended reading: the credits go to an
    /// identity, not to a payee a host would quote.
    #[test]
    fn an_op_return_carrying_value_still_reports_zero() {
        let mut burn = op_return(b"credits");
        burn.value = 500_000;
        assert_eq!(sole_deliverable_value(&[burn]), 0);
    }
}

#[cfg(test)]
mod real_finalizer_tests {
    //! The extern finalizers end to end: a wallet handle, a mnemonic
    //! resolver handle, the recorded builder — no shortcut through the
    //! Rust API — so their wiring (recipe replay, the waiting-coins trial,
    //! options) is what is tested.

    use super::*;
    use crate::core_wallet::broadcast::core_wallet_signed_transaction_free;
    use crate::core_wallet::signed_payment::registry_test_guard;
    use platform_wallet::test_support::{add_bip44_coin, test_platform_wallet_manager};
    use rs_sdk_ffi::{
        dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy,
        mnemonic_resolver_result,
    };

    /// The mnemonic `test_platform_wallet_manager` creates its wallet from.
    const PHRASE: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    unsafe extern "C" fn resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        let phrase = PHRASE.as_bytes();
        if phrase.len() + 1 > out_capacity {
            return mnemonic_resolver_result::BUFFER_TOO_SMALL;
        }
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    /// A registered wallet whose only coin is `value` duffs, final or not.
    fn wallet_with_coin(value: u64, final_: bool) -> (Handle, impl Drop) {
        let (manager, wallet_id) = runtime().block_on(test_platform_wallet_manager());
        let wallet = runtime()
            .block_on(manager.get_wallet(&wallet_id))
            .expect("wallet present");
        runtime().block_on(add_bip44_coin(&wallet, value, final_, COIN_TAG));
        let handle = PLATFORM_WALLET_STORAGE.insert(wallet);
        /// Unregisters the handle; keeps the manager (which owns the wallet's
        /// event task) alive until then.
        struct Release {
            handle: Handle,
            _manager: std::sync::Arc<dyn std::any::Any + Send + Sync>,
        }
        impl Drop for Release {
            fn drop(&mut self) {
                PLATFORM_WALLET_STORAGE.remove(self.handle);
            }
        }
        (
            handle,
            Release {
                handle,
                _manager: manager,
            },
        )
    }

    /// The txid byte of the coin `wallet_with_coin` adds (vout 0).
    const COIN_TAG: u8 = 0x5a;

    fn coin_outpoint() -> OutPointFFI {
        OutPointFFI {
            txid: [COIN_TAG; 32],
            vout: 0,
        }
    }

    fn finalize_real(
        builder: *mut FFITransactionBuilder,
        wallet: Handle,
    ) -> (PlatformWalletFFIResult, Handle) {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), resolve, noop_destroy)
        };
        let mut out: Handle = 0;
        let result = unsafe {
            core_wallet_tx_builder_finalize(
                builder,
                wallet,
                CoreAccountTypeFFI::AllSpendable,
                0,
                resolver,
                &mut out,
            )
        };
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
        (result, out)
    }

    /// Coin control through the real finalizer: the coin recorded with
    /// `add_inputs_from_outpoints` under `use_only_added_inputs` is the input.
    #[test]
    fn should_spend_the_recorded_input_through_the_real_finalizer() {
        let (wallet, _release) = wallet_with_coin(8_999_774, true);
        let builder = app_payment(1_000_000);
        let chosen = [coin_outpoint()];
        let added = unsafe {
            core_wallet_tx_builder_add_inputs_from_outpoints(
                builder,
                wallet,
                CoreAccountTypeFFI::BIP44,
                0,
                chosen.as_ptr(),
                chosen.len(),
            )
        };
        assert_eq!(added.code, PlatformWalletFFIResultCode::Success);
        let only = unsafe { core_wallet_tx_builder_use_only_added_inputs(builder) };
        assert_eq!(only.code, PlatformWalletFFIResultCode::Success);

        let (result, out) = finalize_real(builder, wallet);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        let spent = CORE_SIGNED_TRANSACTION_STORAGE
            .with_item(out, |tx| {
                tx.transaction
                    .transaction()
                    .input
                    .iter()
                    .map(|input| input.previous_output)
                    .collect::<Vec<_>>()
            })
            .expect("published handle");
        assert_eq!(
            spent,
            vec![OutPoint {
                txid: Txid::from_byte_array([COIN_TAG; 32]),
                vout: 0,
            }]
        );
        core_wallet_signed_transaction_free(out);
    }

    /// A recorded input spent before the build is refused by name, as
    /// `ErrorInvalidParameter` (`ChosenInputUnavailable`), nothing published.
    #[test]
    fn should_refuse_a_recorded_input_spent_since_through_the_real_finalizer() {
        let (manager, wallet_id) = runtime().block_on(test_platform_wallet_manager());
        let platform_wallet = runtime()
            .block_on(manager.get_wallet(&wallet_id))
            .expect("wallet present");
        runtime().block_on(add_bip44_coin(&platform_wallet, 8_999_774, true, COIN_TAG));
        let wallet = PLATFORM_WALLET_STORAGE.insert(platform_wallet.clone());
        let builder = app_payment(1_000_000);
        let chosen = [coin_outpoint()];
        let added = unsafe {
            core_wallet_tx_builder_add_inputs_from_outpoints(
                builder,
                wallet,
                CoreAccountTypeFFI::BIP44,
                0,
                chosen.as_ptr(),
                chosen.len(),
            )
        };
        assert_eq!(added.code, PlatformWalletFFIResultCode::Success);
        runtime().block_on(async {
            // Spent since it was chosen.
            let mut wm = platform_wallet.wallet_manager().write().await;
            let (_, info) = wm.get_wallet_and_info_mut(&wallet_id).expect("wallet");
            info.core_wallet
                .first_bip44_managed_account_mut()
                .expect("bip44 account")
                .utxos
                .clear();
        });

        let (result, out) = finalize_real(builder, wallet);
        PLATFORM_WALLET_STORAGE.remove(wallet);
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out, 0);
    }

    /// A locked coin that is not final either: confirmation would not make
    /// it usable, so naming it is the native finalizer's refusal
    /// (`ChosenInputUnavailable::NotSpendable`, `ErrorInvalidParameter`), not
    /// code 59.
    #[test]
    fn should_refuse_a_locked_coin_as_unspendable_before_finality() {
        let (manager, wallet_id) = runtime().block_on(test_platform_wallet_manager());
        let platform_wallet = runtime()
            .block_on(manager.get_wallet(&wallet_id))
            .expect("wallet present");
        runtime().block_on(add_bip44_coin(&platform_wallet, 8_999_774, false, COIN_TAG));
        runtime().block_on(async {
            let mut wm = platform_wallet.wallet_manager().write().await;
            let (_, info) = wm.get_wallet_and_info_mut(&wallet_id).expect("wallet");
            for utxo in info
                .core_wallet
                .first_bip44_managed_account_mut()
                .expect("bip44 account")
                .utxos
                .values_mut()
            {
                utxo.is_locked = true;
            }
        });
        let wallet = PLATFORM_WALLET_STORAGE.insert(platform_wallet.clone());
        let builder = app_payment(1_000_000);
        let chosen = [coin_outpoint()];
        let added = unsafe {
            core_wallet_tx_builder_add_inputs_from_outpoints(
                builder,
                wallet,
                CoreAccountTypeFFI::BIP44,
                0,
                chosen.as_ptr(),
                chosen.len(),
            )
        };
        PLATFORM_WALLET_STORAGE.remove(wallet);
        assert_eq!(
            added.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert!(unsafe { (*builder).state() }.inputs.is_empty());
        unsafe { core_wallet_tx_builder_destroy(builder) };
    }

    /// The app's Send: one output, default rate, the pooled sources.
    fn app_payment(amount: u64) -> *mut FFITransactionBuilder {
        let builder = unsafe { core_wallet_tx_builder_new(FFINetwork::Testnet) };
        let address =
            CString::new(DashAddress::dummy(Network::Testnet, 97).to_string()).expect("address");
        let added = unsafe { core_wallet_tx_builder_add_output(builder, address.as_ptr(), amount) };
        assert_eq!(added.code, PlatformWalletFFIResultCode::Success);
        builder
    }

    /// The app's shape through `core_wallet_tx_builder_finalize`: the only
    /// coin is the change of a send whose broadcast outcome is unknown (not
    /// final), and a second payment is code 59, with no handle published.
    #[test]
    fn should_return_code_59_from_the_real_finalizer_for_the_apps_second_payment() {
        let (wallet, _release) = wallet_with_coin(8_999_774, false);
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), resolve, noop_destroy)
        };
        let mut out: Handle = 7;
        let result = unsafe {
            core_wallet_tx_builder_finalize(
                app_payment(1_000_000),
                wallet,
                CoreAccountTypeFFI::AllSpendable,
                0,
                resolver,
                &mut out,
            )
        };
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorCoreFundsAwaitingNetwork
        );
        assert_eq!(out, 0, "nothing is published");
    }

    /// The same through the deferred finalizer, which shares the plumbing.
    #[test]
    fn should_return_code_59_from_the_deferred_finalizer_too() {
        let _registry = registry_test_guard();
        let (wallet, _release) = wallet_with_coin(8_999_774, false);
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), resolve, noop_destroy)
        };
        let (mut token, mut fee) = (0u64, 0u64);
        let mut txid: *mut c_char = std::ptr::null_mut();
        let mut tx = FFICoreTransaction {
            tx_bytes: std::ptr::null_mut(),
            tx_len: 0,
            fee: 0,
        };
        let mut bytes: *const u8 = std::ptr::null();
        let mut len: usize = 0;
        let result = unsafe {
            core_wallet_signed_payment_finalize(
                app_payment(1_000_000),
                wallet,
                CoreAccountTypeFFI::AllSpendable,
                0,
                resolver,
                &mut token,
                &mut fee,
                &mut txid,
                &mut tx,
                &mut bytes,
                &mut len,
            )
        };
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorCoreFundsAwaitingNetwork
        );
        assert_eq!(token, 0);
    }

    /// A final coin funds the same payment: signed through the resolver, a
    /// handle published.
    #[test]
    fn should_finalize_and_sign_through_the_real_finalizer() {
        let (wallet, _release) = wallet_with_coin(8_999_774, true);
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), resolve, noop_destroy)
        };
        let mut out: Handle = 0;
        let result = unsafe {
            core_wallet_tx_builder_finalize(
                app_payment(1_000_000),
                wallet,
                CoreAccountTypeFFI::AllSpendable,
                0,
                resolver,
                &mut out,
            )
        };
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
        let message = (!result.message.is_null())
            .then(|| unsafe { std::ffi::CStr::from_ptr(result.message) }.to_string_lossy());
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::Success,
            "{message:?}"
        );
        assert_ne!(out, 0);
        core_wallet_signed_transaction_free(out);
    }
}

use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::{Handle, CORE_SIGNED_TRANSACTION_STORAGE, PLATFORM_WALLET_STORAGE};
use crate::runtime::runtime;
use crate::types::{FFINetwork, Network};
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::hashes::Hash;
use dashcore::{Address as DashAddress, OutPoint, Txid};
use key_wallet::account::ManagedAccountCollection;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    TransactionBuilder, MAX_STANDARD_OP_RETURN_BYTES,
};
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::str::FromStr;

/// Opaque, C-compatible transaction builder. `inner` is a heap-boxed
/// key-wallet `TransactionBuilder`; `network` is the wallet network output
/// and change addresses are validated against.
///
/// NOT thread-safe: a single builder must be used from one thread at a time.
/// The setters mutate `*inner` in place (`take_builder`/`store_builder`)
/// without synchronization.
#[repr(C)]
pub struct FFITransactionBuilder {
    inner: *mut c_void,
    network: FFINetwork,
    /// Set by `core_wallet_tx_builder_use_only_added_inputs`. key-wallet takes
    /// this per funding call, which the finalizers make internally, so the
    /// intent has to be carried here and read when they run.
    reservation_only: bool,
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
    /// [`CoreWallet::finalize_transaction`]'s multi-source API, whose first
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

    let ffi = Box::from_raw(builder);
    let inner = *Box::from_raw(ffi.inner as *mut TransactionBuilder);
    let wallet = unwrap_option_or_return!(PLATFORM_WALLET_STORAGE.with_item(wallet, |w| w.clone()));

    let builder_network: Network = ffi.network.into();
    if builder_network != wallet.network() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "builder network does not match wallet network".to_string(),
        );
    }

    let signer =
        MnemonicResolverCoreSigner::new(core_signer_handle, wallet.wallet_id(), wallet.network());
    let reservation_only = ffi.reservation_only;
    let finalized = runtime().block_on(wallet.core().finalize_transaction_with_options(
        inner,
        account_type.funding_sources(),
        account_index,
        &signer,
        reservation_only,
    ));
    let finalized = unwrap_result_or_return!(finalized);

    // Publishing the finalized handle is gated exactly like the deferred-token sibling
    // below (`core_wallet_signed_payment_finalize`). `finalize_transaction` drops
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

/// Atomically fund, reserve, and sign a configured builder for DEFERRED
/// (BIP70/BIP270) submission, then register the built transaction — holding its
/// UTXO reservation — in one native operation.
///
/// This is the deferred counterpart to `core_wallet_tx_builder_finalize`: it
/// runs the same atomic `finalize_transaction`, where selection and insertion
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
/// # Safety
/// `builder` must be a valid, non-destroyed pointer; `wallet` a valid
/// platform-wallet handle; `core_signer_handle` a valid resolver handle; every
/// out-pointer must be writable. `out_tx` must point at writable storage for one
/// `FFICoreTransaction` (typically zeroed).
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
    check_ptr!(builder);
    check_ptr!(core_signer_handle);
    check_ptr!(out_token);
    check_ptr!(out_fee);
    check_ptr!(out_txid);
    check_ptr!(out_tx);
    check_ptr!(out_bytes_ptr);
    check_ptr!(out_bytes_len);
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

    // `finalize_transaction` consumes the builder: reclaim both heap boxes up
    // front so they are freed on every return path below.
    let ffi = Box::from_raw(builder);
    let inner = *Box::from_raw(ffi.inner as *mut TransactionBuilder);

    let wallet = unwrap_option_or_return!(PLATFORM_WALLET_STORAGE.with_item(wallet, |w| w.clone()));

    let builder_network: Network = ffi.network.into();
    if builder_network != wallet.network() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "builder network does not match wallet network".to_string(),
        );
    }

    let signer =
        MnemonicResolverCoreSigner::new(core_signer_handle, wallet.wallet_id(), wallet.network());

    // Atomic select + reserve + sign in one wallet-manager critical section.
    let finalized = runtime().block_on(wallet.core().finalize_transaction(
        inner,
        account_type.funding_sources(),
        account_index,
        &signer,
    ));
    let finalized = unwrap_result_or_return!(finalized);

    // `finalize_transaction` drops the wallet-manager write lock before awaiting
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

impl FFITransactionBuilder {
    /// The inner builder taken out by value, leaving an empty one in its
    /// place. Pair with [`FFITransactionBuilder::store_builder`] to apply a
    /// fluent (`self -> Self`) method.
    ///
    /// # Safety
    /// `self.inner` must point at a live `TransactionBuilder`
    unsafe fn take_builder(&self) -> TransactionBuilder {
        std::mem::take(&mut *(self.inner as *mut TransactionBuilder))
    }

    /// Store `builder` back into the inner slot.
    ///
    /// # Safety
    /// `self.inner` must point at a live `TransactionBuilder`
    unsafe fn store_builder(&self, builder: TransactionBuilder) {
        *(self.inner as *mut TransactionBuilder) = builder;
    }
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
    let inner = Box::into_raw(Box::new(TransactionBuilder::new())) as *mut c_void;
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

    let b = (*builder).take_builder();
    let b = b.add_output(&address, amount);
    (*builder).store_builder(b);

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

    // `add_op_return` takes the builder by value, so a rejected payload drops it and leaves
    // `take_builder`'s `mem::take` default behind — silently discarding outputs and options
    // the caller already configured. Reject an over-long payload *before* taking the builder
    // so the slot keeps its real state. `add_op_return` re-checks; this is the same policy
    // constant, not a second opinion.
    if data_len > MAX_STANDARD_OP_RETURN_BYTES {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "OP_RETURN payload too large: {data_len} bytes (max {MAX_STANDARD_OP_RETURN_BYTES})"
            ),
        );
    }

    let b = (*builder).take_builder();
    let b = match b.add_op_return(bytes) {
        Ok(b) => b,
        Err(err) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                err.to_string(),
            );
        }
    };
    (*builder).store_builder(b);

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

    let b = (*builder).take_builder();
    let b = b.set_change_address(address);
    (*builder).store_builder(b);

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

    let b = (*builder).take_builder();
    let b = b.preserve_output_order();
    (*builder).store_builder(b);

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

    let b = (*builder).take_builder();
    let b = b.change_to_first_input();
    (*builder).store_builder(b);

    PlatformWalletFFIResult::ok()
}

/// # Safety
/// `builder` must be a valid, non-destroyed pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_set_fee_rate(
    builder: *mut FFITransactionBuilder,
    sat_per_kb: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);

    let b = (*builder).take_builder();
    let b = b.set_fee_rate(FeeRate::new(sat_per_kb));
    (*builder).store_builder(b);

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

/// # Safety
/// `builder` must be a valid, non-destroyed pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_set_selection_strategy(
    builder: *mut FFITransactionBuilder,
    strategy: CoreSelectionStrategyFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);

    let b = (*builder).take_builder();
    let b = b.set_selection_strategy(strategy.into());
    (*builder).store_builder(b);

    PlatformWalletFFIResult::ok()
}

/// Set the block height coin selection treats as the chain tip (used for
/// coinbase maturity and locktime).
///
/// This value is advisory: the wallet-aware finalizers override it with the
/// wallet's last processed height when they run, so the wallet height always
/// wins for the funded/signed build. Use this only when building without a
/// wallet.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_set_current_height(
    builder: *mut FFITransactionBuilder,
    height: u32,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);

    let b = (*builder).take_builder();
    let b = b.set_current_height(height);
    (*builder).store_builder(b);

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
    let payload: TransactionPayload =
        match bincode::decode_from_slice(bytes, bincode::config::standard()) {
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

    let b = (*builder).take_builder();
    let b = b.set_special_payload(payload);
    (*builder).store_builder(b);

    PlatformWalletFFIResult::ok()
}

/// Add a caller-chosen subset of the account's UTXOs as inputs. `outpoints`
/// are selected from the account's own UTXO set (the same ones
/// `platform_wallet_account_utxos` returns). An outpoint not owned by the
/// account is an error
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
            .ok_or_else(|| "wallet not found".to_string())?;

        let managed = managed_account(&info.core_wallet.accounts, source, account_index)
            .ok_or_else(|| format!("managed account {source:?} #{account_index} not found"))?;

        let mut selected = Vec::with_capacity(requested.len());
        for op in &requested {
            let utxo = managed
                .utxos
                .get(op)
                .cloned()
                .ok_or_else(|| format!("outpoint {}:{} not in account", op.txid, op.vout))?;
            selected.push(utxo);
        }

        // Validation succeeded — only now consume the builder.
        let taken = (*builder).take_builder();
        (*builder).store_builder(taken.add_inputs(selected));
        Ok::<_, String>(())
    });

    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("add_inputs_from_outpoints failed: {e}"),
        ),
    }
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
    let _ = Box::from_raw(b.inner as *mut TransactionBuilder);
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

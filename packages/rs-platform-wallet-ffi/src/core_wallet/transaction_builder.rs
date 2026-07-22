use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::{Handle, CORE_SIGNED_TRANSACTION_V2_STORAGE, PLATFORM_WALLET_STORAGE};
use crate::runtime::runtime;
use crate::types::{FFINetwork, Network};
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::hashes::Hash;
use dashcore::{Address as DashAddress, OutPoint, Transaction, Txid};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::account::ManagedAccountCollection;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
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
}

/// Broadcast it with `core_wallet_broadcast_transaction`, then release it
/// with `core_wallet_transaction_free`.
#[repr(C)]
pub struct FFICoreTransaction {
    tx_bytes: *mut u8,
    tx_len: usize,
    // Part of the C ABI (the Swift host reads `FFICoreTransaction.fee`); the
    // Rust side only writes it, so silence the never-read lint.
    #[allow(dead_code)]
    fee: u64,
}

/// Internal value behind the opaque V2 numeric handle. Keeping the originating
/// CoreWallet with the signed transaction lets `free` perform the same safe
/// reservation release as explicit abandon, even after the host discarded its
/// transient CoreWallet handle.
pub struct FFICoreSignedTransactionV2 {
    pub(crate) wallet: platform_wallet::CoreWallet<platform_wallet::broadcaster::SpvBroadcaster>,
    pub(crate) transaction: platform_wallet::SignedCoreTransaction,
}

impl FFICoreTransaction {
    pub(crate) fn bytes(&self) -> &[u8] {
        if self.tx_bytes.is_null() || self.tx_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.tx_bytes, self.tx_len) }
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub enum CoreAccountTypeFFI {
    BIP44,
    BIP32,
    CoinJoin,
}

impl From<CoreAccountTypeFFI> for AccountTypePreference {
    fn from(value: CoreAccountTypeFFI) -> Self {
        match value {
            CoreAccountTypeFFI::BIP44 => AccountTypePreference::BIP44,
            CoreAccountTypeFFI::BIP32 => AccountTypePreference::BIP32,
            CoreAccountTypeFFI::CoinJoin => AccountTypePreference::CoinJoin,
        }
    }
}

/// Atomically fund, reserve and sign a configured builder.
///
/// Unlike the deprecated `set_funding` + `build_signed` sequence, selection
/// and insertion into the account ReservationSet cannot interleave with a
/// competing finalizer. The wallet-manager lock is dropped before the host
/// mnemonic resolver is invoked. This function consumes `builder` on every
/// path after its pointer is accepted.
///
/// On success `out_transaction_handle` receives an opaque V2 handle. Consume
/// it with `core_wallet_broadcast_signed_transaction_v2` or
/// `core_wallet_abandon_signed_transaction_v2`.
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
    let finalized = runtime().block_on(wallet.core().finalize_transaction(
        inner,
        account_type.into(),
        account_index,
        &signer,
    ));
    let finalized = unwrap_result_or_return!(finalized);
    *out_transaction_handle =
        CORE_SIGNED_TRANSACTION_V2_STORAGE.insert(FFICoreSignedTransactionV2 {
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
/// deferred build through it closes the double-selection window that the
/// deprecated `set_funding` + `build_signed` + `register` sequence reopened once
/// the Kotlin per-wallet send mutex was removed: two concurrent deferred builds,
/// or a deferred build racing an immediate send, can no longer select the same
/// UTXO. Consumes `builder` on every path after its pointer is accepted.
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
    *out_token = 0;

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
        account_type.into(),
        account_index,
        &signer,
    ));
    let finalized = unwrap_result_or_return!(finalized);

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
    // committed the reservation; register just takes ownership of the built tx so
    // a later broadcast/release can reconcile it, capturing the wallet instance
    // whose `ReservationSet` holds the inputs.
    let token = runtime().block_on(
        crate::core_wallet::signed_payment::SIGNED_PAYMENT_REGISTRY.register(
            wallet.core().clone(),
            finalized.transaction().clone(),
            // Retain the FULL account handle (CoinJoin included), not just the
            // `StandardAccountType` subset: `finalize` reserved the selected
            // inputs regardless of variant, so a CoinJoin-funded deferred payment
            // must be able to release them immediately on rejection/abandon
            // rather than stranding them until the 24-block TTL.
            account_type.into(),
            account_index,
            // Baseline the age guard on the reservation's OWN stamp height,
            // captured inside finalize's funding critical section before the
            // external signer ran — never a fresh post-signing sample.
            Some(finalized.reservation_height()),
        ),
    );

    *out_tx = FFICoreTransaction {
        tx_bytes: Box::into_raw(serialized.into_boxed_slice()) as *mut u8,
        tx_len: len,
        fee,
    };
    *out_token = token;
    *out_fee = fee;
    *out_txid = c_txid.into_raw();
    // Borrowed view into the just-written `out_tx` buffer; the caller copies the
    // bytes out before freeing `out_tx` with `core_wallet_transaction_free`.
    *out_bytes_ptr = (*out_tx).tx_bytes as *const u8;
    *out_bytes_len = len;
    PlatformWalletFFIResult::ok()
}

impl CoreAccountTypeFFI {
    /// The `StandardAccountType` this maps to, or `None` for `CoinJoin`.
    ///
    /// A CoinJoin-funded build DOES end up with reserved UTXOs — `build_signed`
    /// (via `assemble_unsigned`) reserves the selected inputs regardless of
    /// account type, since `set_funding` attaches the shared `ReservationSet`
    /// for every variant. But `reservations.rs`'s release-on-rejection is
    /// defined only over `StandardAccountType` (BIP44/BIP32). Returning `None`
    /// here routes CoinJoin through the plain broadcast, so a rejected CoinJoin
    /// tx keeps its reservation until the TTL backstop. That is intentional: the
    /// only CoinJoin funding path is a sweep — a single sender spending each
    /// UTXO exactly once, with no concurrent build or retry to race — so there
    /// is nothing to reconcile in practice.
    pub(crate) fn as_standard_account_type(&self) -> Option<StandardAccountType> {
        match self {
            CoreAccountTypeFFI::BIP44 => Some(StandardAccountType::BIP44Account),
            CoreAccountTypeFFI::BIP32 => Some(StandardAccountType::BIP32Account),
            CoreAccountTypeFFI::CoinJoin => None,
        }
    }
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
    match source {
        AccountTypePreference::BIP44 => accounts.standard_bip44_accounts.get(&account_index),
        AccountTypePreference::BIP32 => accounts.standard_bip32_accounts.get(&account_index),
        AccountTypePreference::CoinJoin => accounts.coinjoin_accounts.get(&account_index),
    }
}

fn managed_account_mut(
    accounts: &mut ManagedAccountCollection,
    source: AccountTypePreference,
    account_index: u32,
) -> Option<&mut ManagedCoreFundsAccount> {
    match source {
        AccountTypePreference::BIP44 => accounts.standard_bip44_accounts.get_mut(&account_index),
        AccountTypePreference::BIP32 => accounts.standard_bip32_accounts.get_mut(&account_index),
        AccountTypePreference::CoinJoin => accounts.coinjoin_accounts.get_mut(&account_index),
    }
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
/// `core_wallet_tx_builder_destroy` (or `core_wallet_tx_builder_build_signed`,
/// which consumes it).
///
/// # Safety
/// The returned pointer is owned by the caller.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_new(
    network: FFINetwork,
) -> *mut FFITransactionBuilder {
    let inner = Box::into_raw(Box::new(TransactionBuilder::new())) as *mut c_void;
    Box::into_raw(Box::new(FFITransactionBuilder { inner, network }))
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
/// This value is advisory: `core_wallet_tx_builder_set_funding` and
/// `core_wallet_tx_builder_build_signed` both override it with the wallet's
/// last processed height when they run, so the wallet height always wins for
/// the funded/signed build. Use this only when building without a wallet.
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

/// Fund the builder from the wallet account, setting inputs and change.
///
/// # Concurrency limitation (known, intentionally not fixed here)
/// key-wallet's `set_funding` filters out UTXOs already recorded in the
/// account's shared `ReservationSet`, but the reservation for *this* build is
/// only taken at build time (`assemble_unsigned` inside `build_signed`).
/// Because the FFI splits `set_funding` and `build_signed` across the C ABI —
/// the wallet lock cannot be held across the boundary — two concurrent builds
/// on the SAME account can both pass `set_funding` before either reserves and
/// select the same UTXO, producing a double-spend at broadcast. Single-threaded
/// callers (the SDK's send flow) are unaffected; concurrent same-account sends
/// must serialize at the call site.
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer; `wallet` a valid
/// platform-wallet handle.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_tx_builder_set_funding(
    builder: *mut FFITransactionBuilder,
    wallet: Handle,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);

    let wallet = unwrap_option_or_return!(PLATFORM_WALLET_STORAGE.with_item(wallet, |w| w.clone()));

    // Reject a builder created for a different network than the wallet.
    let builder_network: Network = (*builder).network.into();
    if builder_network != wallet.network() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "builder network does not match wallet network".to_string(),
        );
    }

    let wallet_id = wallet.wallet_id();
    let source: AccountTypePreference = account_type.into();

    let result = runtime().block_on(async {
        let mut wm = wallet.wallet_manager().write().await;
        let (w, info) = wm
            .get_wallet_and_info_mut(&wallet_id)
            .ok_or_else(|| "wallet not found".to_string())?;

        let account = match source {
            AccountTypePreference::BIP44 => w.get_bip44_account(account_index),
            AccountTypePreference::BIP32 => w.get_bip32_account(account_index),
            AccountTypePreference::CoinJoin => w.get_coinjoin_account(account_index),
        }
        .ok_or_else(|| format!("wallet account {source:?} #{account_index} not found"))?;

        let height = info.core_wallet.last_processed_height();

        let managed = managed_account_mut(&mut info.core_wallet.accounts, source, account_index)
            .ok_or_else(|| format!("managed account {source:?} #{account_index} not found"))?;

        // Resolution succeeded — only now consume the builder so a lookup
        // failure above can never leave it emptied.
        let taken = (*builder).take_builder();
        let funded = taken
            .set_current_height(height)
            .set_funding(managed, account);
        (*builder).store_builder(funded);
        Ok::<_, String>(())
    });

    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("set_funding failed: {e}"),
        ),
    }
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
    // `set_funding` / `build_signed` so all three wallet-aware entry points fail
    // fast instead of mutating a foreign-network builder.
    let builder_network: Network = (*builder).network.into();
    if builder_network != wallet.network() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "builder network does not match wallet network".to_string(),
        );
    }

    let wallet_id = wallet.wallet_id();
    let source: AccountTypePreference = account_type.into();

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

/// Build and sign, resolving signing paths from the wallet account. Returns
/// consensus-serialized signed bytes and the fee.
///
/// This function also frees the builder
///
/// # Safety
/// `builder` must be a valid, non-destroyed pointer; `wallet` a valid platform-wallet handle;
/// `core_signer_handle` a valid, non-destroyed resolver handle; `out_tx` a
/// writable pointer the caller later frees with `core_wallet_transaction_free`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_tx_builder_build_signed(
    builder: *mut FFITransactionBuilder,
    wallet: Handle,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_tx: *mut FFICoreTransaction,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    // `build` consumes the builder: reclaim both heap boxes up front so they
    // are freed on every return path below
    let ffi = Box::from_raw(builder);
    let inner = *Box::from_raw(ffi.inner as *mut TransactionBuilder);

    check_ptr!(core_signer_handle);
    check_ptr!(out_tx);

    let wallet = unwrap_option_or_return!(PLATFORM_WALLET_STORAGE.with_item(wallet, |w| w.clone()));

    // Backstop network check: reject a builder built for a different network
    // than the wallet, even when `set_funding` already validated it.
    let builder_network: Network = ffi.network.into();
    if builder_network != wallet.network() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "builder network does not match wallet network".to_string(),
        );
    }

    let wallet_id = wallet.wallet_id();
    let source: AccountTypePreference = account_type.into();
    let signer = MnemonicResolverCoreSigner::new(core_signer_handle, wallet_id, wallet.network());

    let build = runtime().block_on(async {
        let wm = wallet.wallet_manager().read().await;
        let info = wm
            .get_wallet_info(&wallet_id)
            .ok_or_else(|| "wallet not found".to_string())?;

        let height = info.core_wallet.last_processed_height();

        let managed = managed_account(&info.core_wallet.accounts, source, account_index)
            .ok_or_else(|| format!("managed account {source:?} #{account_index} not found"))?;

        inner
            .set_current_height(height)
            .build_signed(&signer, |addr| managed.address_derivation_path(&addr))
            .await
            .map_err(|e| e.to_string())
    });

    let (tx, fee): (Transaction, u64) = match build {
        Ok(v) => v,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("transaction build failed: {e}"),
            );
        }
    };

    let serialized = dashcore::consensus::serialize(&tx);
    let len = serialized.len();

    *out_tx = FFICoreTransaction {
        tx_bytes: Box::into_raw(serialized.into_boxed_slice()) as *mut u8,
        tx_len: len,
        fee,
    };

    PlatformWalletFFIResult::ok()
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

/// Free a transaction returned by `core_wallet_tx_builder_build_signed`.
/// Idempotent: the fields are nulled, so a second call is a no-op.
///
/// # Safety
/// `tx` must be a valid pointer to an `FFICoreTransaction` from
/// `core_wallet_tx_builder_build_signed` (or null).
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

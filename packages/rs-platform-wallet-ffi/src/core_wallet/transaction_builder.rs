use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::{Handle, PLATFORM_WALLET_STORAGE};
use crate::runtime::runtime;
use crate::types::{FFINetwork, Network};
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use dashcore::blockdata::script::Instruction;
use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::hashes::Hash;
use dashcore::{Address as DashAddress, OutPoint, PublicKey, ScriptBuf, Transaction, Txid};
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

#[repr(C)]
pub struct FFICoreTransactionInput {
    prev_txid: [u8; 32],
    prev_vout: u32,
    address: *mut c_char,
}

#[repr(C)]
pub struct FFICoreTransactionOutput {
    address: *mut c_char,
    value_duffs: u64,
    script_pubkey: *mut u8,
    script_pubkey_len: usize,
}

#[repr(C)]
pub struct FFICoreTransaction {
    tx_bytes: *mut u8,
    tx_len: usize,
    fee: u64,
    txid: [u8; 32],
    inputs: *mut FFICoreTransactionInput,
    inputs_count: usize,
    outputs: *mut FFICoreTransactionOutput,
    outputs_count: usize,
}

fn vec_to_ptr<T>(v: Vec<T>) -> *mut T {
    if v.is_empty() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(v.into_boxed_slice()) as *mut T
    }
}

fn addr_to_cstr(address: Option<DashAddress>) -> *mut c_char {
    address
        .and_then(|a| CString::new(a.to_string()).ok())
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

fn input_address_from_script_sig(script_sig: &ScriptBuf, network: Network) -> Option<DashAddress> {
    let mut sig: Option<&[u8]> = None;
    let mut pubkey_bytes: Option<&[u8]> = None;
    for instruction in script_sig.instructions() {
        match instruction {
            Ok(Instruction::PushBytes(bytes)) => match (sig, pubkey_bytes) {
                (None, None) => sig = Some(bytes.as_bytes()),
                (Some(_), None) => pubkey_bytes = Some(bytes.as_bytes()),
                _ => return None, // more than two pushes
            },
            _ => return None, // non-push opcode or unparseable script
        }
    }
    let (sig, pubkey_bytes) = (sig?, pubkey_bytes?);
    if sig.is_empty() || sig[0] != 0x30 || sig.len() > 73 {
        return None;
    }
    if pubkey_bytes.len() != 33 && pubkey_bytes.len() != 65 {
        return None;
    }
    let pubkey = PublicKey::from_slice(pubkey_bytes).ok()?;
    Some(DashAddress::p2pkh(&pubkey, network))
}

impl From<(&Transaction, u64, Network)> for FFICoreTransaction {
    fn from((tx, fee, network): (&Transaction, u64, Network)) -> Self {
        let serialized = dashcore::consensus::serialize(tx);
        let tx_len = serialized.len();
        let tx_bytes = if tx_len == 0 {
            std::ptr::null_mut()
        } else {
            Box::into_raw(serialized.into_boxed_slice()) as *mut u8
        };

        let inputs: Vec<FFICoreTransactionInput> = tx
            .input
            .iter()
            .map(|txin| {
                let address = if txin.previous_output.is_null() {
                    None // coinbase
                } else {
                    input_address_from_script_sig(&txin.script_sig, network)
                };
                FFICoreTransactionInput {
                    prev_txid: txin.previous_output.txid.to_byte_array(),
                    prev_vout: txin.previous_output.vout,
                    address: addr_to_cstr(address),
                }
            })
            .collect();

        let outputs: Vec<FFICoreTransactionOutput> = tx
            .output
            .iter()
            .map(|txout| {
                let address = DashAddress::from_script(&txout.script_pubkey, network).ok();
                let script = txout.script_pubkey.to_bytes();
                let script_len = script.len();
                FFICoreTransactionOutput {
                    address: addr_to_cstr(address),
                    value_duffs: txout.value,
                    script_pubkey: vec_to_ptr(script),
                    script_pubkey_len: script_len,
                }
            })
            .collect();

        let inputs_count = inputs.len();
        let outputs_count = outputs.len();

        FFICoreTransaction {
            tx_bytes,
            tx_len,
            fee,
            txid: tx.txid().to_byte_array(),
            inputs: vec_to_ptr(inputs),
            inputs_count,
            outputs: vec_to_ptr(outputs),
            outputs_count,
        }
    }
}

impl Drop for FFICoreTransaction {
    fn drop(&mut self) {
        unsafe {
            if !self.tx_bytes.is_null() && self.tx_len > 0 {
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    self.tx_bytes,
                    self.tx_len,
                )));
            }

            if !self.inputs.is_null() {
                let inputs = Vec::from_raw_parts(self.inputs, self.inputs_count, self.inputs_count);
                for input in &inputs {
                    if !input.address.is_null() {
                        drop(CString::from_raw(input.address));
                    }
                }
                drop(inputs);
            }

            if !self.outputs.is_null() {
                let outputs =
                    Vec::from_raw_parts(self.outputs, self.outputs_count, self.outputs_count);
                for output in &outputs {
                    if !output.address.is_null() {
                        drop(CString::from_raw(output.address));
                    }
                    if !output.script_pubkey.is_null() {
                        drop(Vec::from_raw_parts(
                            output.script_pubkey,
                            output.script_pubkey_len,
                            output.script_pubkey_len,
                        ));
                    }
                }
                drop(outputs);
            }
        }
    }
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
/// writable pointer that, on success, receives an owned `*mut FFICoreTransaction`
/// the caller later frees with `core_wallet_transaction_free`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_tx_builder_build_signed(
    builder: *mut FFITransactionBuilder,
    wallet: Handle,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_tx: *mut *mut FFICoreTransaction,
) -> PlatformWalletFFIResult {
    check_ptr!(builder);
    // `build` consumes the builder: reclaim both heap boxes up front so they
    // are freed on every return path below
    let ffi = Box::from_raw(builder);
    let inner = *Box::from_raw(ffi.inner as *mut TransactionBuilder);

    check_ptr!(core_signer_handle);
    check_ptr!(out_tx);
    *out_tx = std::ptr::null_mut();

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

    // Marshal the whole transaction (bytes, fee, txid, decoded inputs/outputs)
    // into a heap-owned struct. Uses the wallet's network (already validated
    // above) to render addresses.
    *out_tx = Box::into_raw(Box::new(FFICoreTransaction::from((
        &tx,
        fee,
        wallet.network(),
    ))));

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
/// Reclaims the box; its [`Drop`] impl frees the bytes and input/output arrays.
/// Safe to call with null.
///
/// # Safety
/// `tx` must be null or a pointer returned by
/// `core_wallet_tx_builder_build_signed` that has not been freed yet.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_transaction_free(tx: *mut FFICoreTransaction) {
    if tx.is_null() {
        return;
    }
    drop(Box::from_raw(tx)); // Drop frees bytes + inputs + outputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::secp256k1::{Secp256k1, SecretKey};
    use dashcore::{OutPoint, TxIn, TxOut, Txid, Witness};
    use std::ffi::CStr;

    struct Marshalled {
        txid: [u8; 32],
        inputs: Vec<([u8; 32], u32, Option<String>)>,
        outputs: Vec<(Option<String>, u64, Vec<u8>)>,
    }

    unsafe fn cstr_opt(ptr: *mut c_char) -> Option<String> {
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr).to_str().unwrap().to_owned())
        }
    }

    fn marshal_ok(tx: &Transaction, network: Network) -> Marshalled {
        let ffi = FFICoreTransaction::from((tx, 0, network));
        unsafe {
            let inputs = if ffi.inputs.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(ffi.inputs, ffi.inputs_count)
                    .iter()
                    .map(|i| (i.prev_txid, i.prev_vout, cstr_opt(i.address)))
                    .collect()
            };
            let outputs = if ffi.outputs.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(ffi.outputs, ffi.outputs_count)
                    .iter()
                    .map(|o| {
                        let script = if o.script_pubkey.is_null() {
                            Vec::new()
                        } else {
                            std::slice::from_raw_parts(o.script_pubkey, o.script_pubkey_len)
                                .to_vec()
                        };
                        (cstr_opt(o.address), o.value_duffs, script)
                    })
                    .collect()
            };
            Marshalled {
                txid: ffi.txid,
                inputs,
                outputs,
            }
        }
        // `ffi` drops here, freeing bytes + inputs + outputs.
    }

    fn test_pubkey() -> PublicKey {
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[0x42u8; 32]).expect("valid secret key");
        PublicKey::new(sk.public_key(&secp))
    }

    fn p2pkh_spend_tx(network: Network) -> (Transaction, DashAddress) {
        let pubkey = test_pubkey();
        let addr = DashAddress::p2pkh(&pubkey, network);
        let script_sig = dashcore::blockdata::script::Builder::new()
            .push_slice([0x30u8; 71]) // DER-shaped: 0x30 tag, ≤ 73 bytes
            .push_key(&pubkey)
            .into_script();
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([0x11u8; 32]),
                    vout: 3,
                },
                script_sig,
                sequence: 0xffffffff,
                witness: Witness::default(),
            }],
            output: vec![
                TxOut {
                    value: 151_072,
                    script_pubkey: addr.script_pubkey(),
                },
                TxOut {
                    value: 0,
                    script_pubkey: ScriptBuf::new_op_return(&[0xAAu8; 4]),
                },
            ],
            special_transaction_payload: None,
        };
        (tx, addr)
    }

    fn tx_with_script_sig(script_sig: ScriptBuf) -> Transaction {
        let addr = DashAddress::dummy(Network::Testnet, 1);
        let mut tx = Transaction::dummy(&addr, 1..2, &[9_000]);
        tx.input[0].script_sig = script_sig;
        tx
    }

    #[test]
    fn marshals_outputs_with_addresses_and_values() {
        let addr = DashAddress::dummy(Network::Testnet, 7);
        let tx = Transaction::dummy(&addr, 1..2, &[151_072, 20_002]);
        let marshalled = marshal_ok(&tx, Network::Testnet);

        assert_eq!(marshalled.txid, tx.txid().to_byte_array());
        assert_eq!(marshalled.outputs.len(), 2);
        assert_eq!(marshalled.outputs[0].0, Some(addr.to_string()));
        assert_eq!(marshalled.outputs[0].1, 151_072);
        assert_eq!(marshalled.outputs[0].2, addr.script_pubkey().into_bytes());
        assert_eq!(marshalled.outputs[1].0, Some(addr.to_string()));
        assert_eq!(marshalled.outputs[1].1, 20_002);
        assert!(
            addr.to_string().starts_with('y'),
            "testnet P2PKH starts with 'y'"
        );
    }

    #[test]
    fn op_return_output_has_no_address() {
        let (tx, _) = p2pkh_spend_tx(Network::Testnet);
        let marshalled = marshal_ok(&tx, Network::Testnet);
        assert!(marshalled.outputs[1].0.is_none());
        assert!(
            !marshalled.outputs[1].2.is_empty(),
            "script bytes still present"
        );
    }

    #[test]
    fn recovers_p2pkh_input_address_from_script_sig() {
        let (tx, addr) = p2pkh_spend_tx(Network::Testnet);
        let marshalled = marshal_ok(&tx, Network::Testnet);

        assert_eq!(marshalled.inputs.len(), 1);
        assert_eq!(marshalled.inputs[0].0, [0x11u8; 32]);
        assert_eq!(marshalled.inputs[0].1, 3);
        assert_eq!(marshalled.inputs[0].2, Some(addr.to_string()));
    }

    #[test]
    fn network_changes_rendered_addresses() {
        let addr = DashAddress::dummy(Network::Testnet, 7);
        let tx = Transaction::dummy(&addr, 1..2, &[151_072]);
        let marshalled = marshal_ok(&tx, Network::Mainnet);
        let rendered = marshalled.outputs[0].0.clone().unwrap();
        assert_ne!(rendered, addr.to_string());
        assert!(rendered.starts_with('X'), "mainnet P2PKH starts with 'X'");
    }

    #[test]
    fn coinbase_input_has_no_address() {
        let addr = DashAddress::dummy(Network::Testnet, 3);
        let tx = Transaction::dummy_coinbase(&addr, 50_000);
        let marshalled = marshal_ok(&tx, Network::Testnet);
        assert!(marshalled.inputs[0].2.is_none());
    }

    #[test]
    fn non_p2pkh_script_sig_yields_no_input_address() {
        // Transaction::dummy fills script_sig with a *lock* script
        // (OP_DUP OP_HASH160 <20 B> OP_EQUALVERIFY OP_CHECKSIG): opcodes
        // present, last push 20 bytes — must not produce an address.
        let addr = DashAddress::dummy(Network::Testnet, 9);
        let tx = Transaction::dummy(&addr, 1..2, &[1_000]);
        let marshalled = marshal_ok(&tx, Network::Testnet);
        assert!(marshalled.inputs[0].2.is_none());
    }

    #[test]
    fn redeem_script_collision_yields_no_input_address() {
        // Three pushes ending in a valid 33-byte pubkey — the P2SH
        // redeem-script shape the exactly-two-pushes rule exists to reject.
        let script_sig = dashcore::blockdata::script::Builder::new()
            .push_slice([0x30u8; 71])
            .push_slice([0x01u8; 20])
            .push_key(&test_pubkey())
            .into_script();
        let marshalled = marshal_ok(&tx_with_script_sig(script_sig), Network::Testnet);
        assert!(marshalled.inputs[0].2.is_none());
    }

    #[test]
    fn non_signature_first_push_yields_no_input_address() {
        // Two pushes, but the first is not DER-shaped (no 0x30 tag).
        let script_sig = dashcore::blockdata::script::Builder::new()
            .push_slice([0xAAu8; 10])
            .push_key(&test_pubkey())
            .into_script();
        let marshalled = marshal_ok(&tx_with_script_sig(script_sig), Network::Testnet);
        assert!(marshalled.inputs[0].2.is_none());
    }

    #[test]
    fn from_populates_whole_struct() {
        // Exercises `From` end to end (bytes, fee, txid, and the C-side
        // input/output layout) with a raw pointer walk that pins the memory
        // layout. The struct's `Drop` frees everything at scope end.
        let (tx, _) = p2pkh_spend_tx(Network::Testnet);
        let ffi = FFICoreTransaction::from((&tx, 4321, Network::Testnet));

        assert_eq!(ffi.fee, 4321);
        assert_eq!(ffi.txid, tx.txid().to_byte_array());
        assert_eq!(ffi.bytes(), dashcore::consensus::serialize(&tx).as_slice());

        unsafe {
            let inputs = std::slice::from_raw_parts(ffi.inputs, ffi.inputs_count);
            let outputs = std::slice::from_raw_parts(ffi.outputs, ffi.outputs_count);
            assert_eq!(ffi.inputs_count, 1);
            assert_eq!(ffi.outputs_count, 2);
            assert_eq!(inputs[0].prev_vout, 3);
            assert!(!inputs[0].address.is_null());
            assert_eq!(outputs[0].value_duffs, 151_072);
            let addr = CStr::from_ptr(outputs[0].address).to_str().unwrap();
            assert!(addr.starts_with('y'));
            assert!(outputs[0].script_pubkey_len > 0);
        }
    }

    #[test]
    fn transaction_free_is_null_safe_and_frees_boxed() {
        unsafe {
            // Null is a no-op.
            core_wallet_transaction_free(std::ptr::null_mut());

            // A heap-owned tx is reclaimed and its `Drop` frees every
            // allocation (run under Miri to catch leaks / double-frees).
            let (tx, _) = p2pkh_spend_tx(Network::Testnet);
            let boxed = Box::into_raw(Box::new(FFICoreTransaction::from((
                &tx,
                10,
                Network::Testnet,
            ))));
            core_wallet_transaction_free(boxed);
        }
    }
}

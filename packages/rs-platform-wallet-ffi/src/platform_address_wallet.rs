//! FFI bindings for PlatformAddressWallet operations.

use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use platform_wallet::wallet::platform_addresses::InputSelection;
use std::panic;

/// Shared tokio runtime for blocking on async wallet operations.
fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: once_cell::sync::Lazy<tokio::runtime::Runtime> = once_cell::sync::Lazy::new(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for platform-wallet-ffi")
    });
    &RT
}

// ---------------------------------------------------------------------------
// Handle management
// ---------------------------------------------------------------------------

/// Destroy a PlatformAddressWallet handle, releasing its resources.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_destroy(
    handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let _ = out_error;
    PLATFORM_ADDRESS_WALLET_STORAGE.remove(handle);
    PlatformWalletFFIResult::Success
}

// ---------------------------------------------------------------------------
// Simple queries
// ---------------------------------------------------------------------------

/// Add a provider for a new account index.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_add_provider(
    handle: Handle,
    account_index: u32,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| match wallet.add_provider(account_index) {
            Ok(()) => PlatformWalletFFIResult::Success,
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorWalletOperation,
                        e.to_string(),
                    );
                }
                PlatformWalletFFIResult::ErrorWalletOperation
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Get total platform credits across all addresses.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_total_credits(
    handle: Handle,
    out_credits: *mut u64,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_credits.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            let credits = runtime().block_on(wallet.total_credits());
            *out_credits = credits;
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Get all platform addresses with their cached balances.
///
/// On success, `out_entries` and `out_count` are set to a heap-allocated array.
/// Free with `platform_address_wallet_free_address_balances`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_addresses_with_balances(
    handle: Handle,
    out_entries: *mut *mut AddressBalanceEntryFFI,
    out_count: *mut usize,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_entries.is_null() || out_count.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            let balances = runtime().block_on(wallet.addresses_with_balances());
            let entries: Vec<AddressBalanceEntryFFI> = balances
                .into_iter()
                .map(|(address, balance)| AddressBalanceEntryFFI {
                    address: address.into(),
                    balance,
                })
                .collect();
            *out_count = entries.len();
            if entries.is_empty() {
                *out_entries = std::ptr::null_mut();
            } else {
                *out_entries =
                    Box::into_raw(entries.into_boxed_slice()) as *mut AddressBalanceEntryFFI;
            }
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

/// Sync platform address balances across all accounts.
///
/// Free results with `platform_address_wallet_free_sync_result_array` and
/// `platform_address_wallet_free_changeset`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_sync_balances(
    handle: Handle,
    has_config: bool,
    config: AddressSyncConfigFFI,
    out_results: *mut AddressSyncResultArrayFFI,
    out_changeset: *mut PlatformAddressChangeSetFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_results.is_null() || out_changeset.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let config_opt = if has_config {
        Some(dash_sdk::platform::address_sync::AddressSyncConfig::from(
            config,
        ))
    } else {
        None
    };

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.sync_balances(config_opt)) {
                Ok((results, changeset)) => {
                    let ffi_results: Vec<AddressSyncResultFFI> =
                        results.iter().map(AddressSyncResultFFI::from).collect();
                    let count = ffi_results.len();
                    let ptr = if ffi_results.is_empty() {
                        std::ptr::null_mut()
                    } else {
                        Box::into_raw(ffi_results.into_boxed_slice()) as *mut AddressSyncResultFFI
                    };
                    *out_results = AddressSyncResultArrayFFI {
                        results: ptr,
                        count,
                    };
                    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Sync platform address balances for a single account.
///
/// Free results with `platform_address_wallet_free_sync_result` and
/// `platform_address_wallet_free_changeset`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_sync_balances_on_account(
    handle: Handle,
    account_index: u32,
    has_config: bool,
    config: AddressSyncConfigFFI,
    out_result: *mut AddressSyncResultFFI,
    out_changeset: *mut PlatformAddressChangeSetFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_result.is_null() || out_changeset.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let config_opt = if has_config {
        Some(dash_sdk::platform::address_sync::AddressSyncConfig::from(
            config,
        ))
    } else {
        None
    };

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime()
                .block_on(wallet.sync_balances_on_account_index(account_index, config_opt))
            {
                Ok((result, changeset)) => {
                    *out_result = AddressSyncResultFFI::from(&result);
                    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

// ---------------------------------------------------------------------------
// Transfer
// ---------------------------------------------------------------------------

/// Transfer credits between platform addresses.
///
/// Free result with `platform_address_wallet_free_changeset`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_transfer(
    handle: Handle,
    account_index: u32,
    input_type: InputSelectionType,
    explicit_inputs: *const ExplicitInputFFI,
    explicit_inputs_count: usize,
    nonce_inputs: *const ExplicitInputWithNonceFFI,
    nonce_inputs_count: usize,
    outputs: *const AddressBalanceEntryFFI,
    outputs_count: usize,
    fee_strategy: *const FeeStrategyStepFFI,
    fee_strategy_count: usize,
    out_changeset: *mut PlatformAddressChangeSetFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_changeset.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let output_map = match parse_outputs(outputs, outputs_count) {
        Ok(m) => m,
        Err(e) => {
            if !out_error.is_null() {
                *out_error =
                    PlatformWalletFFIError::new(PlatformWalletFFIResult::ErrorInvalidParameter, e);
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

    let input_selection = match input_type {
        InputSelectionType::Auto => InputSelection::Auto,
        InputSelectionType::Explicit => {
            match parse_explicit_inputs(explicit_inputs, explicit_inputs_count) {
                Ok(m) => InputSelection::Explicit(m),
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidParameter,
                            e,
                        );
                    }
                    return PlatformWalletFFIResult::ErrorInvalidParameter;
                }
            }
        }
        InputSelectionType::ExplicitWithNonces => {
            match parse_explicit_inputs_with_nonces(nonce_inputs, nonce_inputs_count) {
                Ok(m) => InputSelection::ExplicitWithNonces(m),
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidParameter,
                            e,
                        );
                    }
                    return PlatformWalletFFIResult::ErrorInvalidParameter;
                }
            }
        }
    };

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.transfer(
                account_index,
                input_selection,
                output_map,
                fee,
                None, // platform_version = latest
            )) {
                Ok(changeset) => {
                    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

// ---------------------------------------------------------------------------
// Withdraw
// ---------------------------------------------------------------------------

/// Withdraw platform credits to a Core L1 address.
///
/// Free result with `platform_address_wallet_free_changeset`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_withdraw(
    handle: Handle,
    account_index: u32,
    input_type: InputSelectionType,
    explicit_inputs: *const ExplicitInputFFI,
    explicit_inputs_count: usize,
    nonce_inputs: *const ExplicitInputWithNonceFFI,
    nonce_inputs_count: usize,
    output_script: *const u8,
    output_script_len: usize,
    core_fee_per_byte: u32,
    fee_strategy: *const FeeStrategyStepFFI,
    fee_strategy_count: usize,
    out_changeset: *mut PlatformAddressChangeSetFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_changeset.is_null() || output_script.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let script_bytes = std::slice::from_raw_parts(output_script, output_script_len);
    let core_script = CoreScript::from_bytes(script_bytes.to_vec());

    let input_selection = match input_type {
        InputSelectionType::Auto => InputSelection::Auto,
        InputSelectionType::Explicit => {
            match parse_explicit_inputs(explicit_inputs, explicit_inputs_count) {
                Ok(m) => InputSelection::Explicit(m),
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidParameter,
                            e,
                        );
                    }
                    return PlatformWalletFFIResult::ErrorInvalidParameter;
                }
            }
        }
        InputSelectionType::ExplicitWithNonces => {
            match parse_explicit_inputs_with_nonces(nonce_inputs, nonce_inputs_count) {
                Ok(m) => InputSelection::ExplicitWithNonces(m),
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidParameter,
                            e,
                        );
                    }
                    return PlatformWalletFFIResult::ErrorInvalidParameter;
                }
            }
        }
    };

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.withdraw(
                account_index,
                input_selection,
                core_script,
                core_fee_per_byte,
                fee,
                None, // platform_version = latest
            )) {
                Ok(changeset) => {
                    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

// ---------------------------------------------------------------------------
// Fund from asset lock
// ---------------------------------------------------------------------------

/// Fund platform addresses from a Core L1 asset lock.
///
/// `asset_lock_proof_bytes` is the bincode-serialized `AssetLockProof`.
/// `private_key_bytes` must point to exactly 32 bytes.
/// Exactly one entry in `addresses` must have `has_balance = false`.
///
/// Free result with `platform_address_wallet_free_changeset`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_fund_from_asset_lock(
    handle: Handle,
    account_index: u32,
    addresses: *const FundingAddressEntryFFI,
    addresses_count: usize,
    asset_lock_proof_bytes: *const u8,
    asset_lock_proof_len: usize,
    private_key_bytes: *const u8,
    fee_strategy: *const FeeStrategyStepFFI,
    fee_strategy_count: usize,
    out_changeset: *mut PlatformAddressChangeSetFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_changeset.is_null()
        || addresses.is_null()
        || asset_lock_proof_bytes.is_null()
        || private_key_bytes.is_null()
    {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Parse addresses
    let mut address_map = std::collections::BTreeMap::new();
    for entry in std::slice::from_raw_parts(addresses, addresses_count) {
        let addr = match PlatformAddress::try_from(entry.address) {
            Ok(a) => a,
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        e,
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };
        let balance = if entry.has_balance {
            Some(entry.balance)
        } else {
            None
        };
        address_map.insert(addr, balance);
    }

    // Deserialize asset lock proof (bincode-encoded)
    let proof_bytes = std::slice::from_raw_parts(asset_lock_proof_bytes, asset_lock_proof_len);
    let asset_lock_proof: dpp::prelude::AssetLockProof =
        match dpp::bincode::decode_from_slice(proof_bytes, dpp::bincode::config::standard()) {
            Ok((proof, _)) => proof,
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorDeserialization,
                        format!("Failed to deserialize AssetLockProof: {}", e),
                    );
                }
                return PlatformWalletFFIResult::ErrorDeserialization;
            }
        };

    // Parse private key (network is irrelevant for raw key bytes)
    let key_bytes = std::slice::from_raw_parts(private_key_bytes, 32);
    let private_key = match dashcore::PrivateKey::from_slice(key_bytes, dashcore::Network::Mainnet)
    {
        Ok(k) => k,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("Invalid private key: {}", e),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.fund_from_asset_lock(
                account_index,
                address_map,
                asset_lock_proof,
                private_key,
                fee,
            )) {
                Ok(changeset) => {
                    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

// ---------------------------------------------------------------------------
// Memory deallocation
// ---------------------------------------------------------------------------

/// Free an address balances array returned by `platform_address_wallet_addresses_with_balances`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_address_balances(
    entries: *mut AddressBalanceEntryFFI,
    count: usize,
) {
    if !entries.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
    }
}

/// Free a changeset returned by transfer/withdraw/fund/sync operations.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_changeset(
    changeset: PlatformAddressChangeSetFFI,
) {
    if !changeset.updated.is_null() && changeset.updated_count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            changeset.updated,
            changeset.updated_count,
        ));
    }
    if !changeset.removed.is_null() && changeset.removed_count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            changeset.removed,
            changeset.removed_count,
        ));
    }
}

/// Free a single sync result.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_sync_result(result: AddressSyncResultFFI) {
    if !result.found.is_null() && result.found_count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            result.found,
            result.found_count,
        ));
    }
    if !result.absent.is_null() && result.absent_count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            result.absent,
            result.absent_count,
        ));
    }
}

/// Free a sync result array returned by `platform_address_wallet_sync_balances`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_sync_result_array(
    array: AddressSyncResultArrayFFI,
) {
    if !array.results.is_null() && array.count > 0 {
        let results = std::slice::from_raw_parts(array.results, array.count);
        for result in results {
            // Free inner arrays (found/absent) — read-only, don't call free_sync_result
            // which takes ownership. Instead manually free:
            if !result.found.is_null() && result.found_count > 0 {
                let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    result.found as *mut FoundAddressEntryFFI,
                    result.found_count,
                ));
            }
            if !result.absent.is_null() && result.absent_count > 0 {
                let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    result.absent as *mut AbsentAddressEntryFFI,
                    result.absent_count,
                ));
            }
        }
        // Free the outer array
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            array.results,
            array.count,
        ));
    }
}

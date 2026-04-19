//! FFI bindings for funding platform addresses from asset locks.

use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use dpp::address_funds::PlatformAddress;

use super::runtime;

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
    let key_array: &[u8; 32] = match key_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    "Private key must be exactly 32 bytes",
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };
    let private_key =
        match dashcore::PrivateKey::from_byte_array(key_array, dashcore::Network::Mainnet) {
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

//! FFI bindings for funding platform addresses from asset locks.

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};
use dpp::address_funds::PlatformAddress;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::runtime;

/// Fund platform addresses from a Core L1 asset lock.
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
    signer_address_handle: *mut SignerHandle,
    out_changeset: *mut PlatformAddressChangeSetFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_changeset);
    check_ptr!(addresses);
    check_ptr!(asset_lock_proof_bytes);
    check_ptr!(private_key_bytes);
    check_ptr!(signer_address_handle);

    let mut address_map = std::collections::BTreeMap::new();
    for entry in std::slice::from_raw_parts(addresses, addresses_count) {
        let addr = unwrap_result_or_return!(PlatformAddress::try_from(entry.address));
        let balance = if entry.has_balance {
            Some(entry.balance)
        } else {
            None
        };
        address_map.insert(addr, balance);
    }

    let proof_bytes = std::slice::from_raw_parts(asset_lock_proof_bytes, asset_lock_proof_len);
    let (asset_lock_proof, _): (dpp::prelude::AssetLockProof, usize) = unwrap_result_or_return!(
        dpp::bincode::decode_from_slice(proof_bytes, dpp::bincode::config::standard(),)
    );

    let key_array = &*(private_key_bytes as *const [u8; 32]);

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    let address_signer: &VTableSigner = &*(signer_address_handle as *const VTableSigner);

    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        // Pull the network from the wallet so `PrivateKey::network`
        // matches the SPV chain we're signing for. The earlier
        // hardcoded `Network::Mainnet` quietly produced a testnet/
        // devnet PrivateKey wearing a mainnet label — harmless for
        // ECDSA signing itself (the `network` field is purely a
        // serialization tag) but a footgun if the value was ever
        // exposed back over an FFI.
        let private_key = match dashcore::PrivateKey::from_byte_array(key_array, wallet.network()) {
            Ok(pk) => pk,
            Err(e) => {
                return Err(platform_wallet::PlatformWalletError::AddressOperation(
                    format!("invalid asset-lock private key bytes: {e}"),
                ));
            }
        };
        runtime().block_on(wallet.fund_from_asset_lock(
            account_index,
            address_map,
            asset_lock_proof,
            private_key,
            fee,
            address_signer,
        ))
    });
    let result = unwrap_option_or_return!(option);
    let changeset = unwrap_result_or_return!(result);
    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
    PlatformWalletFFIResult::ok()
}

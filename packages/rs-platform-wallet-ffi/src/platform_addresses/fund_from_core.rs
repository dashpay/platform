use dashcore::hashes::Hash;
use dpp::address_funds::PlatformAddress;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Fund one of the wallet's own platform addresses by locking
/// `amount_duffs` of Core L1 funds. On success writes the asset-lock
/// outpoint to `out_asset_lock_txid` / `out_asset_lock_vout`.
///
/// # Safety
/// - `mnemonic_resolver_handle` and `signer_address_handle` must be
///   pinned alive for the duration of the call.
/// - `out_*` pointers must reference writable memory.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_fund_platform_address_from_core(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    amount_duffs: u64,
    core_account_index: u32,
    platform_account_index: u32,
    target_address: PlatformAddressFFI,
    signer_address_handle: *mut SignerHandle,
    out_asset_lock_txid: *mut [u8; 32],
    out_asset_lock_vout: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(signer_address_handle);
    check_ptr!(out_asset_lock_txid);
    check_ptr!(out_asset_lock_vout);

    let target = unwrap_result_or_return!(PlatformAddress::try_from(target_address));

    // Pointers → usize so the spawned future is `Send + 'static`.
    let signer_addr = signer_address_handle as usize;
    let resolver_addr = mnemonic_resolver_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wallet = wallet.clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.sdk().network;
        block_on_worker(async move {
            let address_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
            let asset_lock_signer = unsafe {
                MnemonicResolverCoreSigner::new(
                    resolver_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            wallet
                .fund_platform_address_from_core(
                    amount_duffs,
                    core_account_index,
                    platform_account_index,
                    target,
                    &asset_lock_signer,
                    address_signer,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let outcome = unwrap_result_or_return!(result);

    *out_asset_lock_txid = outcome.asset_lock_txid.to_byte_array();
    *out_asset_lock_vout = outcome.asset_lock_vout;
    PlatformWalletFFIResult::ok()
}

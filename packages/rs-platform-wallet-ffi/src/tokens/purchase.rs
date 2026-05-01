//! FFI binding for `IdentityWallet::token_purchase_with_external_signer`.

use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Purchase `amount` of the token at `token_position` on `token_contract_id`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_purchase(
    wallet_handle: Handle,
    identity_id: *const u8,
    token_contract_id: *const u8,
    token_position: u16,
    amount: u64,
    expected_total_cost: u64,
    _signing_key_id: u32,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);

    let id = unwrap_result_or_return!(read_identifier(identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(token_contract_id));

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .token_purchase_with_external_signer(
                    id,
                    contract_id,
                    token_position,
                    amount,
                    expected_total_cost,
                    signer,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

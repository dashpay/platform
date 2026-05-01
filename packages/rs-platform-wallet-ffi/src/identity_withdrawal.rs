//! FFI bindings for identity → Core address withdrawal driven by an
//! external `SignerHandle`.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::str::FromStr;

use dashcore::Address as DashAddress;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Withdraw `amount` credits from `identity_id` to a Dash address.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_withdraw_credits_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    amount: u64,
    to_address: *const c_char,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(to_address);

    let id = unwrap_result_or_return!(read_identifier(identity_id));

    let to_address_str = unwrap_result_or_return!(CStr::from_ptr(to_address).to_str()).to_string();

    let to_address_unchecked = unwrap_result_or_return!(DashAddress::from_str(&to_address_str));

    let signer_addr = signer_handle as usize;

    let option =
        PLATFORM_WALLET_STORAGE.with_item(
            wallet_handle,
            |wallet| -> Result<
                Result<(), platform_wallet::PlatformWalletError>,
                PlatformWalletFFIResult,
            > {
                let wallet_network = wallet.platform().network();
                let to_address_parsed = to_address_unchecked
                    .clone()
                    .require_network(wallet_network)
                    .map_err(PlatformWalletFFIResult::from)?;
                let identity_wallet = wallet.identity().clone();
                Ok(block_on_worker(async move {
                    let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                    identity_wallet
                        .withdraw_credits_with_external_signer(
                            &id,
                            amount,
                            &to_address_parsed,
                            signer,
                            None,
                        )
                        .await
                }))
            },
        );
    let inner = unwrap_option_or_return!(option);
    let result = unwrap_result_or_return!(inner);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

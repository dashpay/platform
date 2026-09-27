//! FFI binding for raising the limits of one of an identity's keys (protocol
//! version 14), driven by an external `SignerHandle`.
//!
//! The identity's MASTER key, or a CRITICAL authentication key without limits
//! and without contract bounds, signs the `IdentityKeyLimitsUpdate` transition
//! via the supplied `signer_handle` (typically the iOS-side `KeychainSigner`).

use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Raise the limits of the key `key_id` of the identity: add `add_budget`
/// credits to its total budget (and to what is left of it) when
/// `has_add_budget`, and move its expiry to `expires_at` (block time in
/// milliseconds) when `has_expires_at`. At least one must be set; a limit
/// the key does not have, a zero top-up and an expiry that is not later
/// are refused before anything is signed, since Platform would refuse them
/// and charge for it.
///
/// The cached identity carries the key as stored after the update, and the
/// client's key row follows through the persistence changeset, as an
/// identity update does for an added key. No identity revision is claimed.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_update_identity_key_limits_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    key_id: u32,
    has_add_budget: bool,
    add_budget: u64,
    has_expires_at: bool,
    expires_at: u64,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);

    let id = unwrap_result_or_return!(read_identifier(identity_id));

    if !has_add_budget && !has_expires_at {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "a budget to add or a new expiry must be given".to_string(),
        );
    }
    let add_budget = has_add_budget.then_some(add_budget);
    let expires_at = has_expires_at.then_some(expires_at);

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .update_identity_key_limits_with_external_signer(
                    &id, key_id, add_budget, expires_at, signer, None,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

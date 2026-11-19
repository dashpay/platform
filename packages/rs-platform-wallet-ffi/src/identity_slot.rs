//! Identity slots for hosts: the index a registration or an invitation claim
//! derives its identity keys at. The wallet chooses it
//! (`IdentityWallet::acquire_identity_slot`); the host derives the keys there
//! and passes the slot back to the registration or claim call.

use std::ffi::CStr;
use std::os::raw::c_char;

use platform_wallet::wallet::identity::crypto::parse_invitation_uri;
use platform_wallet::wallet::identity::{SlotHolder, SlotLeasePhase, SlotRequest};
use platform_wallet::PlatformWalletError;

use crate::asset_lock_persistence::outpoint_to_bytes;
use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_slot_lease_persistence::outpoint_from_bytes;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Lease an identity slot for a new registration.
///
/// Writes the index to derive the identity keys at to `*out_identity_index`,
/// and the slot's token to `*out_slot_token`: pass both to
/// `platform_wallet_register_identity_with_funding_signer`, or the token to
/// [`platform_wallet_identity_slot_release_registration`] when the
/// registration is abandoned. The lease lives in memory only.
///
/// # Safety
/// `out_identity_index` and `out_slot_token` must be valid writable pointers.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_slot_acquire_registration(
    wallet_handle: Handle,
    out_identity_index: *mut u32,
    out_slot_token: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_identity_index);
    check_ptr!(out_slot_token);
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity_wallet = unwrap_option_or_return!(option);
    let slot = unwrap_result_or_return!(block_on_worker(async move {
        identity_wallet
            .acquire_identity_slot(SlotRequest::Registration)
            .await
    }));
    let SlotHolder::Registration { token } = slot.holder else {
        return PlatformWalletError::InvalidParameter(
            "a registration slot was leased to an invitation claim".to_string(),
        )
        .into();
    };
    *out_identity_index = slot.identity_index;
    *out_slot_token = token;
    PlatformWalletFFIResult::ok()
}

/// Lease the identity slot for a claim of the invitation link `uri`. A claim
/// retried gets the slot its earlier attempt leased.
///
/// Costs one funding-tx fetch. Writes the index to derive the identity keys
/// at to `*out_identity_index`, the invitation's funding outpoint (32-byte raw
/// txid, then the 4-byte little-endian vout) to `*out_invitation_out_point`
/// for [`platform_wallet_identity_slot_release_invitation_claim`], and whether
/// an earlier attempt of this claim may have reached Platform to
/// `*out_submitted`. The lease is stored durably; a persister that cannot keep
/// it refuses the claim.
///
/// # Safety
/// - `uri` must be a valid NUL-terminated UTF-8 C string.
/// - The out pointers must be valid and writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_slot_acquire_invitation_claim(
    wallet_handle: Handle,
    uri: *const c_char,
    out_identity_index: *mut u32,
    out_invitation_out_point: *mut [u8; 36],
    out_submitted: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(uri);
    check_ptr!(out_identity_index);
    check_ptr!(out_invitation_out_point);
    check_ptr!(out_submitted);
    let uri = unwrap_result_or_return!(CStr::from_ptr(uri).to_str()).to_string();
    let invitation =
        unwrap_result_or_return!(parse_invitation_uri(&uri).map_err(PlatformWalletError::from));
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity_wallet = unwrap_option_or_return!(option);
    let slot = unwrap_result_or_return!(block_on_worker(async move {
        identity_wallet
            .acquire_identity_slot(SlotRequest::InvitationClaim(&invitation))
            .await
    }));
    let SlotHolder::InvitationClaim { invitation } = slot.holder else {
        return PlatformWalletError::InvalidParameter(
            "an invitation claim slot was leased to a registration".to_string(),
        )
        .into();
    };
    *out_identity_index = slot.identity_index;
    *out_invitation_out_point = outpoint_to_bytes(&invitation);
    *out_submitted = slot.phase == SlotLeasePhase::Submitted;
    PlatformWalletFFIResult::ok()
}

/// Give up the slot of an abandoned registration, by the token
/// [`platform_wallet_identity_slot_acquire_registration`] wrote. A token that
/// holds no slot is not an error.
#[no_mangle]
pub extern "C" fn platform_wallet_identity_slot_release_registration(
    wallet_handle: Handle,
    slot_token: u64,
) -> PlatformWalletFFIResult {
    release(
        wallet_handle,
        SlotHolder::Registration { token: slot_token },
    )
}

/// Give up the slot of an invitation claim abandoned before it was sent, by
/// the outpoint [`platform_wallet_identity_slot_acquire_invitation_claim`]
/// wrote. Refused when the claim may have reached Platform; its outcome frees
/// the slot then. The release is stored durably.
///
/// # Safety
/// `invitation_out_point` must point to 36 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_slot_release_invitation_claim(
    wallet_handle: Handle,
    invitation_out_point: *const [u8; 36],
) -> PlatformWalletFFIResult {
    check_ptr!(invitation_out_point);
    let invitation = outpoint_from_bytes(&*invitation_out_point);
    release(wallet_handle, SlotHolder::InvitationClaim { invitation })
}

fn release(wallet_handle: Handle, holder: SlotHolder) -> PlatformWalletFFIResult {
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity_wallet = unwrap_option_or_return!(option);
    unwrap_result_or_return!(block_on_worker(async move {
        identity_wallet.release_identity_slot(holder).await
    }));
    PlatformWalletFFIResult::ok()
}

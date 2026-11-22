//! FFI bindings for raising the limits of one of an identity's keys
//! (protocol version 14), driven by an external `SignerHandle`.
//!
//! [`platform_wallet_preview_identity_key_limits_update`] asks first,
//! signing and sending nothing: whether the key can be raised at all,
//! which key would sign, the limits the key carries, and the limits a
//! given change would set. [`platform_wallet_update_identity_key_limits_with_signer`]
//! then raises them. Both call into
//! `platform_wallet::IdentityWallet`, which owns every decision; the
//! update runs the same preview, so the two cannot disagree.
//!
//! The identity's MASTER key, or a CRITICAL authentication key without limits
//! and without contract bounds, signs the `IdentityKeyLimitsUpdate` transition
//! via the supplied `signer_handle` (typically the iOS-side `KeychainSigner`).

use dpp::prelude::Identifier;
use platform_wallet::IdentityKeyLimitsUpdatePreview;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// What a key limits update of one key would do, as
/// [`platform_wallet_preview_identity_key_limits_update`] reports it.
/// Plain data: nothing to free. Each optional value is a `has_*` flag and
/// the value, which is meaningful only when the flag is set.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IdentityKeyLimitsUpdatePreviewFFI {
    /// The identity whose key the update raises.
    pub identity_id: [u8; 32],
    /// The key the update raises.
    pub key_id: u32,
    /// The key that would sign the update.
    pub signing_key_id: u32,
    /// The key carries a total budget now.
    pub has_current_total_budget: bool,
    /// The total budget the key carries now, in credits.
    pub current_total_budget: u64,
    /// The key carries an expiry now.
    pub has_current_expires_at: bool,
    /// The expiry the key carries now, block time in milliseconds.
    pub current_expires_at: u64,
    /// The update would set a new total budget.
    pub has_new_total_budget: bool,
    /// The total budget the update would set, in credits.
    pub new_total_budget: u64,
    /// The update would set a new expiry.
    pub has_new_expires_at: bool,
    /// The expiry the update would set, block time in milliseconds.
    pub new_expires_at: u64,
}

impl From<IdentityKeyLimitsUpdatePreviewFFI> for IdentityKeyLimitsUpdatePreview {
    fn from(preview: IdentityKeyLimitsUpdatePreviewFFI) -> Self {
        Self {
            identity_id: Identifier::from(preview.identity_id),
            key_id: preview.key_id,
            signing_key_id: preview.signing_key_id,
            current_total_budget: preview
                .has_current_total_budget
                .then_some(preview.current_total_budget),
            current_expires_at: preview
                .has_current_expires_at
                .then_some(preview.current_expires_at),
            new_total_budget: preview
                .has_new_total_budget
                .then_some(preview.new_total_budget),
            new_expires_at: preview.has_new_expires_at.then_some(preview.new_expires_at),
        }
    }
}

impl From<IdentityKeyLimitsUpdatePreview> for IdentityKeyLimitsUpdatePreviewFFI {
    fn from(preview: IdentityKeyLimitsUpdatePreview) -> Self {
        Self {
            identity_id: preview.identity_id.to_buffer(),
            key_id: preview.key_id,
            signing_key_id: preview.signing_key_id,
            has_current_total_budget: preview.current_total_budget.is_some(),
            current_total_budget: preview.current_total_budget.unwrap_or_default(),
            has_current_expires_at: preview.current_expires_at.is_some(),
            current_expires_at: preview.current_expires_at.unwrap_or_default(),
            has_new_total_budget: preview.new_total_budget.is_some(),
            new_total_budget: preview.new_total_budget.unwrap_or_default(),
            has_new_expires_at: preview.new_expires_at.is_some(),
            new_expires_at: preview.new_expires_at.unwrap_or_default(),
        }
    }
}

/// What raising the limits of the key `key_id` of the identity would do,
/// without signing or sending anything: the key that would sign, the
/// limits the key carries, and the limits the update would set for adding
/// `add_budget` credits (when `has_add_budget`) and moving the expiry to
/// `expires_at` (block time in milliseconds, when `has_expires_at`).
///
/// With neither flag set it answers whether the key can be raised at all
/// and by which key, which is what a client asks before it offers the
/// action. A refusal is an error result whose message carries the reason:
/// the key is missing or disabled, it has no limits, the change is not a
/// raise, or the signer holds no key allowed to sign the update. The
/// answer comes from the identity as the wallet holds it.
///
/// On success `*out_preview` is written; it holds no heap memory.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_preview_identity_key_limits_update(
    wallet_handle: Handle,
    identity_id: *const u8,
    key_id: u32,
    has_add_budget: bool,
    add_budget: u64,
    has_expires_at: bool,
    expires_at: u64,
    signer_handle: *mut SignerHandle,
    out_preview: *mut IdentityKeyLimitsUpdatePreviewFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(out_preview);

    let id = unwrap_result_or_return!(read_identifier(identity_id));

    let add_budget = has_add_budget.then_some(add_budget);
    let expires_at = has_expires_at.then_some(expires_at);

    let signer_addr = signer_handle as usize;

    // Clone the identity wallet under the registry guard, but run outside it: the preview
    // calls the host signer's `can_sign_with`, and a host that destroys a wallet handle from
    // that callback would wait for the registry write guard this call would still hold.
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity_wallet = unwrap_option_or_return!(option);
    let result = block_on_worker(async move {
        let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
        identity_wallet
            .preview_identity_key_limits_update(&id, key_id, add_budget, expires_at, signer)
            .await
    });
    let preview = unwrap_result_or_return!(result);
    *out_preview = preview.into();
    PlatformWalletFFIResult::ok()
}

/// Raise the limits of the key `key_id` of the identity: add `add_budget`
/// credits to its total budget (and to what is left of it) when
/// `has_add_budget`, and move its expiry to `expires_at` (block time in
/// milliseconds) when `has_expires_at`. At least one must be set; a
/// disabled key, a limit the key does not have, a zero top-up and an expiry
/// that is not later are refused before anything is signed, since Platform
/// would refuse them and charge for it.
///
/// A client that showed a preview first signs with
/// [`platform_wallet_update_identity_key_limits_as_reviewed_with_signer`]
/// instead, which signs exactly what that preview showed or nothing.
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

    // Run outside the registry guard, as the preview does: signing calls back into the host.
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity_wallet = unwrap_option_or_return!(option);
    let result = block_on_worker(async move {
        let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
        identity_wallet
            .update_identity_key_limits_with_external_signer(
                &id, key_id, add_budget, expires_at, signer, None,
            )
            .await
    });
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Raise the limits of the key `reviewed_preview` names, of the identity it
/// names, to exactly what it shows: the preview
/// [`platform_wallet_preview_identity_key_limits_update`] returned for the
/// change and the caller showed. The preview carries its target, so it
/// cannot be applied to another key. The update signs that new total, that
/// expiry and that signing key, or nothing: when the wallet's identity
/// moved since the preview, so the same change would now sign other limits
/// or another key, it is refused before a nonce is taken or anything is
/// signed. Preview again and show the new values.
///
/// The cached identity carries the key as stored after the update, and the
/// client's key row follows through the persistence changeset.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_update_identity_key_limits_as_reviewed_with_signer(
    wallet_handle: Handle,
    reviewed_preview: *const IdentityKeyLimitsUpdatePreviewFFI,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(reviewed_preview);
    check_ptr!(signer_handle);

    let reviewed: IdentityKeyLimitsUpdatePreview = (*reviewed_preview).into();

    let signer_addr = signer_handle as usize;

    // Run outside the registry guard, as the preview does: signing calls back into the host.
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity_wallet = unwrap_option_or_return!(option);
    let result = block_on_worker(async move {
        let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
        identity_wallet
            .update_identity_key_limits_as_reviewed_with_external_signer(&reviewed, signer, None)
            .await
    });
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::NonNull;

    /// A signer pointer that is not null and is never dereferenced: every
    /// call below returns before the wallet's closure runs.
    fn unreachable_signer() -> *mut SignerHandle {
        NonNull::<SignerHandle>::dangling().as_ptr()
    }

    #[test]
    fn should_refuse_null_pointers_before_touching_the_wallet() {
        let identity_id = [1u8; 32];
        let mut out = IdentityKeyLimitsUpdatePreviewFFI::default();

        let result = unsafe {
            platform_wallet_preview_identity_key_limits_update(
                Handle::MAX,
                identity_id.as_ptr(),
                2,
                false,
                0,
                false,
                0,
                std::ptr::null_mut(),
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        let result = unsafe {
            platform_wallet_preview_identity_key_limits_update(
                Handle::MAX,
                identity_id.as_ptr(),
                2,
                false,
                0,
                false,
                0,
                unreachable_signer(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    #[test]
    fn should_accept_a_preview_without_a_change_and_answer_an_unknown_wallet_as_not_found() {
        let identity_id = [1u8; 32];
        let mut out = IdentityKeyLimitsUpdatePreviewFFI::default();
        let result = unsafe {
            platform_wallet_preview_identity_key_limits_update(
                Handle::MAX,
                identity_id.as_ptr(),
                2,
                false,
                0,
                false,
                0,
                unreachable_signer(),
                &mut out,
            )
        };
        // Not `ErrorInvalidParameter`: unlike the update, the preview takes
        // no change at all, so it reaches the wallet lookup.
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
        assert_eq!(out, IdentityKeyLimitsUpdatePreviewFFI::default());
    }

    #[test]
    fn should_refuse_a_reviewed_update_without_a_preview_or_a_signer() {
        let preview = IdentityKeyLimitsUpdatePreviewFFI::default();
        let result = unsafe {
            platform_wallet_update_identity_key_limits_as_reviewed_with_signer(
                Handle::MAX,
                std::ptr::null(),
                unreachable_signer(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        let result = unsafe {
            platform_wallet_update_identity_key_limits_as_reviewed_with_signer(
                Handle::MAX,
                &preview,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    #[test]
    fn should_read_back_the_preview_it_wrote() {
        let preview = IdentityKeyLimitsUpdatePreview {
            identity_id: Identifier::from([9; 32]),
            key_id: 4,
            signing_key_id: 1,
            current_total_budget: None,
            current_expires_at: Some(10),
            new_total_budget: None,
            new_expires_at: Some(20),
        };
        let ffi: IdentityKeyLimitsUpdatePreviewFFI = preview.into();
        assert_eq!(IdentityKeyLimitsUpdatePreview::from(ffi), preview);
    }

    #[test]
    fn should_carry_each_optional_limit_as_a_flag_and_a_value() {
        let ffi: IdentityKeyLimitsUpdatePreviewFFI = IdentityKeyLimitsUpdatePreview {
            identity_id: Identifier::from([9; 32]),
            key_id: 2,
            signing_key_id: 0,
            current_total_budget: Some(1_000),
            current_expires_at: None,
            new_total_budget: Some(1_500),
            new_expires_at: None,
        }
        .into();
        assert_eq!(
            ffi,
            IdentityKeyLimitsUpdatePreviewFFI {
                identity_id: [9; 32],
                key_id: 2,
                signing_key_id: 0,
                has_current_total_budget: true,
                current_total_budget: 1_000,
                has_current_expires_at: false,
                current_expires_at: 0,
                has_new_total_budget: true,
                new_total_budget: 1_500,
                has_new_expires_at: false,
                new_expires_at: 0,
            }
        );
    }
}

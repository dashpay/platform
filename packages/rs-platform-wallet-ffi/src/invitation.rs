//! FFI bindings for DashPay invitations (DIP-13 sub-feature 3').
//!
//! Two entry points wrap
//! [`IdentityWallet`](platform_wallet::IdentityWallet)'s invitation flow:
//!
//! - [`platform_wallet_create_invitation`] (inviter) — fund a one-time
//!   asset-lock voucher at the invitation derivation path, export the voucher
//!   key, and return a shareable `dashpay://invite` link. **Only the Core-side
//!   resolver signer is needed** (no identity signer): this is pure voucher
//!   creation, no identity is registered. The single resolver handle is used
//!   twice — as the asset-lock signer (funding-input + credit-output
//!   signatures) and, wrapped as a [`ContactCryptoProvider`], to export the
//!   voucher private key at the path-gated invitation sub-feature.
//! - [`platform_wallet_claim_invitation`] (invitee) — parse the link and
//!   register a NEW identity for the invitee funded by the imported voucher.
//!   The invitee's own identity keys are signed by the supplied `SignerHandle`;
//!   the asset-lock's outer signature uses the imported raw voucher key, so no
//!   Core-side resolver signer is needed here.
//!
//! The link returned by create **contains the plaintext voucher key** — it is a
//! bearer credential. Callers MUST NOT log or persist it (mirrors the treatment
//! of the auto-accept `dapk` URI in [`crate::dashpay`]).
//!
//! Marshaling mirrors
//! [`crate::identity_registration_funded_with_signer`] (signer-handle `usize`
//! round-trip, `block_on_worker`, `MANAGED_IDENTITY_STORAGE` insert).

use std::ffi::CStr;
use std::os::raw::c_char;

use dpp::identity::accessors::IdentityGettersV0;
use platform_wallet::wallet::identity::crypto::{parse_invitation_uri, InviterInfo};
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle, SignerHandle, VTableSigner};

use platform_wallet::wallet::identity::network::MAX_INVITATION_TTL_SECS;

use crate::core_wallet_types::OutPointFFI;
use crate::dashpay::resolver_contact_crypto_provider;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::{decode_identity_pubkeys, IdentityPubkeyFFI};
use crate::runtime::block_on_worker;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

/// Create a DashPay invitation: fund a one-time asset-lock voucher at the
/// DIP-13 invitation path and return a shareable `dashpay://invite` link.
///
/// `inviter_identity_id` / `inviter_username` are **optional**: pass a non-null
/// 32-byte `inviter_identity_id` to opt into the contact-bootstrap (the link
/// then carries the inviter so the invitee can send a contact request back), in
/// which case `inviter_username` is **required** (non-null). Pass a null
/// `inviter_identity_id` for a pure funding voucher; `inviter_username` is then
/// ignored. The optional display name is not carried through this FFI (`None`).
///
/// `now_unix` is the current unix time in seconds, passed in from Swift (the
/// FFI can't read the clock deterministically). The advisory expiry is derived
/// as `now_unix + MAX_INVITATION_TTL_SECS` (a fixed ~24h window inside the
/// InstantSend validity bound); `now_unix == 0` is rejected to catch a failed
/// clock read (which would otherwise produce a 1970-relative expiry).
///
/// On success writes the link to `*out_uri` (heap C string; release with
/// [`crate::platform_wallet_string_free`]) and the funding outpoint to
/// `*out_outpoint`. **The URI embeds the bearer voucher key — never log it.**
///
/// # Safety
/// - `inviter_identity_id` is either null or points to 32 readable bytes (the
///   `*const u8` identity-id convention shared with `read_identifier` /
///   `platform_wallet_build_auto_accept_qr`).
/// - `inviter_username` is either null or a valid NUL-terminated UTF-8 C string.
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle` produced by
///   [`crate::dash_sdk_mnemonic_resolver_create`]. The caller retains ownership.
/// - `out_uri` must be a valid `*mut *mut c_char`; `out_outpoint` a valid
///   `*mut OutPointFFI`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_create_invitation(
    wallet_handle: Handle,
    amount_duffs: u64,
    funding_account_index: u32,
    inviter_identity_id: *const u8,
    inviter_username: *const c_char,
    now_unix: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_uri: *mut *mut c_char,
    out_outpoint: *mut OutPointFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    check_ptr!(out_uri);
    check_ptr!(out_outpoint);
    // Publish FFI-safe sentinels before any fallible work so every early return
    // leaves the out-params well-defined (never uninitialized bytes a
    // cleanup-on-error caller might read or free).
    unsafe {
        *out_uri = std::ptr::null_mut();
        *out_outpoint = OutPointFFI {
            txid: [0u8; 32],
            vout: 0,
        };
    }

    // Reject a failed clock read up front (a zero `now` would derive a
    // 1970-relative expiry). The core `create_invitation` also guards the
    // resulting `expiry_unix == 0`, but catching `now == 0` here gives a
    // clearer, earlier error.
    if now_unix == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "now_unix must be a valid unix timestamp (non-zero)",
        );
    }
    // Derive the advisory expiry: a fixed ~24h window from now, inside the
    // InstantSend validity bound (spec §5.1). `saturating_add` can't overflow a
    // realistic `now`, but keeps the arithmetic total.
    let expiry_unix = now_unix.saturating_add(MAX_INVITATION_TTL_SECS);

    // Build the optional inviter info: present iff `inviter_identity_id` is
    // non-null, and the username is required in that case.
    let inviter: Option<InviterInfo> = if inviter_identity_id.is_null() {
        None
    } else {
        if inviter_username.is_null() {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "inviter_username is required when inviter_identity_id is provided",
            );
        }
        let mut identity_id = [0u8; 32];
        unsafe {
            std::ptr::copy_nonoverlapping(inviter_identity_id, identity_id.as_mut_ptr(), 32);
        }
        let username =
            unwrap_result_or_return!(unsafe { CStr::from_ptr(inviter_username) }.to_str())
                .to_string();
        Some(InviterInfo {
            identity_id,
            username,
            display_name: None,
        })
    };

    // Round-trip the handle through `usize` so the spawned future's capture is
    // `Send + 'static` (raw pointers are `!Send`).
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        block_on_worker(async move {
            // SAFETY: see the fn-level safety doc — the caller pins
            // `core_signer_handle` for the duration of this call. Two views over
            // the same resolver handle: one as the asset-lock/Core signer, one
            // wrapped as the `ContactCryptoProvider` used to export the voucher
            // key. Both are `Send + Sync` and dropped when this task completes.
            let asset_lock_signer = unsafe {
                MnemonicResolverCoreSigner::new(
                    core_signer_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            let provider = unsafe {
                resolver_contact_crypto_provider(
                    core_signer_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            identity_wallet
                .create_invitation(
                    amount_duffs,
                    funding_account_index,
                    inviter,
                    expiry_unix,
                    &asset_lock_signer,
                    &provider,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let invitation = unwrap_result_or_return!(result);

    // Marshal the funding outpoint out. `Txid: AsRef<[u8]>`, matching the
    // conversion convention used across this crate's changeset FFI.
    let mut txid = [0u8; 32];
    txid.copy_from_slice(invitation.out_point.txid.as_ref());
    unsafe {
        *out_outpoint = OutPointFFI {
            txid,
            vout: invitation.out_point.vout,
        };
    }

    // The URI is a secret (embeds the voucher key). Do NOT log it — the error
    // path below only reports the fixed interior-NUL message, never the URI.
    let c_uri = match std::ffi::CString::new(invitation.uri) {
        Ok(c) => c,
        Err(_) => {
            return PlatformWalletFFIResult::from(
                "invitation URI contained an interior NUL".to_string(),
            )
        }
    };
    unsafe {
        *out_uri = c_uri.into_raw();
    }
    PlatformWalletFFIResult::ok()
}

/// Claim a DashPay invitation: register a NEW identity for the invitee, funded
/// by the imported voucher carried in `uri`.
///
/// `uri` is the `dashpay://invite?data=…` link; it is parsed into a
/// `ParsedInvitation` and validated (fail-fast on a stale / wrong-type /
/// mismatched link) before any network act. `identity_pubkeys` are the
/// invitee's own new-identity keys (derived from the invitee's seed), signed by
/// `signer_handle` (the Platform-side per-identity-key signer). The asset-lock's
/// outer signature is produced from the imported raw voucher key, so **no
/// Core-side resolver signer is needed**. `now_unix` is the current unix time
/// used for the advisory-expiry check (passed in from Swift — the FFI can't read
/// the clock deterministically).
///
/// The contact-bootstrap ("establish contact with the sender?") is **not** done
/// here — the UI asks the invitee and, on confirm, calls the existing
/// contact-request path
/// ([`crate::dashpay::platform_wallet_send_contact_request_with_signer`]).
///
/// On success writes the new identity id to `*out_identity_id` and a handle into
/// `MANAGED_IDENTITY_STORAGE` to `*out_identity_handle` (release via
/// [`crate::managed_identity_destroy`]).
///
/// # Safety
/// - `uri` must be a valid NUL-terminated UTF-8 C string.
/// - `identity_pubkeys` must point to `identity_pubkeys_count` readable
///   `IdentityPubkeyFFI` rows (`count >= 1`).
/// - `signer_handle` must be a valid, non-destroyed `*mut SignerHandle` produced
///   by `dash_sdk_signer_create_with_ctx`. The caller retains ownership.
/// - `out_identity_id` must be a valid `*mut [u8; 32]`; `out_identity_handle` a
///   valid `*mut Handle`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_claim_invitation(
    wallet_handle: Handle,
    uri: *const c_char,
    identity_index: u32,
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
    signer_handle: *mut SignerHandle,
    now_unix: u32,
    out_identity_id: *mut [u8; 32],
    out_identity_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(uri);
    check_ptr!(signer_handle);
    check_ptr!(identity_pubkeys);
    check_ptr!(out_identity_id);
    check_ptr!(out_identity_handle);
    if identity_pubkeys_count == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "identity_pubkeys_count must be >= 1",
        );
    }

    let uri = unwrap_result_or_return!(unsafe { CStr::from_ptr(uri) }.to_str()).to_string();
    // Decode the off-chain envelope up front (pure, no network). Structural
    // validity (scheme, version, size caps, key/proof shape) is checked here;
    // the claimability checks (expiry, proof type, credit-output binding) run
    // inside `claim_invitation`.
    let invitation = unwrap_result_or_return!(parse_invitation_uri(&uri));
    let keys_map = match decode_identity_pubkeys(identity_pubkeys, identity_pubkeys_count) {
        Ok(m) => m,
        Err(e) => return e,
    };

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            // SAFETY: see the fn-level safety doc — the caller pins
            // `signer_handle` for the duration of this call.
            let identity_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
            identity_wallet
                .claim_invitation(
                    invitation,
                    identity_index,
                    keys_map,
                    identity_signer,
                    now_unix,
                    None,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let identity = unwrap_result_or_return!(result);
    let id_bytes: [u8; 32] = identity.id().to_buffer();
    unsafe {
        *out_identity_id = id_bytes;
    }
    let managed = platform_wallet::ManagedIdentity::new(identity, identity_index);
    let handle = MANAGED_IDENTITY_STORAGE.insert(managed);
    unsafe {
        *out_identity_handle = handle;
    }
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Marshalling-boundary coverage. The invitation crypto/codec semantics are
    // pinned library-side in `platform_wallet`'s `crypto::invitation` +
    // `network::invitation`; these tests only exercise the FFI's null/parameter
    // guards and the wallet-lookup miss path.

    /// A null `core_signer_handle` is rejected with `ErrorNullPointer` before
    /// any wallet lookup (the `check_ptr!` contract).
    #[test]
    fn create_invitation_null_core_signer_is_null_pointer() {
        let mut uri: *mut c_char = std::ptr::null_mut();
        let mut outpoint = OutPointFFI {
            txid: [0u8; 32],
            vout: 0,
        };
        let r = unsafe {
            platform_wallet_create_invitation(
                1,
                1000,
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                &mut uri,
                &mut outpoint,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// Opting into the contact-bootstrap (non-null `inviter_identity_id`)
    /// without a `inviter_username` is rejected with `ErrorInvalidParameter`.
    #[test]
    fn create_invitation_inviter_without_username_is_invalid_parameter() {
        let dummy_signer = std::ptr::dangling_mut::<MnemonicResolverHandle>();
        let inviter_id = [0xABu8; 32];
        let mut uri: *mut c_char = std::ptr::null_mut();
        let mut outpoint = OutPointFFI {
            txid: [0u8; 32],
            vout: 0,
        };
        let r = unsafe {
            platform_wallet_create_invitation(
                1,
                1000,
                0,
                inviter_id.as_ptr(),
                std::ptr::null(),
                1_700_000_000,
                dummy_signer,
                &mut uri,
                &mut outpoint,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
    }

    /// An unknown `wallet_handle` surfaces `NotFound` via the `with_item`
    /// lookup miss. A pure funding voucher (null inviter) gets past the inviter
    /// build; the dangling signer is never dereferenced (the lookup fails first).
    #[test]
    fn create_invitation_unknown_wallet_is_not_found() {
        let dummy_signer = std::ptr::dangling_mut::<MnemonicResolverHandle>();
        let mut uri: *mut c_char = std::ptr::null_mut();
        let mut outpoint = OutPointFFI {
            txid: [0u8; 32],
            vout: 0,
        };
        let r = unsafe {
            platform_wallet_create_invitation(
                0xDEAD_BEEF,
                1000,
                0,
                std::ptr::null(),
                std::ptr::null(),
                1_700_000_000,
                dummy_signer,
                &mut uri,
                &mut outpoint,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
    }

    /// A zero `now_unix` (a failed Swift clock read) is rejected with
    /// `ErrorInvalidParameter` before any wallet lookup — the derived expiry
    /// would otherwise be 1970-relative. Runs after the pointer checks, so the
    /// dangling signer is never dereferenced.
    #[test]
    fn create_invitation_zero_now_is_invalid_parameter() {
        let dummy_signer = std::ptr::dangling_mut::<MnemonicResolverHandle>();
        let mut uri: *mut c_char = std::ptr::null_mut();
        let mut outpoint = OutPointFFI {
            txid: [0u8; 32],
            vout: 0,
        };
        let r = unsafe {
            platform_wallet_create_invitation(
                1,
                1000,
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                dummy_signer,
                &mut uri,
                &mut outpoint,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
    }

    /// A null `uri` is rejected with `ErrorNullPointer` (the `check_ptr!`
    /// contract) before any parsing.
    #[test]
    fn claim_invitation_null_uri_is_null_pointer() {
        let dummy_signer = std::ptr::dangling_mut::<SignerHandle>();
        let dummy_pubkeys = std::ptr::dangling::<IdentityPubkeyFFI>();
        let mut id = [0u8; 32];
        let mut handle: Handle = 0;
        let r = unsafe {
            platform_wallet_claim_invitation(
                1,
                std::ptr::null(),
                0,
                dummy_pubkeys,
                1,
                dummy_signer,
                0,
                &mut id,
                &mut handle,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// A malformed `uri` (wrong scheme) fails the codec parse before any wallet
    /// lookup, surfacing the invalid-data error rather than a network attempt.
    #[test]
    fn claim_invitation_bad_uri_is_rejected() {
        let dummy_signer = std::ptr::dangling_mut::<SignerHandle>();
        let dummy_pubkeys = std::ptr::dangling::<IdentityPubkeyFFI>();
        let bad = std::ffi::CString::new("https://not-an-invite").unwrap();
        let mut id = [0u8; 32];
        let mut handle: Handle = 0;
        let r = unsafe {
            platform_wallet_claim_invitation(
                1,
                bad.as_ptr(),
                0,
                dummy_pubkeys,
                1,
                dummy_signer,
                0,
                &mut id,
                &mut handle,
            )
        };
        // parse_invitation_uri rejects the scheme → surfaced as an error result.
        assert_ne!(r.code, PlatformWalletFFIResultCode::Success);
    }
}

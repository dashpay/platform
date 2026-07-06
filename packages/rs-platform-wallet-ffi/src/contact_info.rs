//! FFI bindings for DashPay `contactInfo` (alias / note / hidden).
//!
//! One write entry point: set the metadata locally AND publish the
//! self-encrypted `contactInfo` document. The local state ALWAYS
//! lands in SwiftData via the persister; the document publish may be
//! deferred (DIP-15 ≥2-contacts privacy rule) or skipped (watch-only).
//! The `out_outcome` param reports which happened so the UI can tell
//! the user the truth instead of unconditionally claiming a sync.
//! Reads need no new FFI: decrypted values flow into the
//! established-contact changeset during the recurring sync.

use std::os::raw::c_char;

use platform_wallet::ContactInfoPublishOutcome;
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::dashpay::resolver_contact_crypto_provider;

use crate::check_ptr;
use crate::dashpay_profile::decode_opt_c_str;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Outcome discriminant written to `out_outcome` by
/// [`platform_wallet_set_dashpay_contact_info_with_signer`]. Mirrors
/// [`ContactInfoPublishOutcome`].
pub const CONTACT_INFO_PUBLISHED: u8 = 0;
pub const CONTACT_INFO_DEFERRED_UNTIL_TWO_CONTACTS: u8 = 1;
pub const CONTACT_INFO_SKIPPED_WATCH_ONLY: u8 = 2;

/// Set alias / note / hidden for an established contact and publish
/// the corresponding `contactInfo` document.
///
/// `alias` / `note` may be NULL (= clear the field). The signer is
/// the same vtable signer the profile write entry point takes.
/// `out_outcome` (if non-null) receives the publish outcome
/// discriminant (`CONTACT_INFO_*` above): local state is always
/// updated, but the cross-device document publish may have been
/// deferred or skipped.
///
/// `core_signer_handle` is the wallet-HD resolver signer (as for send/accept):
/// the contactInfo AES keys are derived through it, so no resident seed is
/// needed and watch-only / external-signable wallets publish too.
///
/// # Safety
/// `wallet_handle` must be a live wallet handle; `identity_id` and
/// `contact_id` must point at 32 readable bytes; `signer_handle`
/// must be a live `VTableSigner` and `core_signer_handle` a live
/// `*mut MnemonicResolverHandle` for the duration of the call;
/// `out_outcome` must be null or point at one writable byte.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_set_dashpay_contact_info_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    contact_id: *const u8,
    alias: *const c_char,
    note: *const c_char,
    display_hidden: bool,
    signer_handle: *mut SignerHandle,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_outcome: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(core_signer_handle);

    let identity = unwrap_result_or_return!(read_identifier(identity_id));
    let contact = unwrap_result_or_return!(read_identifier(contact_id));
    let alias = unwrap_result_or_return!(decode_opt_c_str(alias));
    let note = unwrap_result_or_return!(decode_opt_c_str(note));

    let signer_addr = signer_handle as usize;
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, move |wallet| {
        let identity_wallet = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: same lifetime contract as the drain/send FFI — the caller pins
        // both handles for the duration of this call.
        let provider = unsafe {
            resolver_contact_crypto_provider(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .dashpay()
                .set_contact_info_with_external_signer(
                    &identity,
                    &contact,
                    alias,
                    note,
                    display_hidden,
                    signer,
                    &provider,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let outcome = unwrap_result_or_return!(result);
    if !out_outcome.is_null() {
        *out_outcome = match outcome {
            ContactInfoPublishOutcome::Published => CONTACT_INFO_PUBLISHED,
            ContactInfoPublishOutcome::DeferredUntilTwoContacts => {
                CONTACT_INFO_DEFERRED_UNTIL_TWO_CONTACTS
            }
            ContactInfoPublishOutcome::SkippedWatchOnly => CONTACT_INFO_SKIPPED_WATCH_ONLY,
        };
    }
    PlatformWalletFFIResult::ok()
}

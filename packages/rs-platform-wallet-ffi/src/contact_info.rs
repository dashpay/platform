//! FFI bindings for DashPay `contactInfo` (alias / note / hidden —
//! M3 task 13).
//!
//! One write entry point: set the metadata locally AND publish the
//! self-encrypted `contactInfo` document (deferred under the DIP-15
//! ≥2-contacts privacy rule — the Rust side logs and skips the
//! network write; local state still lands in SwiftData via the
//! persister). Reads need no new FFI: the decrypted values flow into
//! the established-contact changeset during the recurring sync and
//! surface through the existing contact persistence.

use std::os::raw::c_char;

use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::dashpay_profile::decode_opt_c_str;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Set alias / note / hidden for an established contact and publish
/// the corresponding `contactInfo` document.
///
/// `alias` / `note` may be NULL (= clear the field). The signer is
/// the same vtable signer the profile write entry point takes.
///
/// # Safety
/// `wallet_handle` must be a live wallet handle; `identity_id` and
/// `contact_id` must point at 32 readable bytes; `signer_handle`
/// must be a live `VTableSigner` for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_set_dashpay_contact_info_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    contact_id: *const u8,
    alias: *const c_char,
    note: *const c_char,
    display_hidden: bool,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);

    let identity = unwrap_result_or_return!(read_identifier(identity_id));
    let contact = unwrap_result_or_return!(read_identifier(contact_id));
    let alias = unwrap_result_or_return!(decode_opt_c_str(alias));
    let note = unwrap_result_or_return!(decode_opt_c_str(note));

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, move |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .set_contact_info_with_external_signer(
                    &identity,
                    &contact,
                    alias,
                    note,
                    display_hidden,
                    signer,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

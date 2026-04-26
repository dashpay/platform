//! FFI bindings for identity → Core address withdrawal driven by an
//! external `SignerHandle`.
//!
//! Replaces the panic-prone `IdentitySigner` path on
//! [`IdentityWallet::withdraw_credits`](platform_wallet::IdentityWallet::withdraw_credits).
//! The withdrawal state-transition signature crosses the FFI through
//! the supplied `signer_handle` (typically the iOS-side `KeychainSigner`).

use std::ffi::CStr;
use std::os::raw::c_char;
use std::str::FromStr;

use dashcore::Address as DashAddress;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;

/// Withdraw `amount` credits from `identity_id` to a Dash address
/// (`to_address` — `Address::from_str`-parseable, e.g. P2PKH base58
/// like `"yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf"`) using the supplied
/// `signer_handle` for the identity-state-transition signature.
///
/// Wraps
/// [`IdentityWallet::withdraw_credits_with_external_signer`](platform_wallet::IdentityWallet::withdraw_credits_with_external_signer).
/// On success the identity's local balance on `ManagedIdentity` is
/// updated (the Rust side performs the credit-debit) and a snapshot
/// changeset is emitted via the persister so the Swift
/// `PersistentIdentity` row refreshes.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id` must point at a 32-byte buffer for the duration of
///   the call.
/// - `to_address` must be a NUL-terminated UTF-8 C-string for the
///   duration of the call.
/// - `signer_handle` must be a valid, non-destroyed handle produced by
///   `dash_sdk_signer_create_with_ctx`. Caller retains ownership.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_withdraw_credits_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    amount: u64,
    to_address: *const c_char,
    signer_handle: *mut SignerHandle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if signer_handle.is_null() || to_address.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "signer_handle or to_address is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let id = match read_identifier(identity_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid identity_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let to_address_str = match CStr::from_ptr(to_address).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    "to_address is not valid UTF-8",
                );
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };
    // Network-aware parse; `Address::from_str` returns an unchecked
    // address. `assume_checked` is correct here because Platform
    // accepts the address as-is and the Rust SDK pickle serializes it
    // back out the same way.
    let to_address_unchecked = match DashAddress::from_str(&to_address_str) {
        Ok(a) => a,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("Invalid Dash address: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };
    let to_address_parsed = to_address_unchecked.assume_checked();

    let signer_addr = signer_handle as usize;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity().clone();
            let result = block_on_worker(async move {
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
            });
            match result {
                Ok(()) => PlatformWalletFFIResult::Success,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("withdraw_credits_with_signer failed: {e}"),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidHandle,
                    "Invalid platform-wallet handle",
                );
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

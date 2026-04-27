//! Identity top-up driven by an external `Signer<PlatformAddress>`
//! handle.
//!
//! Mirrors [`crate::identity_registration_with_signer`] (registration)
//! but for an *existing* identity. The single entry point —
//! [`platform_wallet_top_up_from_addresses_with_signer`] — wraps
//! [`IdentityWallet::top_up_from_addresses`](platform_wallet::IdentityWallet::top_up_from_addresses)
//! and reuses the same address-input shape (`IdentityFundingInputFFI`)
//! the registration FFI exposes.
//!
//! Top-up state-transitions are signed entirely with the Platform
//! address inputs' private keys (the SDK uses `BalanceTransfer` to
//! credit the existing identity), so this FFI takes a single
//! `SignerHandle` — `signer_address_handle` — used as
//! `Signer<PlatformAddress>`. No identity-key signer is needed
//! (existing identity, no IdentityCreate to sign).
//!
//! On success the function writes the post-transition credit balance
//! back through `out_new_balance`. The local `ManagedIdentity`
//! manager is updated synchronously inside the library call (see
//! [`top_up_from_addresses`](platform_wallet::IdentityWallet::top_up_from_addresses)
//! for the bookkeeping); callers can re-read the balance via
//! `ManagedIdentity` once this returns.

use std::collections::BTreeMap;
use std::slice;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::Identifier;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::error::*;
use crate::handle::*;
use crate::identity_registration::IdentityFundingInputFFI;
use crate::runtime::block_on_worker;

/// Top up an existing identity's credit balance from one or more
/// Platform addresses, using an external `SignerHandle` for the
/// per-address funding signatures.
///
/// On success `out_new_balance` is populated with the post-transition
/// credit balance returned by Platform. The Rust-side
/// `ManagedIdentity` for `identity_id` has its balance updated and
/// queued for persistence inside the library call, so a subsequent
/// `ManagedIdentity::balance` read on the same handle reflects the
/// new value without an extra round trip.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id` must point at a 32-byte identity id buffer for
///   the duration of the call.
/// - `inputs` must point at a valid `[IdentityFundingInputFFI;
///   inputs_count]` array; each row's `hash` is a 20-byte address
///   hash.
/// - `signer_address_handle` must be a valid, non-destroyed
///   `*mut SignerHandle` produced by `dash_sdk_signer_create_with_ctx`.
///   The caller retains ownership; this function does NOT destroy it.
/// - `out_new_balance` must be writable for the duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_top_up_from_addresses_with_signer(
    wallet_handle: Handle,
    identity_id: *const [u8; 32],
    inputs: *const IdentityFundingInputFFI,
    inputs_count: usize,
    signer_address_handle: *mut SignerHandle,
    out_new_balance: *mut u64,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let invariant_violation: Option<&'static str> = if identity_id.is_null() {
        Some("`identity_id` pointer is null")
    } else if inputs.is_null() {
        Some("`inputs` pointer is null")
    } else if inputs_count == 0 {
        Some("`inputs_count` is zero")
    } else if signer_address_handle.is_null() {
        Some("`signer_address_handle` pointer is null")
    } else if out_new_balance.is_null() {
        Some("`out_new_balance` pointer is null")
    } else {
        None
    };
    if let Some(detail) = invariant_violation {
        if !out_error.is_null() {
            *out_error =
                PlatformWalletFFIError::new(PlatformWalletFFIResult::ErrorNullPointer, detail);
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let identity_id_bytes: [u8; 32] = *identity_id;
    let identity_id = Identifier::from_bytes(&identity_id_bytes).unwrap_or_default();

    let entries = slice::from_raw_parts(inputs, inputs_count);
    let mut input_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    for entry in entries {
        let address = match entry.address_type {
            0 => PlatformAddress::P2pkh(entry.hash),
            1 => PlatformAddress::P2sh(entry.hash),
            _ => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        "invalid address_type (expected 0 or 1)",
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };
        input_map.insert(address, entry.credits);
    }

    // Round-trip the signer pointer through `usize` so the spawned
    // future's capture is `Send + 'static` (the raw pointer is `!Send`,
    // but `usize` is). The underlying `Inner::Callback { ctx, vtable }`
    // is `Send + Sync` — see the unsafe impls in `rs-sdk-ffi/src/signer.rs`.
    let signer_addr = signer_address_handle as usize;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity().clone();

            let result = block_on_worker(async move {
                let address_signer: &VTableSigner =
                    unsafe { &*(signer_addr as *const VTableSigner) };

                identity_wallet
                    .top_up_from_addresses(&identity_id, input_map, address_signer, None)
                    .await
            });

            match result {
                Ok(new_balance) => {
                    *out_new_balance = new_balance;
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("top_up_from_addresses failed: {}", e),
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

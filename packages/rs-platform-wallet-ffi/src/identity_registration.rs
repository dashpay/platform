//! FFI for address-funded identity registration.
//!
//! Exposes `IdentityWallet::register_from_addresses` through a single C
//! entry point. Inputs arrive as a flat array of `(addressType, hash20,
//! nonce, credits)` tuples. The optional refund output is expressed as
//! a sibling triple plus an `has_output` flag.
//!
//! Returns the newly-created identity through two out-params:
//! * `out_identity_id` — the 32-byte platform identifier.
//! * `out_identity_handle` — a handle into `MANAGED_IDENTITY_STORAGE`
//!   pointing at a `platform_wallet::ManagedIdentity` wrapping the
//!   Identity + its HD `identity_index`. The caller owns the handle
//!   and must free it via `managed_identity_destroy` when done.

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::AddressNonce;
use std::collections::BTreeMap;
use std::slice;

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;

/// Flat input entry matching the SDK `put_with_address_funding`
/// shape. One row per contributing Platform Payment address.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityInputAddressFFI {
    /// Address type discriminant (0 = P2PKH, 1 = P2SH). Matches the
    /// encoding used by `PlatformAddressFFI`.
    pub address_type: u8,
    /// 20-byte address hash.
    pub hash: [u8; 20],
    /// Current anti-replay nonce for the address.
    pub nonce: u32,
    /// Credits to spend from this address for the new identity.
    pub credits: u64,
}

/// Optional refund / change output paired with the input list. When
/// `has_output` is false the remaining fields are ignored.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityOutputAddressFFI {
    pub has_output: bool,
    pub address_type: u8,
    pub hash: [u8; 20],
    pub credits: u64,
}

/// Register a new identity funded by Platform-address balances.
///
/// On success both `out_identity_id` (32 bytes) and
/// `out_identity_handle` are populated. The returned handle points at
/// a freshly-inserted `ManagedIdentity` in `MANAGED_IDENTITY_STORAGE`
/// wrapping the new identity together with `identity_index`. The
/// caller owns the handle and must release it via
/// `managed_identity_destroy`.
///
/// Note: the wallet's internal `IdentityManager` also receives a copy
/// of the same identity (via
/// `IdentityWallet::register_from_addresses`), so this handle is a
/// convenience for the caller to query right after creation — the
/// canonical source of truth remains the wallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_register_identity_from_addresses(
    wallet_handle: Handle,
    identity_index: u32,
    key_count: u32,
    inputs: *const IdentityInputAddressFFI,
    inputs_count: usize,
    // Pointer rather than by-value because a 32-byte C struct
    // straddles the "passed by register vs. by stack" boundary
    // differently across toolchains. Swift + Rust agree on
    // pointer ABI, so we dodge that question entirely.
    output: *const IdentityOutputAddressFFI,
    out_identity_id: *mut [u8; 32],
    out_identity_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    // Distinct messages per pointer so the caller can tell which
    // invariant was violated. Swift currently surfaces `.nullPointer`
    // generically; the detail here makes the alert actionable.
    let invariant_violation: Option<&'static str> = if inputs.is_null() {
        Some("`inputs` pointer is null")
    } else if inputs_count == 0 {
        Some("`inputs_count` is zero")
    } else if out_identity_id.is_null() {
        Some("`out_identity_id` pointer is null")
    } else if out_identity_handle.is_null() {
        Some("`out_identity_handle` pointer is null")
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

    // Decode the inputs array into the SDK's expected map shape.
    let entries = slice::from_raw_parts(inputs, inputs_count);
    let mut input_map: BTreeMap<PlatformAddress, (AddressNonce, Credits)> = BTreeMap::new();
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
        input_map.insert(address, (entry.nonce, entry.credits));
    }

    // `output` is allowed to be null — it means "no refund, any
    // residual credits stay with the new identity". Swift always
    // passes a real pointer, but we keep the null branch so the
    // ABI is forgiving to other callers.
    let output_map = if output.is_null() {
        None
    } else {
        let output_ref = &*output;
        if output_ref.has_output {
            let address = match output_ref.address_type {
                0 => PlatformAddress::P2pkh(output_ref.hash),
                1 => PlatformAddress::P2sh(output_ref.hash),
                _ => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidParameter,
                            "invalid output address_type (expected 0 or 1)",
                        );
                    }
                    return PlatformWalletFFIResult::ErrorInvalidParameter;
                }
            };
            Some((address, output_ref.credits))
        } else {
            None
        }
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity();
            let address_wallet = wallet.platform();
            let result = runtime().block_on(identity_wallet.register_from_addresses(
                input_map,
                output_map,
                identity_index,
                key_count,
                address_wallet,
                None,
            ));

            match result {
                Ok(identity) => {
                    let id_bytes: [u8; 32] = identity.id().to_buffer();
                    *out_identity_id = id_bytes;

                    // Wrap the new identity in a ManagedIdentity and
                    // hand the caller a handle into the global FFI
                    // storage. The wallet's IdentityManager keeps its
                    // own copy — this is a convenience reference for
                    // the caller that avoids a follow-up lookup.
                    let managed = platform_wallet::ManagedIdentity::new(identity, identity_index);
                    let handle = MANAGED_IDENTITY_STORAGE.insert(managed);
                    *out_identity_handle = handle;
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("register_from_addresses failed: {}", e),
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

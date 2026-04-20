//! FFI for address-funded identity registration.
//!
//! Exposes `IdentityWallet::register_from_addresses` through a single C
//! entry point. Inputs arrive as a flat array of `(addressType, hash20,
//! nonce, credits)` tuples. The optional refund output is expressed as
//! a sibling triple plus an `has_output` flag.
//!
//! Returns the newly-created identity as platform-serialized bytes
//! that the caller frees via [`free_identity_bytes`].

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::AddressNonce;
use dpp::serialization::PlatformSerializable;
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
/// On success `out_identity_bytes` is set to a heap-allocated
/// `PlatformVersion`-serialized `Identity`. The caller takes
/// ownership and must free with [`free_identity_bytes`].
/// `out_identity_id` receives the 32-byte identity id.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_register_identity_from_addresses(
    wallet_handle: Handle,
    identity_index: u32,
    key_count: u32,
    inputs: *const IdentityInputAddressFFI,
    inputs_count: usize,
    output: IdentityOutputAddressFFI,
    out_identity_id: *mut [u8; 32],
    out_identity_bytes: *mut *mut u8,
    out_identity_bytes_len: *mut usize,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if inputs.is_null()
        || inputs_count == 0
        || out_identity_id.is_null()
        || out_identity_bytes.is_null()
        || out_identity_bytes_len.is_null()
    {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "null pointer or empty inputs",
            );
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

    let output_map = if output.has_output {
        let address = match output.address_type {
            0 => PlatformAddress::P2pkh(output.hash),
            1 => PlatformAddress::P2sh(output.hash),
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
        Some((address, output.credits))
    } else {
        None
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

                    let bytes = match identity.serialize_to_bytes() {
                        Ok(b) => b,
                        Err(e) => {
                            if !out_error.is_null() {
                                *out_error = PlatformWalletFFIError::new(
                                    PlatformWalletFFIResult::ErrorSerialization,
                                    format!("failed to serialize identity: {}", e),
                                );
                            }
                            return PlatformWalletFFIResult::ErrorSerialization;
                        }
                    };
                    let len = bytes.len();
                    let boxed = bytes.into_boxed_slice();
                    *out_identity_bytes_len = len;
                    *out_identity_bytes = Box::into_raw(boxed) as *mut u8;
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

/// Free the identity-bytes buffer returned by
/// [`platform_wallet_register_identity_from_addresses`].
#[no_mangle]
pub unsafe extern "C" fn free_identity_bytes(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(ptr, len);
    let _ = Box::from_raw(slice as *mut [u8]);
}

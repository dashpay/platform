//! FFI bindings for identity → identity credit transfer + identity →
//! platform-address credit transfer driven by an external `SignerHandle`.
//!
//! Replaces the panic-prone `IdentitySigner` path on
//! [`IdentityWallet::transfer_credits`](platform_wallet::IdentityWallet::transfer_credits).
//! Every identity-state-transition signature crosses the FFI through
//! the supplied `signer_handle` (typically the iOS-side `KeychainSigner`),
//! so the wallet's own seed never participates Rust-side. This unblocks
//! watch-only wallets and avoids the inner-lock deadlock the legacy path
//! hit when its derivation tried to `blocking_read` the wallet manager
//! from inside a Tokio worker.
//!
//! Entry points:
//! - [`platform_wallet_transfer_credits_with_signer`] — identity → identity
//!   transfer.
//! - [`platform_wallet_transfer_credits_to_addresses_with_signer`] —
//!   identity → platform addresses transfer (1+ recipients).

use std::collections::BTreeMap;
use std::slice;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;

/// One recipient of a credit transfer-to-addresses call.
///
/// Mirrors the platform-address-side `AddressBalanceEntryFFI` shape
/// but stripped down — only `address_type`, `hash`, and `credits` are
/// needed for the identity-side transfer (no nonce; the SDK fetches
/// it). `address_type` discriminates: `0 = P2PKH`, `1 = P2SH`.
#[repr(C)]
pub struct PlatformAddressCreditOutputFFI {
    pub address_type: u8,
    pub hash: [u8; 20],
    pub credits: u64,
}

/// Transfer `amount` credits from `from_identity_id` to
/// `to_identity_id` using the supplied `signer_handle` for the
/// identity-state-transition signature.
///
/// Wraps
/// [`IdentityWallet::transfer_credits_with_external_signer`](platform_wallet::IdentityWallet::transfer_credits_with_external_signer).
/// On success the sender's local balance on `ManagedIdentity` is
/// updated and a snapshot changeset is emitted via the persister so
/// the Swift `PersistentIdentity` row refreshes through the
/// `on_persist_identities_fn` callback.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `from_identity_id` / `to_identity_id` must each point at a 32-byte
///   buffer for the duration of the call.
/// - `signer_handle` must be a valid, non-destroyed handle produced by
///   `dash_sdk_signer_create_with_ctx` (typically `KeychainSigner.handle`).
///   Caller retains ownership; this function does NOT destroy it.
/// - `out_error` may be null only when the caller is willing to lose
///   the diagnostic message.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_transfer_credits_with_signer(
    wallet_handle: Handle,
    from_identity_id: *const u8,
    to_identity_id: *const u8,
    amount: u64,
    signer_handle: *mut SignerHandle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if signer_handle.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "signer_handle is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let from_id = match read_identifier(from_identity_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid from_identity_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
    let to_id = match read_identifier(to_identity_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid to_identity_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    // Round-trip the signer pointer through `usize` so the spawned
    // future has a `Send + 'static` capture (raw pointers are `!Send`,
    // but `usize` is). The `VTableSigner`'s `Inner::Callback { ctx,
    // vtable }` is `Send + Sync` (see the unsafe impls in
    // `rs-sdk-ffi/src/signer.rs`).
    let signer_addr = signer_handle as usize;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity().clone();
            let result = block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                identity_wallet
                    .transfer_credits_with_external_signer(&from_id, &to_id, amount, signer, None)
                    .await
            });
            match result {
                Ok(()) => PlatformWalletFFIResult::Success,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("transfer_credits_with_signer failed: {e}"),
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

/// Transfer credits from `from_identity_id` to a set of
/// [`PlatformAddressCreditOutputFFI`] recipients using the supplied
/// `signer_handle`.
///
/// Wraps
/// [`IdentityWallet::transfer_credits_to_addresses_with_external_signer`](platform_wallet::IdentityWallet::transfer_credits_to_addresses_with_external_signer).
///
/// `out_new_balance` (when non-null) receives the sender's remaining
/// balance after the transfer.
///
/// # Safety
/// Same null/lifetime rules as
/// [`platform_wallet_transfer_credits_with_signer`]. Additionally
/// `outputs` must point at a valid `[PlatformAddressCreditOutputFFI;
/// outputs_count]` array for the duration of the call (caller retains
/// ownership of the underlying buffers).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_transfer_credits_to_addresses_with_signer(
    wallet_handle: Handle,
    from_identity_id: *const u8,
    outputs: *const PlatformAddressCreditOutputFFI,
    outputs_count: usize,
    signer_handle: *mut SignerHandle,
    out_new_balance: *mut u64,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if signer_handle.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "signer_handle is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    if outputs.is_null() || outputs_count == 0 {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "outputs is null or empty",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let from_id = match read_identifier(from_identity_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid from_identity_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let entries = slice::from_raw_parts(outputs, outputs_count);
    let mut output_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
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
        output_map.insert(address, entry.credits);
    }

    let signer_addr = signer_handle as usize;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity().clone();
            let result = block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                identity_wallet
                    .transfer_credits_to_addresses_with_external_signer(
                        &from_id, output_map, signer, None,
                    )
                    .await
            });
            match result {
                Ok(new_balance) => {
                    if !out_new_balance.is_null() {
                        *out_new_balance = new_balance;
                    }
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("transfer_credits_to_addresses_with_signer failed: {e}"),
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

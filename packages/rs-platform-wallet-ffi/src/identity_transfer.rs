//! FFI bindings for identity → identity credit transfer + identity →
//! platform-address credit transfer driven by an external `SignerHandle`.
//!
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

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

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
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_transfer_credits_with_signer(
    wallet_handle: Handle,
    from_identity_id: *const u8,
    to_identity_id: *const u8,
    amount: u64,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);

    let from_id = unwrap_result_or_return!(read_identifier(from_identity_id));
    let to_id = unwrap_result_or_return!(read_identifier(to_identity_id));

    // Round-trip the signer pointer through `usize` so the spawned
    // future has a `Send + 'static` capture (raw pointers are `!Send`,
    // but `usize` is). The `VTableSigner`'s `Inner::Callback { ctx,
    // vtable }` is `Send + Sync` (see the unsafe impls in
    // `rs-sdk-ffi/src/signer.rs`).
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .transfer_credits_with_external_signer(&from_id, &to_id, amount, signer, None)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Transfer credits from `from_identity_id` to a set of
/// [`PlatformAddressCreditOutputFFI`] recipients using the supplied
/// `signer_handle`.
///
/// Wraps the composite
/// [`PlatformWallet::transfer_credits_to_addresses_with_external_signer`](platform_wallet::PlatformWallet::transfer_credits_to_addresses_with_external_signer),
/// which also reconciles wallet-owned recipient addresses' platform
/// balances from the transfer proof.
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
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(outputs);
    if outputs_count == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "`outputs_count` is zero",
        );
    }

    let from_id = unwrap_result_or_return!(read_identifier(from_identity_id));

    let entries = slice::from_raw_parts(outputs, outputs_count);
    let mut output_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    for entry in entries {
        let address = match entry.address_type {
            0 => PlatformAddress::P2pkh(entry.hash),
            1 => PlatformAddress::P2sh(entry.hash),
            _ => {
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    "invalid address_type (expected 0 or 1)",
                );
            }
        };
        output_map.insert(address, entry.credits);
    }

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wallet = wallet.clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            // The composite transfers the credits AND reconciles any
            // wallet-owned recipient addresses' platform balances from
            // the proof (third-party recipients are skipped).
            wallet
                .transfer_credits_to_addresses_with_external_signer(
                    &from_id, output_map, signer, None,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let new_balance = unwrap_result_or_return!(result);
    if !out_new_balance.is_null() {
        *out_new_balance = new_balance;
    }
    PlatformWalletFFIResult::ok()
}

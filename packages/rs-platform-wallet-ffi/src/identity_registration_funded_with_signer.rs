//! Asset-lock-funded identity registration driven by an external
//! `SignerHandle`.
//!
//! The single entry point —
//! [`platform_wallet_register_identity_with_funding_signer`] — wraps
//! [`IdentityWallet::register_identity_with_funding_external_signer`](platform_wallet::IdentityWallet::register_identity_with_funding_external_signer)
//! and works the same way the address-funded
//! `platform_wallet_register_identity_with_signer` does:
//!
//! - Caller pre-derives the new identity's authentication public keys
//!   via [`crate::dash_sdk_derive_identity_keys_from_mnemonic`] (which
//!   works for watch-only wallets, unlike the wallet-handle variant)
//!   and ships them in as a flat [`crate::IdentityPubkeyFFI`] array.
//! - Caller pre-persists each key's matching private material to
//!   whatever store the supplied `signer_handle` reads from (iOS
//!   Keychain in the typical case).
//! - The asset-lock proof + private key are still built on the Rust
//!   side from `amount_duffs` (the wallet must have spendable Core
//!   UTXOs — same precondition as the legacy variant).
//! - Every IdentityCreate state-transition signature crosses the FFI
//!   through `signer_handle`.
//!
//! The two-signer split that
//! `platform_wallet_register_identity_with_signer` (address-funded)
//! uses isn't needed here: the funding side is a Core-chain asset
//! lock signed with `asset_lock_private_key` (built Rust-side), not a
//! `Signer<PlatformAddress>` operation.

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::slice;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use platform_wallet::wallet::identity::types::funding::IdentityFundingMethod;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::IdentityPubkeyFFI;
use crate::runtime::block_on_worker;

/// Register a new asset-lock-funded identity using an external signer.
///
/// `amount_duffs` funds the asset lock from wallet UTXOs; the SDK
/// builds + broadcasts the asset-lock transaction internally and
/// retries with a ChainLock proof on InstantSend rejection (see
/// [`IdentityWallet::register_identity_with_funding_external_signer`](platform_wallet::IdentityWallet::register_identity_with_funding_external_signer)
/// for the IS->CL fallback details).
///
/// On success `out_identity_id` (32 bytes) and `out_identity_handle`
/// (a fresh `ManagedIdentity` handle in `MANAGED_IDENTITY_STORAGE`)
/// are populated.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_pubkeys` must point at a valid `[IdentityPubkeyFFI;
///   identity_pubkeys_count]` array, and each row's `pubkey_bytes`
///   must be a valid `[u8; pubkey_len]` buffer for the duration of
///   the call.
/// - `signer_handle` must be a valid, non-destroyed handle produced by
///   `dash_sdk_signer_create_with_ctx`. Caller retains ownership.
/// - `out_identity_id` and `out_identity_handle` must be writable for
///   the duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_register_identity_with_funding_signer(
    wallet_handle: Handle,
    amount_duffs: u64,
    identity_index: u32,
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
    signer_handle: *mut SignerHandle,
    out_identity_id: *mut [u8; 32],
    out_identity_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if signer_handle.is_null()
        || identity_pubkeys.is_null()
        || identity_pubkeys_count == 0
        || out_identity_id.is_null()
        || out_identity_handle.is_null()
    {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "signer_handle, identity_pubkeys, out_identity_id, or out_identity_handle is null/empty",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Materialize identity pubkeys into a BTreeMap<u32, IdentityPublicKey>
    // before entering the wallet-storage closure so a parse failure is
    // not gated on whether the wallet handle happens to be valid.
    let pubkey_rows: &[IdentityPubkeyFFI] =
        slice::from_raw_parts(identity_pubkeys, identity_pubkeys_count);
    let mut keys_map: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
    for (i, row) in pubkey_rows.iter().enumerate() {
        let key_type = match KeyType::try_from(row.key_type) {
            Ok(kt) => kt,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!(
                            "identity_pubkeys[{}].key_type = {} is not a valid KeyType",
                            i, row.key_type
                        ),
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };
        let purpose = match Purpose::try_from(row.purpose) {
            Ok(p) => p,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!(
                            "identity_pubkeys[{}].purpose = {} is not a valid Purpose",
                            i, row.purpose
                        ),
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };
        let security_level = match SecurityLevel::try_from(row.security_level) {
            Ok(sl) => sl,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!(
                            "identity_pubkeys[{}].security_level = {} is not a valid SecurityLevel",
                            i, row.security_level
                        ),
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };
        if row.pubkey_bytes.is_null() || row.pubkey_len == 0 {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    format!("identity_pubkeys[{}].pubkey_bytes is null or empty", i),
                );
            }
            return PlatformWalletFFIResult::ErrorNullPointer;
        }
        let pubkey_bytes: Vec<u8> =
            slice::from_raw_parts(row.pubkey_bytes, row.pubkey_len).to_vec();
        keys_map.insert(
            row.key_id,
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: row.key_id,
                purpose,
                security_level,
                contract_bounds: None,
                key_type,
                read_only: row.read_only,
                data: BinaryData::new(pubkey_bytes),
                disabled_at: None,
            }),
        );
    }

    let signer_addr = signer_handle as usize;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity().clone();

            let result = block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                identity_wallet
                    .register_identity_with_funding_external_signer(
                        IdentityFundingMethod::FundWithWallet { amount_duffs },
                        identity_index,
                        keys_map,
                        signer,
                        None,
                    )
                    .await
            });

            match result {
                Ok(identity) => {
                    let id_bytes: [u8; 32] = identity.id().to_buffer();
                    *out_identity_id = id_bytes;
                    let managed = platform_wallet::ManagedIdentity::new(identity, identity_index);
                    let handle = MANAGED_IDENTITY_STORAGE.insert(managed);
                    *out_identity_handle = handle;
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("register_identity_with_funding_signer failed: {}", e),
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

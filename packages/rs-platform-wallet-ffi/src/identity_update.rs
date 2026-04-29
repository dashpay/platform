//! FFI bindings for identity update (add / disable keys) driven by an
//! external `SignerHandle`.
//!
//! The MASTER auth key signs the `IdentityUpdateTransition` via the
//! supplied `signer_handle` (typically the iOS-side `KeychainSigner`).

use std::convert::TryFrom;
use std::slice;

use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::{decode_contract_bounds, IdentityPubkeyFFI};
use crate::runtime::block_on_worker;
use crate::types::*;

/// Update an identity by adding new public keys and/or disabling
/// existing key IDs, signing the resulting `IdentityUpdateTransition`
/// with the supplied `signer_handle`.
///
/// The new keys are passed in as flat [`IdentityPubkeyFFI`] rows
/// (mirroring the registration FFI). Caller is responsible for
/// pre-persisting each new key's private material to whatever store
/// the signer reads from (iOS Keychain in the typical case) so the
/// signer can later sign with the newly-added keys; the signer here
/// only signs the update transition itself with an existing MASTER
/// key.
///
/// Wraps
/// [`IdentityWallet::update_identity_with_external_signer`](platform_wallet::IdentityWallet::update_identity_with_external_signer).
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id` must point at a 32-byte buffer for the duration of
///   the call.
/// - `add_public_keys` must point at a valid `[IdentityPubkeyFFI;
///   add_public_keys_count]` array, and each row's `pubkey_bytes`
///   must be a valid `[u8; pubkey_len]` buffer for the duration of
///   the call. Either `(null, 0)` if no keys are being added.
/// - `disable_public_key_ids` must point at a valid
///   `[u32; disable_public_key_ids_count]` array. Either `(null, 0)`
///   if no keys are being disabled.
/// - `signer_handle` must be a valid, non-destroyed handle produced by
///   `dash_sdk_signer_create_with_ctx`. Caller retains ownership.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_update_identity_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    add_public_keys: *const IdentityPubkeyFFI,
    add_public_keys_count: usize,
    disable_public_key_ids: *const u32,
    disable_public_key_ids_count: usize,
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

    // Materialize add_public_keys into Vec<IdentityPublicKey> before
    // entering the wallet-storage closure so a parse failure is not
    // gated on whether the wallet handle happens to be valid.
    let add_keys: Vec<IdentityPublicKey> = if add_public_keys.is_null()
        || add_public_keys_count == 0
    {
        Vec::new()
    } else {
        let rows: &[IdentityPubkeyFFI] =
            slice::from_raw_parts(add_public_keys, add_public_keys_count);
        let mut keys: Vec<IdentityPublicKey> = Vec::with_capacity(rows.len());
        for (i, row) in rows.iter().enumerate() {
            let key_type = match KeyType::try_from(row.key_type) {
                Ok(kt) => kt,
                Err(_) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidParameter,
                            format!(
                                "add_public_keys[{}].key_type = {} is not a valid KeyType",
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
                                "add_public_keys[{}].purpose = {} is not a valid Purpose",
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
                                "add_public_keys[{}].security_level = {} is not a valid SecurityLevel",
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
                        format!("add_public_keys[{}].pubkey_bytes is null or empty", i),
                    );
                }
                return PlatformWalletFFIResult::ErrorNullPointer;
            }
            let pubkey_bytes: Vec<u8> =
                slice::from_raw_parts(row.pubkey_bytes, row.pubkey_len).to_vec();

            // Decode optional contract-bounds payload. `kind == 0`
            // means "no bounds" — Authentication / Transfer keys
            // and any caller that just doesn't set them. Encryption
            // / Decryption keys MUST carry a value here for Drive
            // to scope the key correctly; the helper rejects
            // unscoped rows for those purposes.
            let contract_bounds =
                match decode_contract_bounds(row, purpose, i, "add_public_keys", out_error) {
                    Ok(b) => b,
                    Err(code) => return code,
                };

            keys.push(IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: row.key_id,
                purpose,
                security_level,
                contract_bounds,
                key_type,
                read_only: row.read_only,
                data: BinaryData::new(pubkey_bytes),
                disabled_at: None,
            }));
        }
        keys
    };

    let disable_ids: Vec<u32> =
        if disable_public_key_ids.is_null() || disable_public_key_ids_count == 0 {
            Vec::new()
        } else {
            slice::from_raw_parts(disable_public_key_ids, disable_public_key_ids_count).to_vec()
        };

    let signer_addr = signer_handle as usize;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity().clone();
            let result = block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                identity_wallet
                    .update_identity_with_external_signer(&id, add_keys, disable_ids, signer, None)
                    .await
            });
            match result {
                Ok(()) => PlatformWalletFFIResult::Success,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("update_identity_with_signer failed: {e}"),
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

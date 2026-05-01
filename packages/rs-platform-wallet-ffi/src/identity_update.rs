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

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::{decode_contract_bounds, IdentityPubkeyFFI};
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Update an identity by adding new public keys and/or disabling
/// existing key IDs, signing the resulting `IdentityUpdateTransition`
/// with the supplied `signer_handle`.
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
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);

    let id = unwrap_result_or_return!(read_identifier(identity_id));

    let add_keys: Vec<IdentityPublicKey> =
        if add_public_keys.is_null() || add_public_keys_count == 0 {
            Vec::new()
        } else {
            let rows: &[IdentityPubkeyFFI] =
                slice::from_raw_parts(add_public_keys, add_public_keys_count);
            let mut keys: Vec<IdentityPublicKey> = Vec::with_capacity(rows.len());
            for (i, row) in rows.iter().enumerate() {
                let key_type = unwrap_result_or_return!(KeyType::try_from(row.key_type));
                let purpose = unwrap_result_or_return!(Purpose::try_from(row.purpose));
                let security_level =
                    unwrap_result_or_return!(SecurityLevel::try_from(row.security_level));
                if row.pubkey_bytes.is_null() || row.pubkey_len == 0 {
                    return PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorNullPointer,
                        format!("add_public_keys[{i}].pubkey_bytes is null or empty"),
                    );
                }
                let pubkey_bytes: Vec<u8> =
                    slice::from_raw_parts(row.pubkey_bytes, row.pubkey_len).to_vec();

                let contract_bounds = unwrap_result_or_return!(decode_contract_bounds(
                    row,
                    purpose,
                    i,
                    "add_public_keys"
                ));

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

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .update_identity_with_external_signer(&id, add_keys, disable_ids, signer, None)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

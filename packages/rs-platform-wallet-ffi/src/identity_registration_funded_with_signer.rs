//! Asset-lock-funded identity registration driven by an external
//! `SignerHandle` (for the per-identity-key signatures) and a
//! `MnemonicResolverHandle` (for the Core-side asset-lock signature).
//!
//! Two signer surfaces are deliberately distinct:
//!
//! - `signer_handle` (a `*mut rs_sdk_ffi::SignerHandle`) is the
//!   Platform-side per-identity-key signer. It produces the `BLS` or
//!   `ECDSA` signatures over the IdentityCreate transition's
//!   per-public-key witnesses.
//! - `core_signer_handle` (a `*mut MnemonicResolverHandle`) is the
//!   Core-side ECDSA signer used for the asset-lock's outer
//!   state-transition signature. It reuses the existing
//!   Keychain-resolver vtable so the credit-output private key never
//!   crosses the FFI boundary as raw bytes — see
//!   [`crate::mnemonic_resolver_core_signer::MnemonicResolverCoreSigner`].

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::slice;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use platform_wallet::wallet::identity::types::funding::IdentityFunding;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::derive_and_persist_callbacks::MnemonicResolverHandle;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::IdentityPubkeyFFI;
use crate::mnemonic_resolver_core_signer::MnemonicResolverCoreSigner;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Register a new asset-lock-funded identity using an external signer.
///
/// # Safety
/// - `signer_handle` must be a valid, non-destroyed `*mut SignerHandle`
///   produced by `dash_sdk_signer_create_with_ctx`. The caller retains
///   ownership.
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle` produced by
///   [`crate::dash_sdk_mnemonic_resolver_create`]. The caller retains
///   ownership.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_register_identity_with_funding_signer(
    wallet_handle: Handle,
    amount_duffs: u64,
    identity_index: u32,
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
    signer_handle: *mut SignerHandle,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_identity_id: *mut [u8; 32],
    out_identity_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(core_signer_handle);
    check_ptr!(identity_pubkeys);
    check_ptr!(out_identity_id);
    check_ptr!(out_identity_handle);
    if identity_pubkeys_count == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "identity_pubkeys_count must be >= 1",
        );
    }

    let pubkey_rows: &[IdentityPubkeyFFI] =
        slice::from_raw_parts(identity_pubkeys, identity_pubkeys_count);
    let mut keys_map: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
    for (i, row) in pubkey_rows.iter().enumerate() {
        let key_type = unwrap_result_or_return!(KeyType::try_from(row.key_type));
        let purpose = unwrap_result_or_return!(Purpose::try_from(row.purpose));
        let security_level = unwrap_result_or_return!(SecurityLevel::try_from(row.security_level));
        if row.pubkey_bytes.is_null() || row.pubkey_len == 0 {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorNullPointer,
                format!("identity_pubkeys[{i}].pubkey_bytes is null or empty"),
            );
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

    // Round-trip both handles through `usize` so the spawned future's
    // capture is `Send + 'static` — same pattern used by the existing
    // address-signer FFI (raw pointers are `!Send`, `usize` isn't).
    let signer_addr = signer_handle as usize;
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        // Capture the network the asset-lock signer should derive
        // under. Pulled from the wallet itself rather than threaded
        // as an extra FFI parameter — it would be ambiguous if the
        // two disagreed.
        let network = wallet.sdk().network;
        block_on_worker(async move {
            // SAFETY: see the fn-level safety doc — both handles are
            // pinned alive for the duration of this FFI call.
            let identity_signer: &VTableSigner =
                unsafe { &*(signer_addr as *const VTableSigner) };
            let asset_lock_signer = unsafe {
                MnemonicResolverCoreSigner::new(
                    core_signer_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            identity_wallet
                .register_identity_with_funding(
                    IdentityFunding::FromWalletBalance { amount_duffs },
                    identity_index,
                    keys_map,
                    identity_signer,
                    &asset_lock_signer,
                    None,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let identity = unwrap_result_or_return!(result);
    let id_bytes: [u8; 32] = identity.id().to_buffer();
    *out_identity_id = id_bytes;
    let managed = platform_wallet::ManagedIdentity::new(identity, identity_index);
    let handle = MANAGED_IDENTITY_STORAGE.insert(managed);
    *out_identity_handle = handle;
    PlatformWalletFFIResult::ok()
}

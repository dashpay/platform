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
//!   [`rs_sdk_ffi::MnemonicResolverCoreSigner`].

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::slice;

use dashcore::hashes::Hash;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use platform_wallet::wallet::identity::types::funding::IdentityFunding;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use rs_sdk_ffi::MnemonicResolverHandle;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::{decode_contract_bounds, IdentityPubkeyFFI};
use rs_sdk_ffi::MnemonicResolverCoreSigner;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Decode the C-side `IdentityPubkeyFFI` rows into the
/// `BTreeMap<u32, IdentityPublicKey>` shape that
/// `IdentityWallet::register_identity_with_funding` expects.
///
/// Shared by both the [`platform_wallet_register_identity_with_funding_signer`]
/// (fresh asset-lock build) and
/// [`platform_wallet_resume_identity_with_existing_asset_lock_signer`]
/// (resume from tracked outpoint) entry points — the two differ only in
/// how they construct the [`IdentityFunding`] variant; the keys-map
/// shape is identical.
///
/// Returns `Err(PlatformWalletFFIResult)` carrying the FFI error the
/// caller should bubble up directly. Mirrors the inline `Err(...)` /
/// `unwrap_result_or_return!` flow elsewhere in this file.
unsafe fn decode_identity_pubkeys(
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
) -> Result<BTreeMap<u32, IdentityPublicKey>, PlatformWalletFFIResult> {
    let pubkey_rows: &[IdentityPubkeyFFI] =
        slice::from_raw_parts(identity_pubkeys, identity_pubkeys_count);
    let mut keys_map: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
    for (i, row) in pubkey_rows.iter().enumerate() {
        let key_type = KeyType::try_from(row.key_type).map_err(|e| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("identity_pubkeys[{i}].key_type invalid: {e}"),
            )
        })?;
        let purpose = Purpose::try_from(row.purpose).map_err(|e| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("identity_pubkeys[{i}].purpose invalid: {e}"),
            )
        })?;
        let security_level = SecurityLevel::try_from(row.security_level).map_err(|e| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("identity_pubkeys[{i}].security_level invalid: {e}"),
            )
        })?;
        if row.pubkey_bytes.is_null() || row.pubkey_len == 0 {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorNullPointer,
                format!("identity_pubkeys[{i}].pubkey_bytes is null or empty"),
            ));
        }
        let pubkey_bytes: Vec<u8> =
            slice::from_raw_parts(row.pubkey_bytes, row.pubkey_len).to_vec();
        // ContractBounds round-trip: decode the kind/id/document_type
        // tuple the Swift side marshalled, with the same enforcement
        // the signer-only registration path uses (Encryption /
        // Decryption purposes must carry bounds; kind 0 for those is
        // rejected with a clean FFI error rather than producing a key
        // Drive silently can't use).
        let contract_bounds = decode_contract_bounds(row, purpose, i, "identity_pubkeys")?;
        keys_map.insert(
            row.key_id,
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: row.key_id,
                purpose,
                security_level,
                contract_bounds,
                key_type,
                read_only: row.read_only,
                data: BinaryData::new(pubkey_bytes),
                disabled_at: None,
            }),
        );
    }
    Ok(keys_map)
}

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

    let keys_map = match decode_identity_pubkeys(identity_pubkeys, identity_pubkeys_count) {
        Ok(m) => m,
        Err(e) => return e,
    };

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

/// Resume identity registration from an already-tracked asset lock.
///
/// Sister to [`platform_wallet_register_identity_with_funding_signer`]:
/// instead of building a fresh asset-lock transaction from wallet
/// balance, this entry point picks up an existing tracked lock by
/// outpoint and drives it through whatever stages remain (broadcast,
/// IS/CL wait, Platform submission). Use case is crash recovery — a
/// prior registration attempt left the lock in storage at
/// `InstantSendLocked` / `ChainLocked` but the IdentityCreate transition
/// never completed (app killed, network error, dismissed flow), and the
/// user now wants to consume the lock from the
/// "Fund from unused Asset Lock" picker in `CreateIdentityView`.
///
/// The Rust side dispatches via [`IdentityFunding::FromExistingAssetLock`]
/// inside the same `register_identity_with_funding` helper used by the
/// wallet-balance path — the resume logic and IS→CL fallback live
/// there, not here. This FFI is a thin marshaler.
///
/// # Safety
/// - `out_point` must be a valid, non-null pointer to an
///   `OutPointFFI` (32-byte raw txid + u32 vout). The caller retains
///   ownership; the FFI does not free it.
/// - `signer_handle` must be a valid, non-destroyed `*mut SignerHandle`
///   produced by `dash_sdk_signer_create_with_ctx`. The caller retains
///   ownership.
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle` produced by
///   [`crate::dash_sdk_mnemonic_resolver_create`]. The caller retains
///   ownership.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_resume_identity_with_existing_asset_lock_signer(
    wallet_handle: Handle,
    out_point: *const OutPointFFI,
    identity_index: u32,
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
    signer_handle: *mut SignerHandle,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_identity_id: *mut [u8; 32],
    out_identity_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_point);
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

    // `OutPointFFI::txid` is `[u8; 32]` so the conversion is
    // infallible — `from_byte_array` consumes the array directly,
    // unlike `from_slice` which would defensively return a `Result`
    // for length checking we don't need here. Matches the
    // convention already used across `rs-drive-abci` /
    // `rs-platform-wallet-ffi/src/persistence.rs`.
    let out_point_ffi = *out_point;
    let resume_outpoint = dashcore::OutPoint {
        txid: dashcore::Txid::from_byte_array(out_point_ffi.txid),
        vout: out_point_ffi.vout,
    };

    let keys_map = match decode_identity_pubkeys(identity_pubkeys, identity_pubkeys_count) {
        Ok(m) => m,
        Err(e) => return e,
    };

    let signer_addr = signer_handle as usize;
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
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
                    IdentityFunding::FromExistingAssetLock {
                        out_point: resume_outpoint,
                    },
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

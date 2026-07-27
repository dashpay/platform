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

use dashcore::hashes::Hash;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use platform_wallet::AssetLockFunding;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::{decode_identity_pubkeys, IdentityPubkeyFFI};
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverCoreSigner;
use rs_sdk_ffi::MnemonicResolverHandle;

fn existing_asset_lock_funding(
    out_point: dashcore::OutPoint,
    consume_invitation_voucher: bool,
) -> AssetLockFunding {
    AssetLockFunding::FromExistingAssetLock {
        out_point,
        consume_invitation_voucher,
    }
}

/// Register a new asset-lock-funded identity using an external signer.
///
/// `account_index` selects which BIP44 *standard* account (by BIP44
/// account index) the asset-lock funding UTXOs are drawn from. Only
/// BIP44 standard accounts are supported today; the Swift UI is
/// expected to filter the funding picker accordingly (CoinJoin / BIP32
/// funding for new-identity registration is not yet wired through
/// `create_funded_asset_lock_proof`).
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
    account_index: u32,
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
            let identity_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
            let asset_lock_signer = unsafe {
                MnemonicResolverCoreSigner::new(
                    core_signer_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            identity_wallet
                .register_identity_with_funding(
                    AssetLockFunding::FromWalletBalance {
                        amount_duffs,
                        account_index,
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
/// The Rust side dispatches via [`AssetLockFunding::FromExistingAssetLock`]
/// inside the same `register_identity_with_funding` helper used by the
/// wallet-balance path — the resume logic and IS→CL fallback live
/// there, not here. This FFI is a thin marshaler.
///
/// `consume_invitation_voucher` is the explicit authorization to consume an
/// `IdentityInvitation`-typed lock (a DashPay bearer voucher whose key is
/// shared in the invitation link). Pass `false` for every generic resume
/// surface — the resolver then refuses invitation locks, so a shared voucher
/// can never be silently consumed into an unrelated local identity. Only the
/// invitation reclaim flow passes `true`.
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
    consume_invitation_voucher: bool,
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
            let identity_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
            let asset_lock_signer = unsafe {
                MnemonicResolverCoreSigner::new(
                    core_signer_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            identity_wallet
                .register_identity_with_funding(
                    existing_asset_lock_funding(resume_outpoint, consume_invitation_voucher),
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

/// Top up an EXISTING identity from an already-tracked Core asset lock.
///
/// The crash-recovery counterpart to
/// [`crate::platform_wallet_top_up_identity_with_funding_signer`] (which
/// builds a *new* lock from wallet balance): this consumes a lock that
/// already confirmed on Core but whose `IdentityTopUp` never reached
/// Platform (app killed / network drop between broadcast and submit),
/// completing the top-up against `identity_id` from the stored outpoint.
/// Sister to
/// [`platform_wallet_resume_identity_with_existing_asset_lock_signer`],
/// which resumes the lock as a NEW-identity registration instead. Also the
/// DashPay invitation "reclaim into an existing identity" path (see
/// `consume_invitation_voucher` below).
///
/// The `FromExistingAssetLock` resume + IS→CL fallback logic lives in
/// `top_up_identity_with_funding`; this FFI is a thin marshaler. No
/// per-identity-key signer is needed (a top-up creates no keys); only the
/// Core-side asset-lock signature, produced by the wallet's own resolver.
///
/// `consume_invitation_voucher` is the explicit authorization to consume an
/// `IdentityInvitation`-typed lock (a DashPay bearer voucher whose key is
/// shared in the invitation link) — the invitation reclaim flow passes
/// `true`; every generic top-up/crash-recovery surface must pass `false`
/// and is refused invitation locks by the funding resolver.
///
/// # Safety
/// - `out_point` must be a valid, non-null `*const OutPointFFI`; the caller
///   retains ownership.
/// - `identity_id` must point to 32 readable bytes.
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`; the caller retains ownership.
/// - `out_new_balance` must be a valid `*mut u64`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_topup_identity_with_existing_asset_lock_signer(
    wallet_handle: Handle,
    out_point: *const OutPointFFI,
    identity_id: *const [u8; 32],
    core_signer_handle: *mut MnemonicResolverHandle,
    consume_invitation_voucher: bool,
    out_new_balance: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_point);
    check_ptr!(identity_id);
    check_ptr!(core_signer_handle);
    check_ptr!(out_new_balance);
    // FFI-safe sentinel before any fallible work.
    *out_new_balance = 0;

    let out_point_ffi = *out_point;
    let reclaim_outpoint = dashcore::OutPoint {
        txid: dashcore::Txid::from_byte_array(out_point_ffi.txid),
        vout: out_point_ffi.vout,
    };
    let identity_id = Identifier::from(*identity_id);

    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.sdk().network;
        block_on_worker(async move {
            // SAFETY: see the fn-level safety doc — the handle is pinned alive
            // for the duration of this FFI call.
            let asset_lock_signer = unsafe {
                MnemonicResolverCoreSigner::new(
                    core_signer_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            identity_wallet
                .top_up_identity_with_funding(
                    &identity_id,
                    existing_asset_lock_funding(reclaim_outpoint, consume_invitation_voucher),
                    &asset_lock_signer,
                    None,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let new_balance = unwrap_result_or_return!(result);
    *out_new_balance = new_balance;
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod topup_existing_lock_guard_tests {
    use super::*;
    use crate::error::PlatformWalletFFIResultCode;

    /// Non-null but never-dereferenced core-signer pointer: the null guards
    /// under test return before the handle is used.
    fn dangling_core_signer() -> *mut MnemonicResolverHandle {
        std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr()
    }

    fn zero_out_point() -> OutPointFFI {
        OutPointFFI {
            txid: [0u8; 32],
            vout: 0,
        }
    }

    #[test]
    fn rejects_null_out_point() {
        let id = [0u8; 32];
        let mut balance = 0u64;
        let res = unsafe {
            platform_wallet_topup_identity_with_existing_asset_lock_signer(
                0,
                std::ptr::null(),
                &id,
                dangling_core_signer(),
                false,
                &mut balance,
            )
        };
        assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    #[test]
    fn rejects_null_identity_id() {
        let op = zero_out_point();
        let mut balance = 0u64;
        let res = unsafe {
            platform_wallet_topup_identity_with_existing_asset_lock_signer(
                0,
                &op,
                std::ptr::null(),
                dangling_core_signer(),
                false,
                &mut balance,
            )
        };
        assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    #[test]
    fn rejects_null_core_signer() {
        let op = zero_out_point();
        let id = [0u8; 32];
        let mut balance = 0u64;
        let res = unsafe {
            platform_wallet_topup_identity_with_existing_asset_lock_signer(
                0,
                &op,
                &id,
                std::ptr::null_mut(),
                false,
                &mut balance,
            )
        };
        assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    #[test]
    fn rejects_null_out_balance() {
        let op = zero_out_point();
        let id = [0u8; 32];
        let res = unsafe {
            platform_wallet_topup_identity_with_existing_asset_lock_signer(
                0,
                &op,
                &id,
                dangling_core_signer(),
                false,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    #[test]
    fn topup_reclaim_forwards_explicit_invitation_authority() {
        let out_point = dashcore::OutPoint::null();
        assert!(matches!(
            existing_asset_lock_funding(out_point, true),
            AssetLockFunding::FromExistingAssetLock {
                out_point: actual,
                consume_invitation_voucher: true,
            } if actual == out_point
        ));
    }
}

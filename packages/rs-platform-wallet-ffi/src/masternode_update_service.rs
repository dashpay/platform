//! FFI bindings for the masternode update-service (ProUpServTx / unban)
//! action — `platform_wallet::masternode::update_service`.
//!
//! Two entry points, mirroring the withdraw pair. Both fund the L1 fee from
//! `wallet_id`'s core funds (input signing goes through the host's mnemonic
//! resolver, like every wallet-key signing path); they differ only in where
//! the operator BLS key comes from:
//!
//! - [`platform_wallet_manager_masternode_update_service`][]: wallet-owned
//!   masternodes — the operator key is derived from the wallet's
//!   `ProviderOperatorKeys` account at `operator_key_index` (the index the
//!   masternode record's derive-and-compare join already resolved).
//! - [`platform_wallet_manager_tracked_masternode_update_service`][]: tracked
//!   masternodes — the operator key is the host-vaulted key text (64-char
//!   hex or 32-byte base64), parsed and matched exactly like
//!   `platform_wallet_manager_masternode_verify_key`.
//!
//! The action is revive-only: service values are copied from the live
//! masternode-list entry. `out_txid` (32 wire-order bytes) is written only
//! when the broadcast definitively succeeded; an ambiguous outcome returns
//! `ErrorTransactionBroadcastUnconfirmed` and the reserved inputs stay held
//! for the wallet's normal reconciliation — never retry the call on that
//! code.

use std::os::raw::c_char;
use std::sync::Arc;

use dashcore::hashes::Hash;
use platform_wallet::masternode::{
    execute_masternode_update_service, parse_secret_for_role, prepare_masternode_update_service,
    LocatorSecret, MasternodeKeyRole, MasternodeUpdateServiceParams,
};
use platform_wallet::{PlatformWallet, ProviderKeyKind};
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};
use zeroize::Zeroizing;

use crate::core_wallet::FFICoreSignedTransaction;
use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_seed_from_resolver;
use crate::runtime::block_on_worker;
use crate::tracked_masternode::{invalid_handle, optional_string};
use crate::{check_ptr, unwrap_result_or_return};

/// Everything both externs snapshot from the manager before releasing the
/// handle-storage guard, so the network work runs unguarded.
pub(crate) struct ResolvedContext {
    pub(crate) wallet: Arc<PlatformWallet>,
    pub(crate) spv: Arc<platform_wallet::SpvRuntime>,
    pub(crate) network: dashcore::Network,
}

pub(crate) unsafe fn resolve_context(
    manager_handle: Handle,
    wallet_id: *const u8,
) -> Result<ResolvedContext, PlatformWalletFFIResult> {
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let resolved = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        (
            manager.get_wallet_blocking(&wid),
            manager.spv_arc(),
            manager.sdk().network,
        )
    });
    match resolved {
        None => Err(invalid_handle()),
        Some((None, _, _)) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "wallet not found in the manager",
        )),
        Some((Some(wallet), spv, network)) => Ok(ResolvedContext {
            wallet,
            spv,
            network,
        }),
    }
}

/// Derive a wallet provider secret (32-byte scalar — big-endian BLS for
/// operator keys, raw secp256k1 for owner keys) at `index`, resolving the
/// raw BIP39 seed through the mnemonic resolver when the wallet has no
/// resident keys — the same three phases as
/// `platform_wallet_provider_key_at_index`, with the resolver never invoked
/// under a wallet guard. Shared with the registrar-update module.
pub(crate) unsafe fn wallet_provider_secret(
    wallet: &Arc<PlatformWallet>,
    kind: ProviderKeyKind,
    index: u32,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
) -> Result<Zeroizing<[u8; 32]>, PlatformWalletFFIResult> {
    // Phase 1 — capability probe under a SHORT read guard, dropped before
    // any resolver interaction.
    let is_resident = {
        let wm = wallet.wallet_manager().blocking_read();
        match wm.get_wallet(&wallet.wallet_id()) {
            Some(key_wallet) => !key_wallet.is_external_signable() && !key_wallet.is_watch_only(),
            None => {
                return Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidHandle,
                    "wallet not found in wallet manager",
                ));
            }
        }
    };

    // Phase 2 — resolve the raw BIP39 seed for external-signable /
    // watch-only wallets. The resolver synchronously re-enters Swift and
    // reads the iOS Keychain, so never under a wallet guard.
    let mut seed_opt: Option<Zeroizing<[u8; 64]>> = None;
    if !is_resident {
        if mnemonic_resolver_handle.is_null() {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "this wallet has no resident private keys (external-signable / watch-only); \
                 a mnemonic resolver handle is required to derive the provider key",
            ));
        }
        let wallet_id = wallet.wallet_id();
        seed_opt = Some(resolve_seed_from_resolver(
            mnemonic_resolver_handle,
            &wallet_id,
        )?);
    }

    // Phase 3 — library derive; the resolver, if any, has already run.
    let derived = wallet
        .derive_provider_key_at_index(kind, index, seed_opt.as_deref().map(|s| &s[..]), true)
        .map_err(PlatformWalletFFIResult::from)?;
    let private = derived.private_key.ok_or_else(|| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "the wallet did not return the provider private key",
        )
    })?;
    // Copy straight into zeroizing storage — a plain `[u8; 32]` intermediate
    // is `Copy` and would leave an unscrubbed stack copy of the secret.
    if private.len() != 32 {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "the derived provider private key is not 32 bytes",
        ));
    }
    let mut bytes = Zeroizing::new([0u8; 32]);
    bytes.copy_from_slice(private.as_slice());
    Ok(bytes)
}

/// Parse a host-supplied operator key text (64-char hex or 32-byte base64)
/// into its BLS secret, shared by the tracked broadcast and prepare externs.
pub(crate) fn tracked_operator_secret(
    key_text: &str,
    network: dashcore::Network,
) -> Result<Zeroizing<[u8; 32]>, PlatformWalletFFIResult> {
    match parse_secret_for_role(key_text, MasternodeKeyRole::Operator, network) {
        // Move the existing zeroizing container; dereferencing it would
        // place a `Copy` of the secret on the stack.
        Ok(LocatorSecret::Bls(secret)) => Ok(secret),
        Ok(_) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "the operator key must be a BLS secret (64-char hex or 32-byte base64)",
        )),
        Err(e) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("operator key is not usable: {e}"),
        )),
    }
}

unsafe fn marshal_params(
    pro_tx_hash: *const u8,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    operator_payout_address: *const c_char,
) -> Result<MasternodeUpdateServiceParams, PlatformWalletFFIResult> {
    Ok(MasternodeUpdateServiceParams {
        pro_tx_hash: std::ptr::read(pro_tx_hash as *const [u8; 32]),
        platform_p2p_port: has_platform_p2p_port.then_some(platform_p2p_port),
        operator_payout_address: optional_string(operator_payout_address)?,
    })
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_update_service(
    context: ResolvedContext,
    pro_tx_hash: *const u8,
    operator_secret: Zeroizing<[u8; 32]>,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    operator_payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_txid: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    let params = match marshal_params(
        pro_tx_hash,
        has_platform_p2p_port,
        platform_p2p_port,
        operator_payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };

    let ResolvedContext {
        wallet,
        spv,
        network,
    } = context;
    let wallet_id_bytes = wallet.wallet_id();
    // Cross the Send boundary as usize; the handle is borrowed, never
    // destroyed — the calling thread blocks for the duration.
    let signer_addr = mnemonic_resolver_handle as usize;
    let txid = unwrap_result_or_return!(block_on_worker(async move {
        let signer = MnemonicResolverCoreSigner::new(
            signer_addr as *mut MnemonicResolverHandle,
            wallet_id_bytes,
            network,
        );
        execute_masternode_update_service(&wallet, &spv, params, operator_secret, &signer).await
    }));

    *out_txid = txid.to_raw_hash().to_byte_array();
    PlatformWalletFFIResult::ok()
}

/// Prepare-only sibling of [`run_update_service`]: identical up to the
/// broadcast, then registers the signed transaction — holding its input
/// reservation — as a core signed-transaction handle the host later
/// broadcasts (`core_wallet_broadcast_signed_transaction`), abandons
/// (`core_wallet_abandon_signed_transaction`) or frees (which abandons).
#[allow(clippy::too_many_arguments)]
unsafe fn run_prepare_update_service(
    context: ResolvedContext,
    pro_tx_hash: *const u8,
    operator_secret: Zeroizing<[u8; 32]>,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    operator_payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_transaction_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    let params = match marshal_params(
        pro_tx_hash,
        has_platform_p2p_port,
        platform_p2p_port,
        operator_payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };

    let ResolvedContext {
        wallet,
        spv,
        network,
    } = context;
    let wallet_id_bytes = wallet.wallet_id();
    let signer_addr = mnemonic_resolver_handle as usize;
    let (wallet, prepared) = unwrap_result_or_return!(block_on_worker(async move {
        let signer = MnemonicResolverCoreSigner::new(
            signer_addr as *mut MnemonicResolverHandle,
            wallet_id_bytes,
            network,
        );
        prepare_masternode_update_service(&wallet, &spv, params, operator_secret, &signer)
            .await
            .map(|prepared| (wallet, prepared))
    }));

    *out_transaction_handle = CORE_SIGNED_TRANSACTION_STORAGE.insert(FFICoreSignedTransaction {
        wallet: wallet.core().clone(),
        transaction: prepared,
    });
    PlatformWalletFFIResult::ok()
}

/// Broadcast a ProUpServTx re-asserting a wallet-owned masternode's current
/// service values — which revives it if it is PoSe-banned — signed with the
/// wallet's operator key at `operator_key_index` (the index the masternode
/// record's `operator_key_index` join field reports).
///
/// - `wallet_id` / `pro_tx_hash` — 32 bytes each; `pro_tx_hash` in WIRE
///   order, as the masternode list reports it.
/// - `platform_p2p_port` (honoured when `has_platform_p2p_port`) — required
///   for an evonode, forbidden otherwise; the masternode list does not
///   carry it.
/// - `operator_payout_address` — nullable. Must be null when the ProRegTx's
///   `operatorReward` is 0, and must be given when it is not (the payload
///   REPLACES the payout script on-chain; an empty one would clear it).
/// - `out_txid` — 32 wire-order bytes, written on definitive success.
///
/// On `ErrorTransactionBroadcastUnconfirmed` the outcome is ambiguous: the
/// reserved inputs stay held and the wallet reconciles through sync — do
/// not retry.
///
/// # Safety
/// Pointer args must be valid for the stated sizes; `mnemonic_resolver_handle`
/// must come from `dash_sdk_mnemonic_resolver_create` and remain valid for
/// the duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_masternode_update_service(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_index: u32,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    operator_payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_txid: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    // `out_txid` first: the zero-on-every-path contract must hold even
    // when a later required pointer is null.
    check_ptr!(out_txid);
    *out_txid = [0u8; 32];
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(mnemonic_resolver_handle);

    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let operator_secret = match wallet_provider_secret(
        &context.wallet,
        ProviderKeyKind::Operator,
        operator_key_index,
        mnemonic_resolver_handle,
    ) {
        Ok(secret) => secret,
        Err(e) => return e,
    };
    run_update_service(
        context,
        pro_tx_hash,
        operator_secret,
        has_platform_p2p_port,
        platform_p2p_port,
        operator_payout_address,
        mnemonic_resolver_handle,
        out_txid,
    )
}

/// Broadcast a ProUpServTx re-asserting a masternode's current service
/// values — which revives it if it is PoSe-banned — signed with a
/// host-supplied operator key (the tracked-masternode vault's key text:
/// 64-char hex or 32-byte base64). The L1 fee is still funded from
/// `wallet_id`'s core funds through the mnemonic resolver.
///
/// Parameters and outcome semantics are identical to
/// [`platform_wallet_manager_masternode_update_service`], with
/// `operator_key_text` replacing `operator_key_index`. The key is verified
/// against the masternode-list entry's operator public key (basic or legacy
/// serialization) before any network work.
///
/// # Safety
/// Pointer args must be valid for the stated sizes; `operator_key_text`
/// must be a NUL-terminated UTF-8 string; `mnemonic_resolver_handle` must
/// come from `dash_sdk_mnemonic_resolver_create` and remain valid for the
/// duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_tracked_masternode_update_service(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_text: *const c_char,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    operator_payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_txid: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    // `out_txid` first: the zero-on-every-path contract must hold even
    // when a later required pointer is null.
    check_ptr!(out_txid);
    *out_txid = [0u8; 32];
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(operator_key_text);
    check_ptr!(mnemonic_resolver_handle);

    let key_text = unwrap_result_or_return!(std::ffi::CStr::from_ptr(operator_key_text).to_str());

    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let secret = match tracked_operator_secret(key_text, context.network) {
        Ok(secret) => secret,
        Err(e) => return e,
    };
    run_update_service(
        context,
        pro_tx_hash,
        secret,
        has_platform_p2p_port,
        platform_p2p_port,
        operator_payout_address,
        mnemonic_resolver_handle,
        out_txid,
    )
}

/// Prepare — but do NOT broadcast — the ProUpServTx that
/// [`platform_wallet_manager_masternode_update_service`][] would send for a
/// wallet-owned masternode, so the host can show the transaction before the
/// user commits to it.
///
/// Same parameters and same preflights as the broadcasting entry point. On
/// success `out_transaction_handle` receives a core signed-transaction
/// handle whose inputs are RESERVED. The host must then either broadcast it
/// (`core_wallet_broadcast_signed_transaction`), abandon it
/// (`core_wallet_abandon_signed_transaction`), or free it
/// (`core_wallet_signed_transaction_free`, which abandons) — the fee and the
/// consensus-serialized bytes are readable meanwhile via
/// `core_wallet_signed_transaction_fee` / `core_wallet_signed_transaction_bytes`.
/// Dropping the handle without any of those strands the reservation until
/// the TTL backstop reclaims it.
///
/// # Safety
/// Pointer args must be valid for the stated sizes; `mnemonic_resolver_handle`
/// must come from `dash_sdk_mnemonic_resolver_create` and remain valid for
/// the duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_masternode_prepare_update_service(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_index: u32,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    operator_payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_transaction_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    // `out_transaction_handle` first: the zero-on-every-path contract must
    // hold even when a later required pointer is null.
    check_ptr!(out_transaction_handle);
    *out_transaction_handle = 0;
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(mnemonic_resolver_handle);

    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let operator_secret = match wallet_provider_secret(
        &context.wallet,
        ProviderKeyKind::Operator,
        operator_key_index,
        mnemonic_resolver_handle,
    ) {
        Ok(secret) => secret,
        Err(e) => return e,
    };
    run_prepare_update_service(
        context,
        pro_tx_hash,
        operator_secret,
        has_platform_p2p_port,
        platform_p2p_port,
        operator_payout_address,
        mnemonic_resolver_handle,
        out_transaction_handle,
    )
}

/// Prepare — but do NOT broadcast — the ProUpServTx that
/// [`platform_wallet_manager_tracked_masternode_update_service`][] would send,
/// signed with the host-vaulted operator key text.
///
/// Handle ownership and the broadcast / abandon / free contract are exactly
/// those of [`platform_wallet_manager_masternode_prepare_update_service`][].
///
/// # Safety
/// Pointer args must be valid for the stated sizes; `operator_key_text` must
/// be a NUL-terminated UTF-8 string; `mnemonic_resolver_handle` must come
/// from `dash_sdk_mnemonic_resolver_create` and remain valid for the
/// duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_tracked_masternode_prepare_update_service(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_text: *const c_char,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    operator_payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_transaction_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    // `out_transaction_handle` first: the zero-on-every-path contract must
    // hold even when a later required pointer is null.
    check_ptr!(out_transaction_handle);
    *out_transaction_handle = 0;
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(operator_key_text);
    check_ptr!(mnemonic_resolver_handle);

    let key_text = unwrap_result_or_return!(std::ffi::CStr::from_ptr(operator_key_text).to_str());
    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let secret = match tracked_operator_secret(key_text, context.network) {
        Ok(secret) => secret,
        Err(e) => return e,
    };
    run_prepare_update_service(
        context,
        pro_tx_hash,
        secret,
        has_platform_p2p_port,
        platform_p2p_port,
        operator_payout_address,
        mnemonic_resolver_handle,
        out_transaction_handle,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_wallet_ffi_result_free;

    /// Unknown manager handles must come back as `ErrorInvalidHandle` with
    /// the out-param still zeroed — mirroring the withdraw pair's contract.
    #[test]
    fn unknown_handles_are_invalid_handles() {
        unsafe {
            let wallet_id = [0u8; 32];
            let pro_tx_hash = [0u8; 32];
            let mut txid = [0xAAu8; 32];
            // A dangling-but-non-null resolver pointer is fine: the handle
            // lookup fails before the resolver is ever touched.
            let resolver = std::ptr::dangling_mut::<MnemonicResolverHandle>();

            let result = platform_wallet_manager_masternode_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                false,
                0,
                std::ptr::null(),
                resolver,
                &mut txid,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
            assert_eq!(txid, [0u8; 32], "out_txid is zeroed on every path");
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);

            let mut txid = [0xAAu8; 32];
            let key = std::ffi::CString::new("00").unwrap();
            let result = platform_wallet_manager_tracked_masternode_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                key.as_ptr(),
                false,
                0,
                std::ptr::null(),
                resolver,
                &mut txid,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
            assert_eq!(txid, [0u8; 32], "out_txid is zeroed on every path");
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);
        }
    }

    /// The prepare pair keeps the same contract: an unknown manager handle
    /// is an invalid-handle error and the out-param is left at the null
    /// handle, so a host can never broadcast a stale handle after a failure.
    #[test]
    fn prepare_unknown_handles_are_invalid_handles() {
        unsafe {
            let wallet_id = [0u8; 32];
            let pro_tx_hash = [0u8; 32];
            let resolver = std::ptr::dangling_mut::<MnemonicResolverHandle>();

            let mut transaction_handle: Handle = 7;
            let result = platform_wallet_manager_masternode_prepare_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                false,
                0,
                std::ptr::null(),
                resolver,
                &mut transaction_handle,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
            assert_eq!(transaction_handle, 0, "no handle is handed back on failure");
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);

            let mut transaction_handle: Handle = 7;
            let key = std::ffi::CString::new("00").unwrap();
            let result = platform_wallet_manager_tracked_masternode_prepare_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                key.as_ptr(),
                false,
                0,
                std::ptr::null(),
                resolver,
                &mut transaction_handle,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
            assert_eq!(transaction_handle, 0, "no handle is handed back on failure");
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);
        }
    }

    /// Null required pointers are rejected before anything else runs —
    /// and a valid `out_txid` is still zeroed first, per its contract.
    #[test]
    fn null_pointers_are_rejected() {
        unsafe {
            let wallet_id = [0u8; 32];
            let pro_tx_hash = [0u8; 32];
            let mut txid = [0xAAu8; 32];

            let result = platform_wallet_manager_masternode_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                false,
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
                &mut txid,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            assert_eq!(
                txid, [0u8; 32],
                "out_txid is zeroed before other pointer checks"
            );
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);

            let mut txid = [0xAAu8; 32];
            let result = platform_wallet_manager_tracked_masternode_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                std::ptr::null(),
                false,
                0,
                std::ptr::null(),
                std::ptr::dangling_mut::<MnemonicResolverHandle>(),
                &mut txid,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            assert_eq!(
                txid, [0u8; 32],
                "out_txid is zeroed before other pointer checks"
            );
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);
        }
    }
}

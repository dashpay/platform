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
    execute_masternode_update_service, parse_secret_for_role, LocatorSecret, MasternodeKeyRole,
    MasternodeUpdateServiceParams,
};
use platform_wallet::{PlatformWallet, ProviderKeyKind};
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};
use zeroize::Zeroizing;

use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_seed_from_resolver;
use crate::runtime::block_on_worker;
use crate::tracked_masternode::{invalid_handle, optional_string};
use crate::{check_ptr, unwrap_result_or_return};

/// Everything both externs snapshot from the manager before releasing the
/// handle-storage guard, so the network work runs unguarded.
struct ResolvedContext {
    wallet: Arc<PlatformWallet>,
    spv: Arc<platform_wallet::SpvRuntime>,
    network: dashcore::Network,
}

unsafe fn resolve_context(
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

/// Derive the wallet's operator BLS secret (big-endian scalar) at `index`,
/// resolving the raw BIP39 seed through the mnemonic resolver when the
/// wallet has no resident keys — the same three phases as
/// `platform_wallet_provider_key_at_index`, with the resolver never invoked
/// under a wallet guard.
unsafe fn wallet_operator_secret(
    wallet: &Arc<PlatformWallet>,
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
                 a mnemonic resolver handle is required to derive the operator key",
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
        .derive_provider_key_at_index(
            ProviderKeyKind::Operator,
            index,
            seed_opt.as_deref().map(|s| &s[..]),
            true,
        )
        .map_err(PlatformWalletFFIResult::from)?;
    let private = derived.private_key.ok_or_else(|| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "the wallet did not return the operator private key",
        )
    })?;
    // Copy straight into zeroizing storage — a plain `[u8; 32]` intermediate
    // is `Copy` and would leave an unscrubbed stack copy of the secret.
    if private.len() != 32 {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "the derived operator private key is not 32 bytes",
        ));
    }
    let mut bytes = Zeroizing::new([0u8; 32]);
    bytes.copy_from_slice(private.as_slice());
    Ok(bytes)
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
    let target: [u8; 32] = std::ptr::read(pro_tx_hash as *const [u8; 32]);
    let operator_payout_address = match optional_string(operator_payout_address) {
        Ok(text) => text,
        Err(e) => return e,
    };
    let params = MasternodeUpdateServiceParams {
        pro_tx_hash: target,
        platform_p2p_port: has_platform_p2p_port.then_some(platform_p2p_port),
        operator_payout_address,
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
    let operator_secret = match wallet_operator_secret(
        &context.wallet,
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
    let secret = match parse_secret_for_role(key_text, MasternodeKeyRole::Operator, context.network)
    {
        // Move the existing zeroizing container; dereferencing it would
        // place a `Copy` of the secret on the stack.
        Ok(LocatorSecret::Bls(secret)) => secret,
        Ok(_) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "the operator key must be a BLS secret (64-char hex or 32-byte base64)",
            );
        }
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("operator key is not usable: {e}"),
            );
        }
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

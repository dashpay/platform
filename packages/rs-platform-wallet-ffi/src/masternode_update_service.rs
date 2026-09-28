//! FFI bindings for the masternode update-service (ProUpServTx / unban)
//! action — `platform_wallet::masternode::update_service`.
//!
//! Two entry points, mirroring the withdraw pair, each with a prepare-only
//! twin. All fund the L1 fee from `wallet_id`'s core funds (input signing
//! goes through the host's mnemonic resolver, like every wallet-key signing
//! path); the pairs differ only in where the operator BLS key comes from:
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
//! The service values are an explicit input the host has the user confirm:
//! `service_address` (`"a.b.c.d:port"`) always, and for an evonode
//! `platform_node_id` (20 bytes) with both platform ports. The payload is
//! built from that input alone.
//! [`platform_wallet_manager_masternode_update_service_suggestion`] reads
//! what the synced masternode list shows, as a hint to prefill the form.
//!
//! `out_txid` (32 wire-order bytes) is written only when the broadcast
//! definitively succeeded; an ambiguous outcome returns
//! `ErrorTransactionBroadcastUnconfirmed` and the reserved inputs stay held
//! for the wallet's normal reconciliation. Never retry the call on that
//! code.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

use dashcore::hashes::Hash;
use platform_wallet::masternode::{
    execute_masternode_update_service, masternode_update_service_suggestion, parse_secret_for_role,
    parse_service_address, prepare_masternode_update_service, ConfirmedMasternodeService,
    LocatorSecret, MasternodeKeyRole, MasternodeServiceSuggestion, MasternodeUpdateServiceParams,
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

/// Parse a host-supplied operator key text (64-char hex or 32-byte base64)
/// into its BLS secret, shared by the tracked broadcast and prepare externs.
fn tracked_operator_secret(
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

/// Marshal the confirmed service values and the rest of the request.
/// `platform_node_id` selects the shape: null for a regular masternode
/// (whose platform ports must then be 0), 20 bytes for an evonode (whose
/// ports the library then requires).
unsafe fn marshal_params(
    pro_tx_hash: *const u8,
    service_address: *const c_char,
    platform_node_id: *const u8,
    platform_p2p_port: u16,
    platform_http_port: u16,
    operator_payout_address: *const c_char,
) -> Result<MasternodeUpdateServiceParams, PlatformWalletFFIResult> {
    let service_text = CStr::from_ptr(service_address).to_str()?;
    let service_address =
        parse_service_address(service_text).map_err(PlatformWalletFFIResult::from)?;
    let service = if platform_node_id.is_null() {
        if platform_p2p_port != 0 || platform_http_port != 0 {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "platform ports were given without a platform node id; they apply only to an \
                 evonode",
            ));
        }
        ConfirmedMasternodeService::Regular { service_address }
    } else {
        ConfirmedMasternodeService::Evonode {
            service_address,
            platform_node_id: std::ptr::read(platform_node_id as *const [u8; 20]),
            platform_p2p_port,
            platform_http_port,
        }
    };
    Ok(MasternodeUpdateServiceParams {
        pro_tx_hash: std::ptr::read(pro_tx_hash as *const [u8; 32]),
        service,
        operator_payout_address: optional_string(operator_payout_address)?,
    })
}

unsafe fn run_update_service(
    context: ResolvedContext,
    params: MasternodeUpdateServiceParams,
    operator_secret: Zeroizing<[u8; 32]>,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_txid: *mut [u8; 32],
) -> PlatformWalletFFIResult {
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
unsafe fn run_prepare_update_service(
    context: ResolvedContext,
    params: MasternodeUpdateServiceParams,
    operator_secret: Zeroizing<[u8; 32]>,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_transaction_handle: *mut Handle,
) -> PlatformWalletFFIResult {
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

/// Broadcast a ProUpServTx asserting a wallet-owned masternode's confirmed
/// service values, which also revives it if it is PoSe-banned, signed with
/// the wallet's operator key at `operator_key_index` (the index the
/// masternode record's `operator_key_index` join field reports).
///
/// - `wallet_id` / `pro_tx_hash`: 32 bytes each; `pro_tx_hash` in WIRE
///   order, as the masternode list reports it.
/// - `service_address`: required NUL-terminated `"a.b.c.d:port"`, the Core
///   P2P endpoint the user confirmed. The payload carries it, not the
///   masternode list's entry.
/// - `platform_node_id`: null for a regular masternode, or 20 bytes for an
///   evonode, which then also needs `platform_p2p_port` and
///   `platform_http_port`. A regular masternode passes 0 for both ports.
///   The shape must match the masternode's registration.
/// - `operator_payout_address`: nullable. Must be null when the ProRegTx's
///   `operatorReward` is 0, and must be given when it is not (the payload
///   REPLACES the payout script on-chain; an empty one would clear it).
/// - `out_txid`: 32 wire-order bytes, written on definitive success.
///
/// On `ErrorTransactionBroadcastUnconfirmed` the outcome is ambiguous: the
/// reserved inputs stay held and the wallet reconciles through sync. Do
/// not retry.
///
/// # Safety
/// Pointer args must be valid for the stated sizes; `service_address` must
/// be a NUL-terminated UTF-8 string; `mnemonic_resolver_handle` must come
/// from `dash_sdk_mnemonic_resolver_create` and remain valid for the
/// duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_masternode_update_service(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_index: u32,
    service_address: *const c_char,
    platform_node_id: *const u8,
    platform_p2p_port: u16,
    platform_http_port: u16,
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
    check_ptr!(service_address);
    check_ptr!(mnemonic_resolver_handle);

    // Marshal first so malformed input fails before the resolver is used.
    let params = match marshal_params(
        pro_tx_hash,
        service_address,
        platform_node_id,
        platform_p2p_port,
        platform_http_port,
        operator_payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };
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
        params,
        operator_secret,
        mnemonic_resolver_handle,
        out_txid,
    )
}

/// Broadcast a ProUpServTx asserting a masternode's confirmed service
/// values, which also revives it if it is PoSe-banned, signed with a
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
/// Pointer args must be valid for the stated sizes; `operator_key_text` and
/// `service_address` must be NUL-terminated UTF-8 strings;
/// `mnemonic_resolver_handle` must come from
/// `dash_sdk_mnemonic_resolver_create` and remain valid for the duration of
/// the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_tracked_masternode_update_service(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_text: *const c_char,
    service_address: *const c_char,
    platform_node_id: *const u8,
    platform_p2p_port: u16,
    platform_http_port: u16,
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
    check_ptr!(service_address);
    check_ptr!(mnemonic_resolver_handle);

    let key_text = unwrap_result_or_return!(CStr::from_ptr(operator_key_text).to_str());
    let params = match marshal_params(
        pro_tx_hash,
        service_address,
        platform_node_id,
        platform_p2p_port,
        platform_http_port,
        operator_payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };

    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let secret = match tracked_operator_secret(key_text, context.network) {
        Ok(secret) => secret,
        Err(e) => return e,
    };
    run_update_service(context, params, secret, mnemonic_resolver_handle, out_txid)
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
/// Pointer args must be valid for the stated sizes; `service_address` must
/// be a NUL-terminated UTF-8 string; `mnemonic_resolver_handle` must come
/// from `dash_sdk_mnemonic_resolver_create` and remain valid for the
/// duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_masternode_prepare_update_service(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_index: u32,
    service_address: *const c_char,
    platform_node_id: *const u8,
    platform_p2p_port: u16,
    platform_http_port: u16,
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
    check_ptr!(service_address);
    check_ptr!(mnemonic_resolver_handle);

    let params = match marshal_params(
        pro_tx_hash,
        service_address,
        platform_node_id,
        platform_p2p_port,
        platform_http_port,
        operator_payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };
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
    run_prepare_update_service(
        context,
        params,
        operator_secret,
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
/// Pointer args must be valid for the stated sizes; `operator_key_text` and
/// `service_address` must be NUL-terminated UTF-8 strings;
/// `mnemonic_resolver_handle` must come from
/// `dash_sdk_mnemonic_resolver_create` and remain valid for the duration of
/// the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_tracked_masternode_prepare_update_service(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_text: *const c_char,
    service_address: *const c_char,
    platform_node_id: *const u8,
    platform_p2p_port: u16,
    platform_http_port: u16,
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
    check_ptr!(service_address);
    check_ptr!(mnemonic_resolver_handle);

    let key_text = unwrap_result_or_return!(CStr::from_ptr(operator_key_text).to_str());
    let params = match marshal_params(
        pro_tx_hash,
        service_address,
        platform_node_id,
        platform_p2p_port,
        platform_http_port,
        operator_payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };
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
        params,
        secret,
        mnemonic_resolver_handle,
        out_transaction_handle,
    )
}

/// What the synced masternode list shows for a masternode's service, for
/// the host to prefill its update-service form. Returned by
/// [`platform_wallet_manager_masternode_update_service_suggestion`].
///
/// A hint only: the host shows these values and has the user confirm or
/// correct them before passing them to an update-service call. The list does not carry the
/// platform P2P port, so an evonode's always has to be entered.
#[repr(C)]
pub struct MasternodeServiceSuggestionFFI {
    /// `"ip:port"` of the Core P2P endpoint the list shows: a heap C string,
    /// or null when the entry has no plain IP:port (Tor / I2P / domain-only).
    /// Free with [`crate::platform_wallet_string_free`].
    pub service_address: *mut c_char,
    /// The list shows a high-performance (evonode) entry.
    pub is_evonode: bool,
    /// Tenderdash node id, gated by `has_platform_node_id` (evonodes only).
    pub platform_node_id: [u8; 20],
    pub has_platform_node_id: bool,
    /// Platform HTTP (DAPI) port, gated by `has_platform_http_port`
    /// (evonodes only).
    pub platform_http_port: u16,
    pub has_platform_http_port: bool,
    /// The list shows the masternode PoSe-banned. When it does not, the host
    /// may warn that the node looks healthy; updating a healthy node's
    /// service is legitimate, so nothing refuses on it.
    pub pose_banned: bool,
    /// The list shows v3 extended network info, which the update refuses (a
    /// version-2 payload would replace the whole endpoint map).
    pub has_extended_net_info: bool,
}

impl MasternodeServiceSuggestionFFI {
    fn empty() -> Self {
        Self {
            service_address: std::ptr::null_mut(),
            is_evonode: false,
            platform_node_id: [0u8; 20],
            has_platform_node_id: false,
            platform_http_port: 0,
            has_platform_http_port: false,
            pose_banned: false,
            has_extended_net_info: false,
        }
    }

    fn from_suggestion(suggestion: &MasternodeServiceSuggestion) -> Self {
        Self {
            service_address: suggestion
                .service_address
                .and_then(|address| CString::new(address.to_string()).ok())
                .map(CString::into_raw)
                .unwrap_or(std::ptr::null_mut()),
            is_evonode: suggestion.is_evonode,
            platform_node_id: suggestion.platform_node_id.unwrap_or([0u8; 20]),
            has_platform_node_id: suggestion.platform_node_id.is_some(),
            platform_http_port: suggestion.platform_http_port.unwrap_or(0),
            has_platform_http_port: suggestion.platform_http_port.is_some(),
            pose_banned: suggestion.pose_banned,
            has_extended_net_info: suggestion.has_extended_net_info,
        }
    }
}

/// Read what the synced masternode list shows for `pro_tx_hash` (32 WIRE
/// bytes) as a prefill suggestion for the update-service form. Read-only
/// and local.
///
/// Errors: `NotFound` when the list has no such masternode;
/// `ErrorMasternodeListUnavailable` before the list has synced. On any
/// error `out_suggestion` is left empty (null `service_address`).
///
/// # Safety
/// `pro_tx_hash` must point at 32 readable bytes; `out_suggestion` must be
/// writable. Free a non-null `service_address` with
/// [`crate::platform_wallet_string_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_masternode_update_service_suggestion(
    manager_handle: Handle,
    pro_tx_hash: *const u8,
    out_suggestion: *mut MasternodeServiceSuggestionFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_suggestion);
    std::ptr::write(out_suggestion, MasternodeServiceSuggestionFFI::empty());
    check_ptr!(pro_tx_hash);

    let target: [u8; 32] = std::ptr::read(pro_tx_hash as *const [u8; 32]);
    let Some(spv) =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| manager.spv_arc())
    else {
        return invalid_handle();
    };
    let suggestion = unwrap_result_or_return!(block_on_worker(async move {
        masternode_update_service_suggestion(&spv, &target).await
    }));
    let Some(suggestion) = suggestion else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "no masternode with this proTxHash on the masternode list",
        );
    };
    std::ptr::write(
        out_suggestion,
        MasternodeServiceSuggestionFFI::from_suggestion(&suggestion),
    );
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_wallet_ffi_result_free;
    use crate::platform_wallet_string_free;

    fn service(text: &str) -> CString {
        CString::new(text).expect("no interior NUL")
    }

    /// Unknown manager handles must come back as `ErrorInvalidHandle` with
    /// the out-param still zeroed, mirroring the withdraw pair's contract.
    #[test]
    fn should_report_unknown_manager_handles_as_invalid() {
        unsafe {
            let wallet_id = [0u8; 32];
            let pro_tx_hash = [0u8; 32];
            let address = service("203.0.113.7:19999");
            let mut txid = [0xAAu8; 32];
            // A dangling-but-non-null resolver pointer is fine: the handle
            // lookup fails before the resolver is ever touched.
            let resolver = std::ptr::dangling_mut::<MnemonicResolverHandle>();

            let result = platform_wallet_manager_masternode_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                address.as_ptr(),
                std::ptr::null(),
                0,
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
            let key = CString::new("00").unwrap();
            let result = platform_wallet_manager_tracked_masternode_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                key.as_ptr(),
                address.as_ptr(),
                std::ptr::null(),
                0,
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
    fn should_report_unknown_manager_handles_as_invalid_when_preparing() {
        unsafe {
            let wallet_id = [0u8; 32];
            let pro_tx_hash = [0u8; 32];
            let address = service("203.0.113.7:19999");
            let resolver = std::ptr::dangling_mut::<MnemonicResolverHandle>();

            let mut transaction_handle: Handle = 7;
            let result = platform_wallet_manager_masternode_prepare_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                address.as_ptr(),
                std::ptr::null(),
                0,
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
            let key = CString::new("00").unwrap();
            let result = platform_wallet_manager_tracked_masternode_prepare_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                key.as_ptr(),
                address.as_ptr(),
                std::ptr::null(),
                0,
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

    /// Null required pointers are rejected before anything else runs, and
    /// a valid `out_txid` is still zeroed first, per its contract. The
    /// service address is required.
    #[test]
    fn should_reject_null_pointers() {
        unsafe {
            let wallet_id = [0u8; 32];
            let pro_tx_hash = [0u8; 32];
            let address = service("203.0.113.7:19999");
            let mut txid = [0xAAu8; 32];

            let result = platform_wallet_manager_masternode_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                address.as_ptr(),
                std::ptr::null(),
                0,
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
                address.as_ptr(),
                std::ptr::null(),
                0,
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

            let mut transaction_handle: Handle = 7;
            let result = platform_wallet_manager_masternode_prepare_update_service(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                0,
                std::ptr::null(),
                std::ptr::dangling_mut::<MnemonicResolverHandle>(),
                &mut transaction_handle,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            assert_eq!(transaction_handle, 0, "the service address is required");
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);
        }
    }

    /// Malformed service input fails as an invalid parameter before the
    /// manager handle, the wallet or the resolver is touched.
    #[test]
    fn should_refuse_malformed_service_input_before_resolving_anything() {
        unsafe {
            let wallet_id = [0u8; 32];
            let pro_tx_hash = [0u8; 32];
            let resolver = std::ptr::dangling_mut::<MnemonicResolverHandle>();
            let node_id = [0x5Au8; 20];
            let cases: [(&str, *const u8, u16, u16); 3] = [
                ("203.0.113.7", std::ptr::null(), 0, 0),
                ("203.0.113.7:19999", std::ptr::null(), 36656, 0),
                ("not an address", node_id.as_ptr(), 36656, 1443),
            ];
            for (text, node_id, p2p, http) in cases {
                let address = service(text);
                let mut txid = [0xAAu8; 32];
                let result = platform_wallet_manager_masternode_update_service(
                    Handle::MAX,
                    wallet_id.as_ptr(),
                    pro_tx_hash.as_ptr(),
                    0,
                    address.as_ptr(),
                    node_id,
                    p2p,
                    http,
                    std::ptr::null(),
                    resolver,
                    &mut txid,
                );
                assert_eq!(
                    result.code,
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    "{text} with p2p {p2p} http {http}"
                );
                assert_eq!(txid, [0u8; 32]);
                let mut result = result;
                platform_wallet_ffi_result_free(&mut result);
            }
        }
    }

    /// A null node id is a regular masternode; 20 bytes are an evonode with
    /// the given platform ports.
    #[test]
    fn should_marshal_the_service_shape_from_the_platform_node_id() {
        unsafe {
            let pro_tx_hash = [0x11u8; 32];
            let address = service("203.0.113.7:19999");

            let regular = marshal_params(
                pro_tx_hash.as_ptr(),
                address.as_ptr(),
                std::ptr::null(),
                0,
                0,
                std::ptr::null(),
            )
            .unwrap_or_else(|_| panic!("a regular service marshals"));
            assert_eq!(regular.pro_tx_hash, pro_tx_hash);
            assert_eq!(
                regular.service,
                ConfirmedMasternodeService::Regular {
                    service_address: "203.0.113.7:19999".parse().unwrap(),
                }
            );
            assert_eq!(regular.operator_payout_address, None);

            let node_id = [0x5Au8; 20];
            let evonode = marshal_params(
                pro_tx_hash.as_ptr(),
                address.as_ptr(),
                node_id.as_ptr(),
                36656,
                1443,
                std::ptr::null(),
            )
            .unwrap_or_else(|_| panic!("an evonode service marshals"));
            assert_eq!(
                evonode.service,
                ConfirmedMasternodeService::Evonode {
                    service_address: "203.0.113.7:19999".parse().unwrap(),
                    platform_node_id: node_id,
                    platform_p2p_port: 36656,
                    platform_http_port: 1443,
                }
            );
        }
    }

    #[test]
    fn should_marshal_the_suggestion_fields() {
        let suggestion = MasternodeServiceSuggestion {
            service_address: Some("1.2.3.4:9999".parse().unwrap()),
            is_evonode: true,
            platform_node_id: Some([4u8; 20]),
            platform_http_port: Some(443),
            pose_banned: true,
            has_extended_net_info: false,
        };
        let ffi = MasternodeServiceSuggestionFFI::from_suggestion(&suggestion);
        assert_eq!(
            unsafe { CStr::from_ptr(ffi.service_address) }
                .to_str()
                .unwrap(),
            "1.2.3.4:9999"
        );
        assert!(ffi.is_evonode);
        assert!(ffi.has_platform_node_id);
        assert_eq!(ffi.platform_node_id, [4u8; 20]);
        assert!(ffi.has_platform_http_port);
        assert_eq!(ffi.platform_http_port, 443);
        assert!(ffi.pose_banned);
        assert!(!ffi.has_extended_net_info);
        unsafe { platform_wallet_string_free(ffi.service_address) };

        let without_address =
            MasternodeServiceSuggestionFFI::from_suggestion(&MasternodeServiceSuggestion {
                service_address: None,
                is_evonode: false,
                platform_node_id: None,
                platform_http_port: None,
                pose_banned: false,
                has_extended_net_info: false,
            });
        assert!(without_address.service_address.is_null());
        assert!(!without_address.has_platform_node_id);
        assert!(!without_address.has_platform_http_port);
    }

    #[test]
    fn should_report_an_unknown_manager_handle_when_suggesting() {
        let pro_tx_hash = [0u8; 32];
        let mut out = MasternodeServiceSuggestionFFI::empty();
        out.pose_banned = true;
        let mut result = unsafe {
            platform_wallet_manager_masternode_update_service_suggestion(
                Handle::MAX,
                pro_tx_hash.as_ptr(),
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert!(out.service_address.is_null());
        assert!(!out.pose_banned, "the out-param is reset on every path");
        unsafe { platform_wallet_ffi_result_free(&mut result) };

        let mut result = unsafe {
            platform_wallet_manager_masternode_update_service_suggestion(
                Handle::MAX,
                std::ptr::null(),
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    }
}

//! FFI bindings for the masternode key-rotation (ProUpRegTx) action and its
//! stage-two reactivation — `platform_wallet::masternode::update_registrar`
//! and the explicit-values update-service.
//!
//! Entry-point families, all additive, mirroring the update-service module:
//!
//! - `..._masternode_update_registrar` / `..._tracked_masternode_update_registrar`
//!   (+ `_prepare_` variants): the owner-signed rotation. The wallet form
//!   derives the owner key at `owner_key_index`; the tracked form parses
//!   the host-vaulted owner key text (WIF or hex).
//! - `..._masternode_update_service_with_values` (+ `_prepare_`): stage two —
//!   re-assert caller-captured service values, signed with the (post-
//!   rotation, wallet-held) operator key. There is no tracked form: after a
//!   rotation the operator key is by definition a wallet key.
//! - `..._provider_key_candidates`: the wallet's operator / voting keys by
//!   index with their network-wide usage, for the rotation key picker.
//!
//! Rotating the operator key PoSe-bans the node with its service fields
//! reset until stage two lands — callers capture the entry's service values
//! BEFORE broadcasting the rotation.

use std::ffi::CString;
use std::os::raw::c_char;

use dashcore::hashes::Hash;
use platform_wallet::masternode::{
    execute_masternode_update_registrar, execute_masternode_update_service_with_values,
    parse_secret_for_role, prepare_masternode_update_registrar,
    prepare_masternode_update_service_with_values, provider_key_candidates, LocatorSecret,
    MasternodeKeyRole, MasternodeUpdateRegistrarParams, MasternodeUpdateServiceParams, OwnerSecret,
    ProviderKeyCandidate, UpdateServiceValues,
};
use platform_wallet::ProviderKeyKind;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};

use crate::core_wallet::FFICoreSignedTransaction;
use crate::error::*;
use crate::handle::*;
use crate::masternode_update_service::{resolve_context, wallet_provider_secret, ResolvedContext};
use crate::runtime::block_on_worker;
use crate::tracked_masternode::optional_string;
use crate::{check_ptr, unwrap_result_or_return};

/// Parse a host-supplied owner key text (WIF or 64-char hex) into the
/// secp256k1 secret + compression flag the compact signature header needs.
fn tracked_owner_secret(
    key_text: &str,
    network: dashcore::Network,
) -> Result<OwnerSecret, PlatformWalletFFIResult> {
    match parse_secret_for_role(key_text, MasternodeKeyRole::Owner, network) {
        Ok(LocatorSecret::Ecdsa { secret, compressed }) => Ok(OwnerSecret { secret, compressed }),
        Ok(_) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "the owner key must be a secp256k1 secret (WIF or 64-char hex)",
        )),
        Err(e) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("owner key is not usable: {e}"),
        )),
    }
}

unsafe fn marshal_registrar_params(
    pro_tx_hash: *const u8,
    has_new_operator_key_index: bool,
    new_operator_key_index: u32,
    has_new_voting_key_index: bool,
    new_voting_key_index: u32,
    payout_address: *const c_char,
) -> Result<MasternodeUpdateRegistrarParams, PlatformWalletFFIResult> {
    let payout = optional_string(payout_address)?.ok_or_else(|| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "the payout address is required: the update replaces the payout script on-chain",
        )
    })?;
    Ok(MasternodeUpdateRegistrarParams {
        pro_tx_hash: std::ptr::read(pro_tx_hash as *const [u8; 32]),
        new_operator_key_index: has_new_operator_key_index.then_some(new_operator_key_index),
        new_voting_key_index: has_new_voting_key_index.then_some(new_voting_key_index),
        payout_address: payout,
    })
}

enum RegistrarOutcome {
    Broadcast(*mut [u8; 32]),
    Prepare(*mut Handle),
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_update_registrar(
    context: ResolvedContext,
    params: MasternodeUpdateRegistrarParams,
    owner: OwnerSecret,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    outcome: RegistrarOutcome,
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
    match outcome {
        RegistrarOutcome::Broadcast(out_txid) => {
            let txid = unwrap_result_or_return!(block_on_worker(async move {
                let signer = MnemonicResolverCoreSigner::new(
                    signer_addr as *mut MnemonicResolverHandle,
                    wallet_id_bytes,
                    network,
                );
                execute_masternode_update_registrar(&wallet, &spv, params, owner, &signer).await
            }));
            *out_txid = txid.to_raw_hash().to_byte_array();
        }
        RegistrarOutcome::Prepare(out_transaction_handle) => {
            let (wallet, prepared) = unwrap_result_or_return!(block_on_worker(async move {
                let signer = MnemonicResolverCoreSigner::new(
                    signer_addr as *mut MnemonicResolverHandle,
                    wallet_id_bytes,
                    network,
                );
                prepare_masternode_update_registrar(&wallet, &spv, params, owner, &signer)
                    .await
                    .map(|prepared| (wallet, prepared))
            }));
            *out_transaction_handle =
                CORE_SIGNED_TRANSACTION_STORAGE.insert(FFICoreSignedTransaction {
                    wallet: wallet.core().clone(),
                    transaction: prepared,
                });
        }
    }
    PlatformWalletFFIResult::ok()
}

/// Broadcast a ProUpRegTx rotating a wallet-owned masternode's operator
/// and/or voting key to fresh wallet keys, signed with the wallet's owner
/// key at `owner_key_index`.
///
/// - `has_new_operator_key_index` / `has_new_voting_key_index` choose what
///   rotates; at least one is required. Rotating the operator key PoSe-bans
///   the node with its service fields reset — capture them first and follow
///   with `platform_wallet_manager_masternode_update_service_with_values`.
/// - `payout_address` is REQUIRED (non-null): the payload replaces the
///   payout script on-chain.
/// - `out_txid` — 32 wire-order bytes, zeroed on every path, written on
///   definitive success. `ErrorTransactionBroadcastUnconfirmed` is
///   ambiguous: never retry.
///
/// # Safety
/// Pointer args must be valid for the stated sizes; `mnemonic_resolver_handle`
/// must come from `dash_sdk_mnemonic_resolver_create` and remain valid for
/// the duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_masternode_update_registrar(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    owner_key_index: u32,
    has_new_operator_key_index: bool,
    new_operator_key_index: u32,
    has_new_voting_key_index: bool,
    new_voting_key_index: u32,
    payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_txid: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(out_txid);
    *out_txid = [0u8; 32];
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(payout_address);
    check_ptr!(mnemonic_resolver_handle);

    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let params = match marshal_registrar_params(
        pro_tx_hash,
        has_new_operator_key_index,
        new_operator_key_index,
        has_new_voting_key_index,
        new_voting_key_index,
        payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };
    let secret = match wallet_provider_secret(
        &context.wallet,
        ProviderKeyKind::Owner,
        owner_key_index,
        mnemonic_resolver_handle,
    ) {
        Ok(secret) => secret,
        Err(e) => return e,
    };
    // Wallet-derived owner keys are compressed secp256k1 keys.
    let owner = OwnerSecret {
        secret,
        compressed: true,
    };
    run_update_registrar(
        context,
        params,
        owner,
        mnemonic_resolver_handle,
        RegistrarOutcome::Broadcast(out_txid),
    )
}

/// Prepare-only sibling of
/// [`platform_wallet_manager_masternode_update_registrar`][]: identical up
/// to the broadcast, handing back a core signed-transaction handle with the
/// inputs reserved — broadcast, abandon or free it via the existing
/// `core_wallet_*_signed_transaction` verbs.
///
/// # Safety
/// As [`platform_wallet_manager_masternode_update_registrar`][].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_masternode_prepare_update_registrar(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    owner_key_index: u32,
    has_new_operator_key_index: bool,
    new_operator_key_index: u32,
    has_new_voting_key_index: bool,
    new_voting_key_index: u32,
    payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_transaction_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_transaction_handle);
    *out_transaction_handle = 0;
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(payout_address);
    check_ptr!(mnemonic_resolver_handle);

    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let params = match marshal_registrar_params(
        pro_tx_hash,
        has_new_operator_key_index,
        new_operator_key_index,
        has_new_voting_key_index,
        new_voting_key_index,
        payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };
    let secret = match wallet_provider_secret(
        &context.wallet,
        ProviderKeyKind::Owner,
        owner_key_index,
        mnemonic_resolver_handle,
    ) {
        Ok(secret) => secret,
        Err(e) => return e,
    };
    let owner = OwnerSecret {
        secret,
        compressed: true,
    };
    run_update_registrar(
        context,
        params,
        owner,
        mnemonic_resolver_handle,
        RegistrarOutcome::Prepare(out_transaction_handle),
    )
}

/// [`platform_wallet_manager_masternode_update_registrar`][] for a TRACKED
/// masternode: the owner key is the host-vaulted key text (WIF or 64-char
/// hex) instead of a wallet derivation; the fee and the new keys still come
/// from `wallet_id`.
///
/// # Safety
/// As the wallet form; `owner_key_text` must be a NUL-terminated UTF-8
/// string.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_tracked_masternode_update_registrar(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    owner_key_text: *const c_char,
    has_new_operator_key_index: bool,
    new_operator_key_index: u32,
    has_new_voting_key_index: bool,
    new_voting_key_index: u32,
    payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_txid: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(out_txid);
    *out_txid = [0u8; 32];
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(owner_key_text);
    check_ptr!(payout_address);
    check_ptr!(mnemonic_resolver_handle);

    let key_text = unwrap_result_or_return!(std::ffi::CStr::from_ptr(owner_key_text).to_str());
    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let params = match marshal_registrar_params(
        pro_tx_hash,
        has_new_operator_key_index,
        new_operator_key_index,
        has_new_voting_key_index,
        new_voting_key_index,
        payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };
    let owner = match tracked_owner_secret(key_text, context.network) {
        Ok(owner) => owner,
        Err(e) => return e,
    };
    run_update_registrar(
        context,
        params,
        owner,
        mnemonic_resolver_handle,
        RegistrarOutcome::Broadcast(out_txid),
    )
}

/// Prepare-only sibling of
/// [`platform_wallet_manager_tracked_masternode_update_registrar`][].
///
/// # Safety
/// As the broadcasting form.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_tracked_masternode_prepare_update_registrar(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    owner_key_text: *const c_char,
    has_new_operator_key_index: bool,
    new_operator_key_index: u32,
    has_new_voting_key_index: bool,
    new_voting_key_index: u32,
    payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_transaction_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_transaction_handle);
    *out_transaction_handle = 0;
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(owner_key_text);
    check_ptr!(payout_address);
    check_ptr!(mnemonic_resolver_handle);

    let key_text = unwrap_result_or_return!(std::ffi::CStr::from_ptr(owner_key_text).to_str());
    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let params = match marshal_registrar_params(
        pro_tx_hash,
        has_new_operator_key_index,
        new_operator_key_index,
        has_new_voting_key_index,
        new_voting_key_index,
        payout_address,
    ) {
        Ok(params) => params,
        Err(e) => return e,
    };
    let owner = match tracked_owner_secret(key_text, context.network) {
        Ok(owner) => owner,
        Err(e) => return e,
    };
    run_update_registrar(
        context,
        params,
        owner,
        mnemonic_resolver_handle,
        RegistrarOutcome::Prepare(out_transaction_handle),
    )
}

// MARK: stage two — explicit-values update service

#[allow(clippy::too_many_arguments)]
unsafe fn marshal_service_values(
    service_address: *const c_char,
    has_platform_node_id: bool,
    platform_node_id: *const u8,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    has_platform_http_port: bool,
    platform_http_port: u16,
) -> Result<UpdateServiceValues, PlatformWalletFFIResult> {
    let service = optional_string(service_address)?.ok_or_else(|| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "the service address is required",
        )
    })?;
    let node_id = if has_platform_node_id {
        if platform_node_id.is_null() {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorNullPointer,
                "platform_node_id is null despite has_platform_node_id",
            ));
        }
        Some(std::ptr::read(platform_node_id as *const [u8; 20]))
    } else {
        None
    };
    Ok(UpdateServiceValues {
        service_address: service,
        platform_node_id: node_id,
        platform_p2p_port: has_platform_p2p_port.then_some(platform_p2p_port),
        platform_http_port: has_platform_http_port.then_some(platform_http_port),
    })
}

/// Stage two of an operator rotation: broadcast a ProUpServTx re-asserting
/// caller-captured service values (the registrar update reset the entry's
/// own), signed with the wallet's operator key at `operator_key_index` —
/// after a rotation that key is by definition a wallet key, so there is no
/// tracked form.
///
/// `operator_payout_address` follows the same reward-driven rule as the
/// unban path. `out_txid` is zeroed on every path.
///
/// # Safety
/// Pointer args must be valid for the stated sizes; `mnemonic_resolver_handle`
/// must come from `dash_sdk_mnemonic_resolver_create` and remain valid for
/// the duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_masternode_update_service_with_values(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_index: u32,
    service_address: *const c_char,
    has_platform_node_id: bool,
    platform_node_id: *const u8,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    has_platform_http_port: bool,
    platform_http_port: u16,
    operator_payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_txid: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(out_txid);
    *out_txid = [0u8; 32];
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(service_address);
    check_ptr!(mnemonic_resolver_handle);

    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let values = match marshal_service_values(
        service_address,
        has_platform_node_id,
        platform_node_id,
        has_platform_p2p_port,
        platform_p2p_port,
        has_platform_http_port,
        platform_http_port,
    ) {
        Ok(values) => values,
        Err(e) => return e,
    };
    let operator_payout_address = match optional_string(operator_payout_address) {
        Ok(text) => text,
        Err(e) => return e,
    };
    let params = MasternodeUpdateServiceParams {
        pro_tx_hash: std::ptr::read(pro_tx_hash as *const [u8; 32]),
        platform_p2p_port: None,
        operator_payout_address,
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

    let ResolvedContext {
        wallet,
        spv,
        network,
    } = context;
    let wallet_id_bytes = wallet.wallet_id();
    let signer_addr = mnemonic_resolver_handle as usize;
    let txid = unwrap_result_or_return!(block_on_worker(async move {
        let signer = MnemonicResolverCoreSigner::new(
            signer_addr as *mut MnemonicResolverHandle,
            wallet_id_bytes,
            network,
        );
        execute_masternode_update_service_with_values(
            &wallet,
            &spv,
            params,
            values,
            operator_secret,
            &signer,
        )
        .await
    }));
    *out_txid = txid.to_raw_hash().to_byte_array();
    PlatformWalletFFIResult::ok()
}

/// Prepare-only sibling of
/// [`platform_wallet_manager_masternode_update_service_with_values`][];
/// handle ownership as every other prepare entry point.
///
/// # Safety
/// As the broadcasting form.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_masternode_prepare_update_service_with_values(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    operator_key_index: u32,
    service_address: *const c_char,
    has_platform_node_id: bool,
    platform_node_id: *const u8,
    has_platform_p2p_port: bool,
    platform_p2p_port: u16,
    has_platform_http_port: bool,
    platform_http_port: u16,
    operator_payout_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_transaction_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_transaction_handle);
    *out_transaction_handle = 0;
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(service_address);
    check_ptr!(mnemonic_resolver_handle);

    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };
    let values = match marshal_service_values(
        service_address,
        has_platform_node_id,
        platform_node_id,
        has_platform_p2p_port,
        platform_p2p_port,
        has_platform_http_port,
        platform_http_port,
    ) {
        Ok(values) => values,
        Err(e) => return e,
    };
    let operator_payout_address = match optional_string(operator_payout_address) {
        Ok(text) => text,
        Err(e) => return e,
    };
    let params = MasternodeUpdateServiceParams {
        pro_tx_hash: std::ptr::read(pro_tx_hash as *const [u8; 32]),
        platform_p2p_port: None,
        operator_payout_address,
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
        prepare_masternode_update_service_with_values(
            &wallet,
            &spv,
            params,
            values,
            operator_secret,
            &signer,
        )
        .await
        .map(|prepared| (wallet, prepared))
    }));
    *out_transaction_handle = CORE_SIGNED_TRANSACTION_STORAGE.insert(FFICoreSignedTransaction {
        wallet: wallet.core().clone(),
        transaction: prepared,
    });
    PlatformWalletFFIResult::ok()
}

// MARK: key candidates

/// One wallet provider key with its network-wide usage — a rotation
/// key-picker row.
#[repr(C)]
pub struct ProviderKeyCandidateFFI {
    pub index: u32,
    /// Modern-serialization public key bytes; `public_key_len` says how
    /// many are meaningful (48 BLS operator, 33 secp voting).
    pub public_key: [u8; 48],
    pub public_key_len: u8,
    /// Whether a masternode-list entry currently uses this key.
    pub used: bool,
    /// proTxHash (wire order) of that entry; zeroed when unused.
    pub used_by_pro_tx_hash: [u8; 32],
    /// P2PKH address (voting keys only) — heap C string, freed by
    /// [`platform_wallet_manager_free_provider_key_candidates`]; null for
    /// BLS keys.
    pub address: *mut c_char,
}

fn candidate_to_ffi(candidate: ProviderKeyCandidate) -> ProviderKeyCandidateFFI {
    let mut public_key = [0u8; 48];
    let len = candidate.public_key_bytes.len().min(48);
    public_key[..len].copy_from_slice(&candidate.public_key_bytes[..len]);
    let address = candidate
        .address
        .and_then(|a| CString::new(a).ok())
        .map_or(std::ptr::null_mut(), CString::into_raw);
    ProviderKeyCandidateFFI {
        index: candidate.index,
        public_key,
        public_key_len: len as u8,
        used: candidate.used_by.is_some(),
        used_by_pro_tx_hash: candidate.used_by.unwrap_or([0u8; 32]),
        address,
    }
}

/// The wallet's first `count` provider keys of `kind`
/// ([`crate::provider_key_at_index::PROVIDER_KEY_KIND_OPERATOR`] = 10,
/// [`crate::provider_key_at_index::PROVIDER_KEY_KIND_VOTING`] = 8 — the
/// same account-type tags every provider-key FFI uses), each joined against the
/// live masternode list so a rotation picker can default to (and enforce)
/// unused keys. Fails with `ErrorMasternodeListUnavailable` before the list
/// has synced — "unused" cannot be asserted without it.
///
/// Free with [`platform_wallet_manager_free_provider_key_candidates`].
///
/// # Safety
/// Pointer args must be valid; `out_entries` / `out_count` receive a
/// Rust-owned array to be freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_provider_key_candidates(
    manager_handle: Handle,
    wallet_id: *const u8,
    kind: u8,
    count: u32,
    out_entries: *mut *mut ProviderKeyCandidateFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_entries);
    check_ptr!(out_count);
    *out_entries = std::ptr::null_mut();
    *out_count = 0;
    check_ptr!(wallet_id);

    let kind = match kind {
        crate::provider_key_at_index::PROVIDER_KEY_KIND_VOTING => ProviderKeyKind::Voting,
        crate::provider_key_at_index::PROVIDER_KEY_KIND_OPERATOR => ProviderKeyKind::Operator,
        other => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!(
                    "unsupported provider key kind {other} (expected {} voting or {} operator)",
                    crate::provider_key_at_index::PROVIDER_KEY_KIND_VOTING,
                    crate::provider_key_at_index::PROVIDER_KEY_KIND_OPERATOR
                ),
            );
        }
    };
    let context = match resolve_context(manager_handle, wallet_id) {
        Ok(context) => context,
        Err(e) => return e,
    };

    let ResolvedContext { wallet, spv, .. } = context;
    let candidates = unwrap_result_or_return!(block_on_worker(async move {
        let summaries = spv
            .masternode_list_summaries()
            .await
            .ok_or(platform_wallet::PlatformWalletError::MasternodeListUnavailable)?;
        provider_key_candidates(&wallet, &summaries, kind, count)
    }));

    let mut entries: Vec<ProviderKeyCandidateFFI> =
        candidates.into_iter().map(candidate_to_ffi).collect();
    entries.shrink_to_fit();
    *out_count = entries.len();
    let mut boxed = entries.into_boxed_slice();
    *out_entries = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    PlatformWalletFFIResult::ok()
}

/// Free an array returned by
/// [`platform_wallet_manager_provider_key_candidates`], including each
/// entry's heap address string.
///
/// # Safety
/// `entries` / `count` must be exactly what the candidates call returned;
/// call once.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_free_provider_key_candidates(
    entries: *mut ProviderKeyCandidateFFI,
    count: usize,
) {
    if entries.is_null() {
        return;
    }
    let boxed = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
    for entry in boxed.iter() {
        if !entry.address.is_null() {
            drop(CString::from_raw(entry.address));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_wallet_ffi_result_free;

    /// Unknown manager handles come back as invalid-handle errors with
    /// every out-param left at its zero state — the contract every
    /// masternode extern in this crate keeps.
    #[test]
    fn unknown_handles_are_invalid_handles() {
        unsafe {
            let wallet_id = [0u8; 32];
            let pro_tx_hash = [0u8; 32];
            let payout = std::ffi::CString::new("yPayout").unwrap();
            let resolver = std::ptr::dangling_mut::<MnemonicResolverHandle>();

            let mut txid = [0xAAu8; 32];
            let result = platform_wallet_manager_masternode_update_registrar(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                true,
                0,
                false,
                0,
                payout.as_ptr(),
                resolver,
                &mut txid,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
            assert_eq!(txid, [0u8; 32], "out_txid is zeroed on every path");
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);

            let key = std::ffi::CString::new("00").unwrap();
            let mut txid = [0xAAu8; 32];
            let result = platform_wallet_manager_tracked_masternode_update_registrar(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                key.as_ptr(),
                true,
                0,
                false,
                0,
                payout.as_ptr(),
                resolver,
                &mut txid,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
            assert_eq!(txid, [0u8; 32]);
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);

            let mut transaction_handle: Handle = 7;
            let result = platform_wallet_manager_masternode_prepare_update_registrar(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                true,
                0,
                false,
                0,
                payout.as_ptr(),
                resolver,
                &mut transaction_handle,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
            assert_eq!(transaction_handle, 0);
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);

            let service = std::ffi::CString::new("1.2.3.4:9999").unwrap();
            let mut txid = [0xAAu8; 32];
            let result = platform_wallet_manager_masternode_update_service_with_values(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                service.as_ptr(),
                false,
                std::ptr::null(),
                false,
                0,
                false,
                0,
                std::ptr::null(),
                resolver,
                &mut txid,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
            assert_eq!(txid, [0u8; 32]);
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);

            let mut entries: *mut ProviderKeyCandidateFFI = std::ptr::dangling_mut();
            let mut count: usize = 7;
            let result = platform_wallet_manager_provider_key_candidates(
                Handle::MAX,
                wallet_id.as_ptr(),
                10,
                5,
                &mut entries,
                &mut count,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
            assert!(entries.is_null());
            assert_eq!(count, 0);
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);
        }
    }

    /// A missing payout address is refused before the handle lookup could
    /// even matter — the payload would replace the payout script on-chain.
    #[test]
    fn registrar_requires_a_payout_address() {
        unsafe {
            let wallet_id = [0u8; 32];
            let pro_tx_hash = [0u8; 32];
            let resolver = std::ptr::dangling_mut::<MnemonicResolverHandle>();
            let mut txid = [0xAAu8; 32];
            let result = platform_wallet_manager_masternode_update_registrar(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                0,
                true,
                0,
                false,
                0,
                std::ptr::null(),
                resolver,
                &mut txid,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            assert_eq!(txid, [0u8; 32]);
            let mut result = result;
            platform_wallet_ffi_result_free(&mut result);
        }
    }
}

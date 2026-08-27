//! FFI bindings for the shielded spend pipeline (transitions
//! 15/16/17/19/20 — shield, transfer, unshield, withdraw,
//! identity-create-from-pool).
//!
//! Transitions 16/17/19 sign with the bound shielded wallet's
//! Orchard `SpendAuthorizingKey`, which lives on the
//! `OrchardKeySet` cached after [`platform_wallet_manager_bind_shielded`].
//! No host-side `Signer<PlatformAddress>` is required — the host
//! only supplies the recipient + amount (+ core fee rate for
//! withdrawal) and the resulting Halo 2 proof + state transition
//! is built and broadcast on the Rust side.
//!
//! Transition 20 (`identity_create_from_pool` — Shielded→new
//! identity) additionally takes the new identity's public keys plus
//! a host-supplied `Signer<IdentityPublicKey>` for the per-key
//! proofs-of-possession (mirroring address-funded identity
//! registration). The Orchard spend authority is still the bound
//! wallet's own `SpendAuthorizingKey`; only the new identity keys'
//! PoP signatures come from the host signer.
//!
//! Transition 15 (`shield` — Platform→Shielded) additionally
//! takes a host-supplied `Signer<PlatformAddress>` because the
//! input addresses' ECDSA signatures live in the host keychain.
//! Per-input nonces are fetched from Platform inside
//! [`ShieldedWallet::shield`] before building.
//!
//! Type 18 (`shield_from_asset_lock` — Core L1→Shielded) is wired
//! through [`platform_wallet_manager_shielded_fund_from_asset_lock`]
//! and its resume sibling further down. Both follow the
//! address-funding signer pattern: the asset-lock-proof signature
//! is produced by a `MnemonicResolverHandle` so the raw key never
//! crosses the FFI boundary.
//!
//! Feature-gated behind `shielded`. The accompanying
//! [`platform_wallet_shielded_warm_up_prover`] entry-point is
//! also defined here so hosts can pre-build the Halo 2 proving
//! key on a background thread at app startup.
//!
//! [`ShieldedWallet`]: platform_wallet::wallet::shielded::ShieldedWallet
//! [`ShieldedWallet::shield`]: platform_wallet::wallet::shielded::ShieldedWallet::shield

use std::ffi::CStr;
use std::os::raw::c_char;

use dashcore::hashes::Hash;
use dpp::address_funds::{OrchardAddress, PlatformAddress};
use dpp::prelude::Identifier;
use dpp::shielded::{
    compute_minimum_shielded_fee, compute_shielded_unshield_fee, compute_shielded_withdrawal_fee,
    ShieldedMemo,
};
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_wallet::wallet::asset_lock::AssetLockFunding;
use platform_wallet::wallet::shielded::{
    generate_one_time_orchard_key, orchard_address_from_spending_key, CachedOrchardProver,
};
use platform_wallet::PlatformWalletError;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::{decode_identity_pubkeys, IdentityPubkeyFFI};
use crate::runtime::{block_on_worker, runtime};
use crate::shielded_types::ShieldedShieldPreflightFFI;

/// A serialized `PlatformAddress` is exactly 21 bytes (1-byte variant tag + 20-byte hash).
const PLATFORM_ADDRESS_LEN: usize = 21;

/// Parse an optional platform address supplied as raw `PlatformAddress`
/// storage bytes (21 bytes: 1-byte variant tag + 20-byte hash — the
/// encoding `PlatformAddress::to_bytes()` produces and
/// `PlatformAddressWasm`/the Swift wrapper expose). Shared by the
/// `surplus_output` and `send_to_address_on_creation_failure` params;
/// `field_name` names the parameter in any error message.
///
/// `ptr == null` (or `len == 0`) means "no address" → `Ok(None)`.
/// A non-null pointer is read for `len` bytes and decoded; a malformed
/// address is surfaced as an `Err(PlatformWalletFFIResult)` so the
/// caller fails fast rather than building a transition the wallet would
/// reject.
///
/// # Safety
/// When `ptr` is non-null it must point to at least `len` readable
/// bytes for the duration of this call.
unsafe fn parse_optional_platform_address(
    ptr: *const u8,
    len: usize,
    field_name: &str,
) -> Result<Option<PlatformAddress>, PlatformWalletFFIResult> {
    if ptr.is_null() || len == 0 {
        return Ok(None);
    }
    // A serialized PlatformAddress is exactly 21 bytes (1-byte variant tag + 20-byte hash).
    // `from_bytes` decodes via bincode, which does NOT require full-slice consumption, so an
    // over-length buffer with a valid 21-byte prefix would otherwise be silently accepted (and the
    // trailing bytes dropped). Reject any non-21-byte input so a malformed/padded address fails fast
    // here rather than being silently truncated before signing.
    if len != PLATFORM_ADDRESS_LEN {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("{field_name} must be exactly {PLATFORM_ADDRESS_LEN} bytes, got {len}"),
        ));
    }
    let bytes = std::slice::from_raw_parts(ptr, len);
    match PlatformAddress::from_bytes(bytes) {
        Ok(addr) => Ok(Some(addr)),
        Err(e) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("invalid {field_name} platform address: {e}"),
        )),
    }
}

/// Decode a REQUIRED `PlatformAddress` from a raw pointer with no companion length argument over the
/// C ABI — the caller's safety contract guarantees exactly [`PLATFORM_ADDRESS_LEN`] readable bytes.
/// A null pointer or a malformed address is a hard error. `field_name` names the parameter in errors.
///
/// # Safety
/// `ptr` must point to at least [`PLATFORM_ADDRESS_LEN`] readable bytes for the duration of the call.
unsafe fn parse_required_platform_address(
    ptr: *const u8,
    field_name: &str,
) -> Result<PlatformAddress, PlatformWalletFFIResult> {
    match parse_optional_platform_address(ptr, PLATFORM_ADDRESS_LEN, field_name)? {
        Some(addr) => Ok(addr),
        None => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("{field_name} is required ({PLATFORM_ADDRESS_LEN} PlatformAddress bytes)"),
        )),
    }
}

/// Kick off the Halo 2 proving-key build on a background tokio
/// worker if it hasn't been built yet. Returns immediately —
/// hosts can call this at app startup without blocking the UI
/// thread. Subsequent calls are cheap no-ops once the key is
/// cached. The first shielded send still pays the ~30 s build
/// cost only if it fires before the warm-up worker finishes;
/// `platform_wallet_shielded_prover_is_ready` reports whether
/// that's the case.
///
/// Independent of any manager — the cache is a process-global
/// `OnceLock`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_shielded_warm_up_prover() {
    runtime().spawn_blocking(|| CachedOrchardProver::new().warm_up());
}

/// Whether the Halo 2 proving key has already been built.
///
/// Useful as a UI indicator ("preparing prover…") before the
/// first shielded send. `false` doesn't mean shielded sends will
/// fail — it just means the next one will pay the ~30s build
/// cost up front.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_shielded_prover_is_ready() -> bool {
    CachedOrchardProver::new().is_ready()
}

/// Map a fee `kind` byte to its consensus fee formula, or `None` for an
/// unknown kind. All three formulas share the `(num_actions, version) →
/// credits` shape, so the selection is version-independent and testable
/// against any explicit [`PlatformVersion`].
#[allow(clippy::type_complexity)]
fn shielded_fee_formula(
    kind: u8,
) -> Option<
    fn(usize, &dpp::version::PlatformVersion) -> Result<dpp::fee::Credits, dpp::ProtocolError>,
> {
    match kind {
        0 => Some(compute_minimum_shielded_fee),
        1 => Some(compute_shielded_unshield_fee),
        2 => Some(compute_shielded_withdrawal_fee),
        _ => None,
    }
}

/// Estimate the consensus-pinned flat shielded fee (in credits) for a
/// pool-paid shielded transition.
///
/// `kind` selects the transition's fee formula:
/// - `0` → ShieldedTransfer / Shield (`compute_minimum_shielded_fee` — the
///   base flat fee; Shield's structure check reserves the same base via
///   `compute_minimum_shielded_fee(2)`),
/// - `1` → Unshield (`compute_shielded_unshield_fee` — base + the flat
///   `AddBalanceToAddress` output-write cost),
/// - `2` → ShieldedWithdrawal (`compute_shielded_withdrawal_fee` — base +
///   the flat Core withdrawal-document cost).
///
/// `num_actions` is the Orchard action count of the bundle the host will
/// build (a single-note spend with change is 2 actions). The fee is
/// computed at `handle`'s manager's network-tracked platform version
/// (`sdk.version()`) — the same version the shielded builders in
/// `platform-wallet` resolve — so the estimate can't drift from the fee
/// the builder carves and the consensus gate validates, even when the
/// connected network hasn't activated the client's latest protocol
/// version yet.
///
/// No network round-trip and no wallet resolution — just the handle →
/// version lookup and a pure computation. Writes the fee to `out_fee` and
/// returns `ok()`. An unknown `kind` returns `ErrorInvalidParameter` (and
/// is checked before the handle, so it fails the same way regardless of
/// handle validity); an unknown `handle` returns `ErrorInvalidHandle`; a
/// fee-formula overflow returns `ErrorArithmeticOverflow`.
///
/// # Safety
/// `out_fee` must point to 8 writable bytes (a `u64`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_shielded_estimate_fee(
    handle: Handle,
    kind: u8,
    num_actions: usize,
    out_fee: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_fee);

    let Some(formula) = shielded_fee_formula(kind) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("unknown shielded fee kind {kind} (expected 0/1/2)"),
        );
    };
    let Some(platform_version) =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| manager.sdk().version())
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            format!("invalid manager handle: {handle}"),
        );
    };
    match formula(num_actions, platform_version) {
        Ok(credits) => {
            *out_fee = credits;
            PlatformWalletFFIResult::ok()
        }
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorArithmeticOverflow,
            format!("shielded fee estimation failed: {e}"),
        ),
    }
}

/// Encode an optional host-supplied memo string into the on-chain
/// 36-byte `DashMemo` layout via [`ShieldedMemo`].
///
/// Rules (the encoding decision lives here on the Rust side, not in
/// the Swift caller):
/// - `None` or an empty string → `ShieldedMemo::Empty` → all-zero
///   36 bytes (identical to today's hardcoded `[0u8; 36]`).
/// - Otherwise a UTF-8 text memo whose byte length must be ≤
///   [`MEMO_PAYLOAD_SIZE`]; over-length is rejected with
///   `ErrorInvalidParameter`.
///
/// Factored out as a pure function so the text→bytes rules are unit
/// testable without a live wallet handle.
fn encode_memo_text(memo_text: Option<&str>) -> Result<[u8; 36], PlatformWalletFFIResult> {
    match memo_text {
        None | Some("") => Ok(ShieldedMemo::Empty.to_bytes()),
        Some(text) => ShieldedMemo::text(text)
            .map(|memo| memo.to_bytes())
            .map_err(|e| {
                PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    e.to_string(),
                )
            }),
    }
}

/// Send a shielded → shielded transfer.
///
/// Spends notes from `wallet_id`'s shielded balance and creates a
/// new note for `recipient_raw_43`. `amount` is in credits
/// (1 DASH = 1e11 credits). Errors if the wallet has no bound
/// shielded sub-wallet, no spendable notes, or insufficient
/// shielded balance to cover `amount + estimated_fee`.
///
/// `mnemonic_resolver_handle` supplies the Orchard spend authority:
/// the resolver fires exactly once, the seed is derived in a
/// `Zeroizing` buffer, the account's `SpendAuthorizingKey` is
/// re-derived for this call only, and everything is dropped before
/// this function returns. No spend key is resident between spends
/// (mirror of the transparent `MnemonicResolverCoreSigner` flow).
///
/// `memo_text` is an optional NUL-terminated UTF-8 string attached
/// to the recipient's note. `null` or an empty string means no memo
/// (the all-zero 36-byte memo). A non-empty memo's UTF-8 byte length
/// must be ≤ 32; longer memos are rejected with
/// `ErrorInvalidParameter`. The 36-byte `DashMemo` encoding is done
/// on the Rust side.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must come from
///   `dash_sdk_mnemonic_resolver_create` and outlive this call; the
///   caller retains ownership.
/// - `recipient_raw_43` must point to 43 readable bytes (the
///   recipient's raw Orchard payment address — same shape
///   `platform_wallet_manager_shielded_default_address` returns).
/// - `memo_text`, when non-null, must be a valid NUL-terminated UTF-8
///   C string for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_transfer(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    recipient_raw_43: *const u8,
    amount: u64,
    memo_text: *const c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(recipient_raw_43);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let mut recipient = [0u8; 43];
    std::ptr::copy_nonoverlapping(recipient_raw_43, recipient.as_mut_ptr(), 43);

    // Decode the optional memo string before resolving the wallet so a
    // malformed memo fails fast without touching wallet state.
    let memo_str = if memo_text.is_null() {
        None
    } else {
        match CStr::from_ptr(memo_text).to_str() {
            Ok(s) => Some(s),
            Err(e) => {
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                    format!("memo_text is not valid UTF-8: {e}"),
                );
            }
        }
    };
    let memo = match encode_memo_text(memo_str) {
        Ok(m) => m,
        Err(result) => return result,
    };

    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };

    // Resolve the spend authority per-operation: mnemonic → seed in
    // `Zeroizing` buffers, scrubbed when the worker task drops them.
    let seed = match crate::identity_keys_from_mnemonic::resolve_seed_from_resolver(
        mnemonic_resolver_handle,
        &wallet_id,
    ) {
        Ok(seed) => seed,
        Err(result) => return result,
    };

    // Run the proof on a worker thread (8 MB stack). Halo 2 circuit
    // synthesis recurses past the ~512 KB iOS dispatch-thread stack
    // and crashes with EXC_BAD_ACCESS at the first
    // `synthesize(... measure(pass))` call when polled on the
    // calling thread.
    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        let r = wallet
            .shielded_transfer_to(
                &coordinator,
                seed.as_ref(),
                account,
                &recipient,
                amount,
                memo,
                &prover,
            )
            .await;
        poke_sync_on_unconfirmed(&r, handle);
        r
    });
    map_spend_result(result, "shielded transfer")
}

/// Unshield: spend shielded notes and send `amount` credits to a
/// platform address.
///
/// `to_platform_addr_cstr` is the recipient as a NUL-terminated
/// UTF-8 bech32m string (e.g. `"dash1..."` on mainnet,
/// `"tdash1..."` on testnet). The Rust side parses it via
/// `PlatformAddress::from_bech32m_string` so hosts don't have to
/// hand-roll the bincode storage variant tag (`0x00`/`0x01`),
/// which differs from the bech32m payload's type byte
/// (`0xb0`/`0x80`).
///
/// `mnemonic_resolver_handle` supplies the per-operation Orchard
/// spend authority (see `platform_wallet_manager_shielded_transfer`).
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must come from
///   `dash_sdk_mnemonic_resolver_create` and outlive this call; the
///   caller retains ownership.
/// - `to_platform_addr_cstr` must be a valid NUL-terminated UTF-8
///   C string for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_unshield(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    to_platform_addr_cstr: *const c_char,
    amount: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(to_platform_addr_cstr);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let to_addr_str = match CStr::from_ptr(to_platform_addr_cstr).to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                format!("to_platform_addr is not valid UTF-8: {e}"),
            );
        }
    };

    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };

    let seed = match crate::identity_keys_from_mnemonic::resolve_seed_from_resolver(
        mnemonic_resolver_handle,
        &wallet_id,
    ) {
        Ok(seed) => seed,
        Err(result) => return result,
    };

    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        let r = wallet
            .shielded_unshield_to(
                &coordinator,
                seed.as_ref(),
                account,
                &to_addr_str,
                amount,
                &prover,
            )
            .await;
        poke_sync_on_unconfirmed(&r, handle);
        r
    });
    map_spend_result(result, "shielded unshield")
}

/// Withdraw: spend shielded notes and send `amount` credits to a
/// Core L1 address. `to_core_address_cstr` is the address as a
/// Base58Check NUL-terminated UTF-8 string (e.g.
/// `"yL...."` on testnet); the Rust side parses it and verifies
/// the network matches the wallet's. `core_fee_per_byte` is the
/// L1 fee rate in duffs/byte (`1` is the dashmate default).
///
/// `mnemonic_resolver_handle` supplies the per-operation Orchard
/// spend authority (see `platform_wallet_manager_shielded_transfer`).
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must come from
///   `dash_sdk_mnemonic_resolver_create` and outlive this call; the
///   caller retains ownership.
/// - `to_core_address_cstr` must be a valid NUL-terminated UTF-8
///   C string for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_withdraw(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    to_core_address_cstr: *const c_char,
    amount: u64,
    core_fee_per_byte: u32,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(to_core_address_cstr);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let to_core = match CStr::from_ptr(to_core_address_cstr).to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                format!("to_core_address is not valid UTF-8: {e}"),
            );
        }
    };

    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };

    let seed = match crate::identity_keys_from_mnemonic::resolve_seed_from_resolver(
        mnemonic_resolver_handle,
        &wallet_id,
    ) {
        Ok(seed) => seed,
        Err(result) => return result,
    };

    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        let r = wallet
            .shielded_withdraw_to(
                &coordinator,
                seed.as_ref(),
                account,
                &to_core,
                amount,
                core_fee_per_byte,
                &prover,
            )
            .await;
        poke_sync_on_unconfirmed(&r, handle);
        r
    });
    map_spend_result(result, "shielded withdraw")
}

/// On the AMBIGUOUS outcome (broadcast accepted, result unconfirmed),
/// kick an immediate forced shielded sync so the first re-drive check —
/// nullifier re-check, then re-broadcast of the persisted transition —
/// happens now instead of at the next background tick.
///
/// Routed through the manager's [`ShieldedSyncManager::sync_now`] so the
/// pass respects the same `is_syncing` CAS + `quiescing` drain barrier as
/// the periodic loop and the host's Sync Now button — a raw
/// `coordinator.sync(...)` here would race both. `force = true` bypasses
/// only the caught-up cooldown, never the serialization gate. If a pass
/// is already in flight the poke no-ops (empty summary) and the next
/// tick's pass picks the redrive up — that pass's pre-scan snapshot may
/// predate this arm, which is fine.
///
/// Fire-and-forget: the spend's own result is already decided and the
/// sync pass owns resolution from here (`redrive_pending_spends` + the
/// prune backstop); the pass outcome is logged, not surfaced.
///
/// [`ShieldedSyncManager::sync_now`]: platform_wallet::manager::shielded_sync::ShieldedSyncManager::sync_now
fn poke_sync_on_unconfirmed<T>(result: &Result<T, PlatformWalletError>, handle: Handle) {
    let ambiguous = matches!(
        result,
        Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
            | Err(PlatformWalletError::ShieldedBroadcastUnconfirmed { .. })
    );
    if !ambiguous {
        return;
    }
    let Some(sync_manager) =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| manager.shielded_sync_arc())
    else {
        return;
    };
    runtime().spawn(async move {
        let summary = sync_manager.sync_now(true).await;
        if summary.sync_unix_seconds == 0 {
            tracing::debug!(
                "post-unconfirmed shielded sync poke skipped (a pass was already in flight                  or shielded is unconfigured); the next pass owns the re-drive"
            );
        } else {
            tracing::debug!(
                wallets = summary.wallet_results.len(),
                "post-unconfirmed shielded sync pass completed"
            );
        }
    });
}

/// Map a shielded operation outcome (shield / unshield / transfer /
/// withdraw) to a typed FFI result, mirroring the identity-create sibling's
/// code split so hosts can tell "definitively failed, safe to retry" from
/// "may have executed, do NOT retry".
fn map_spend_result(
    result: Result<(), PlatformWalletError>,
    operation: &str,
) -> PlatformWalletFFIResult {
    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        // Ambiguous: the broadcast was accepted but its execution result
        // couldn't be confirmed — the host must NOT re-submit. For the
        // spend-based operations the notes stay reserved wallet-side; a
        // shield reserves nothing. Either way the next sync (or an app
        // restart) reconciles the outcome; the typed Display already
        // carries the operation name and guidance.
        Err(e @ PlatformWalletError::ShieldedSpendUnconfirmed { .. }) => {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorShieldedSpendUnconfirmed,
                e.to_string(),
            )
        }
        // Retryable: the wallet couldn't build the spend against any
        // Platform-recorded anchor yet (its commitment tree is mid-block after
        // an index-chunk sync). Nothing was broadcast and the notes were
        // released, so the host may retry after the next shielded sync.
        Err(e @ PlatformWalletError::ShieldedNoRecordedAnchor(_)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorShieldedNoRecordedAnchor,
            format!("Wallet is still syncing to a confirmed state — try again shortly. ({e})"),
        ),
        // Definitive failure: the transition was not executed and the notes
        // were released; the host may retry.
        Err(e @ PlatformWalletError::ShieldedBroadcastFailed(_)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorShieldedBroadcastFailed,
            format!("{operation} failed: {e}"),
        ),
        // Definitively failed on an address-nonce race (a shield spends platform
        // address funds; a shield reserves no notes). Its own code carries the
        // safe-to-retry contract AND lets the host recognize the self-healing
        // nonce mismatch — a plain retry re-fetches the nonce. Without this arm
        // it would regress to the generic `ErrorWalletOperation` below.
        Err(e @ PlatformWalletError::AddressNonceMismatch { .. }) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorAddressNonceMismatch,
            format!("{operation} failed: {e}"),
        ),
        // The cached Platform Payment-account set no longer covers the
        // requested claim plus input-0's fee reserve. Keep this distinct from
        // generic wallet-operation failures so hosts can refresh preflight and
        // re-confirm a smaller amount instead of retrying unchanged.
        Err(e @ PlatformWalletError::PlatformShieldCapacityExceeded { .. }) => {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorShieldedInsufficientBalance,
                format!("{operation} failed: {e}"),
            )
        }
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("{operation} failed: {e}"),
        ),
    }
}

/// Render a caught panic payload as a human-readable string.
fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

/// Run an FFI export body under [`std::panic::catch_unwind`], converting a panic into the
/// operation's contract-appropriate result `code` (with `guidance` appended to the message)
/// instead of letting it reach the `extern "C"` frame.
///
/// A Rust panic cannot unwind through a C ABI boundary: it aborts the process. The JNI layer
/// wraps its calls in `support::guard` (which catches panics and raises a Java exception), but
/// that guard sits on the FAR side of this `extern "C"` export, so it never sees the unwind — the
/// process is already gone. `block_on_worker` makes this reachable rather than theoretical: it
/// `.expect`s on the tokio `JoinError`, so any panic inside the proving future (Halo 2 synthesis,
/// note bookkeeping, the SDK) re-panics right here inside the export.
///
/// The `code` must be chosen per operation to preserve that operation's result contract — a
/// panic can strike after side effects (note reservation, a broadcast) have happened, so the
/// outcome is genuinely ambiguous and the code must never promise a definitive failure. See
/// [`catch_spend_panic`] and the per-export call sites.
///
/// NOTE: this guard is only effective where panics unwind. The Android (`*-android`) and
/// host/test profiles build with `panic = "unwind"`, so it works there; the iOS profiles
/// (`dev-ios` / `release-ios`) build with `panic = "abort"` as part of their staticlib size
/// tuning (see the workspace `Cargo.toml` profile comments), so on iOS a panic still aborts the
/// process before this guard can see it.
fn catch_panic_to_code(
    operation: &str,
    code: PlatformWalletFFIResultCode,
    guidance: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(payload) => PlatformWalletFFIResult::err(
            code,
            format!(
                "{operation} panicked: {}. {guidance}",
                panic_payload_message(payload.as_ref())
            ),
        ),
    }
}

/// Post-panic guidance for the note-spending exports. Paired with
/// `ErrorShieldedSpendUnconfirmed` in [`catch_spend_panic`].
const SPEND_PANIC_GUIDANCE: &str = "The spend may or may not have been broadcast — do NOT \
     retry; the next shielded sync reconciles the outcome.";

/// [`catch_panic_to_code`] specialized for the note-spending exports.
///
/// The panic is mapped to [`PlatformWalletFFIResultCode::ErrorShieldedSpendUnconfirmed`], NOT to
/// a definitive failure code: a panic can strike after the notes were reserved and even after the
/// transition was broadcast, so the outcome is genuinely ambiguous. That code's contract is
/// exactly the conservative one this needs — the host must not auto-retry, the reservation stays
/// in place, and the next nullifier sync (or an app restart) reconciles whether the spend landed.
fn catch_spend_panic(
    operation: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        operation,
        PlatformWalletFFIResultCode::ErrorShieldedSpendUnconfirmed,
        SPEND_PANIC_GUIDANCE,
        body,
    )
}

/// Post-panic guidance for the asset-lock funding exports. Paired with
/// `ErrorTransactionBroadcastUnconfirmed` in [`catch_funding_panic`].
const FUNDING_PANIC_GUIDANCE: &str = "The asset lock may or may not have been broadcast — do \
     NOT retry; the funding UTXOs stay reserved, and the reservation TTL or the next sync \
     reconciles the outcome (a tracked lock resumes via the resume entry point).";

/// [`catch_panic_to_code`] specialized for the asset-lock funding exports.
///
/// The panic is mapped to
/// [`PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed`], NOT to a definitive
/// failure code: a panic can strike after the asset-lock transaction reached the wire (broadcast
/// precedes the ChainLock wait, the Platform submit, and the note bookkeeping), so the outcome
/// is genuinely ambiguous. That code's contract is exactly the conservative one this needs — the
/// host must not auto-retry, the funding UTXOs' reservation is still held (`ReservationToken` is
/// a plain id, not a drop-release guard, so the unwind does not free it and an immediate retry
/// fails at input selection instead of double-spending), and the reservation TTL or a sync
/// observing the transaction reconciles the outcome; a tracked lock is resumable through
/// `platform_wallet_manager_shielded_resume_fund_from_asset_lock`.
fn catch_funding_panic(
    operation: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        operation,
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        FUNDING_PANIC_GUIDANCE,
        body,
    )
}

/// Post-panic guidance for the one-time-key invitation claim. Paired with the
/// generic [`PlatformWalletFFIResultCode::ErrorUnknown`] in
/// [`catch_one_time_claim_panic`].
///
/// Unlike [`SPEND_PANIC_GUIDANCE`], this does NOT say "do not retry": a claim's
/// durable recovery record is retained until the created identity is durably
/// registered, and re-running the same claim is exactly how that record is
/// resumed — it recovers the identity the first attempt created rather than
/// creating a second one. What it must not do is retry IMMEDIATELY: the panic
/// unwound past the claim's deterministic lease release, so its admission and
/// its per-invitation reservation are reclaimed by expiry, and an attempt
/// before then is refused as busy.
const ONE_TIME_CLAIM_PANIC_GUIDANCE: &str = "The claim may or may not have been broadcast and \
     the new identity may already exist on chain — out_identity_id was NOT written, and the \
     identity slot must NOT be released. The claim's recovery record is retained: re-running \
     this same invitation claim once its lease expires resumes it and recovers the identity \
     rather than creating a second one.";

/// [`catch_panic_to_code`] specialized for the one-time-key claim export
/// (#4313 review finding 945163f6ed5b).
///
/// The panic maps to the generic [`PlatformWalletFFIResultCode::ErrorUnknown`],
/// deliberately NOT to any of this export's richer codes, none of which a panic
/// can honestly promise:
///
/// * `ErrorShieldedBroadcastUnconfirmed` (17) — its ABI contract says
///   `out_identity_id` IS written on that code, and a panic destroyed the
///   result, so there is no id to write.
/// * `ErrorShieldedInviteAlreadyClaimed` (43) — TERMINAL. Reporting it would
///   tell the claimer the invitation can never be claimed again, which is the
///   single worst thing to say about an outcome nobody knows.
/// * `ErrorShieldedScanBudgetExhausted` (44) / `ErrorShieldedLifecycleBusy`
///   (45) — both promise that nothing was scanned, built, or broadcast. A
///   panic can strike after the broadcast.
/// * `ErrorWalletOperation` (6) and `ErrorShieldedBroadcastFailed` (16) —
///   definitive failures, which this is not.
///
/// No dedicated panic code exists in the registry-tracked enum, and minting one
/// here would risk the cross-branch numeric collisions the codes 28-33 comment
/// warns about — so the generic internal code carries it, with the recovery
/// contract spelled out in the message.
fn catch_one_time_claim_panic(
    operation: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        operation,
        PlatformWalletFFIResultCode::ErrorUnknown,
        ONE_TIME_CLAIM_PANIC_GUIDANCE,
        body,
    )
}

/// Preserve the typed funding reports that hosts branch on across the FFI
/// boundary while keeping every other funding failure on the existing generic
/// error path.
///
/// Both preserved variants reach their dedicated code through the blanket
/// `From<PlatformWalletError> for PlatformWalletFFIResult` impl in
/// [`crate::error`], so `e.into()` also carries each typed `Display`
/// rendering verbatim — the structured figures ride the message string or
/// not at all (`PlatformWalletFFIResult` is ABI-frozen at code + message).
///
/// - `AssetLockAlreadyConsumed` -> `ErrorAssetLockAlreadyConsumed` (24). The
///   wallet retains nonterminal consumption-unknown state; the host must not
///   interpret this code as authenticated completion.
/// - `AssetLockInsufficientFunds` -> `ErrorAssetLockInsufficientFunds` (29).
///   Coin selection came up short over the permitted funding set, so nothing
///   was built or broadcast and no funding output was consumed; the host may
///   re-run preflight and retry. Recovery depends on which funding form
///   raised it, and BOTH reach this one code: an exact-amount build
///   (`AssetLockFunding::FromWalletBalance`) can be re-confirmed at a smaller
///   amount, but the whole-account CoinJoin *drain* takes no amount argument
///   at all — there is nothing to lower. A drain shortfall means the account's
///   drainable balance sits under the required minimum lock floor, so the
///   host's only remedies are to add funds to that account or lower the
///   floor. Do not surface "try a smaller amount" for the drain form.
///
///   Without this arm the shortfall flattened into the generic
///   `ErrorWalletOperation` (6) catch-all below, hiding a typed error behind
///   the code every unclassified failure already uses and forcing hosts back
///   to substring-matching the Display text.
fn map_asset_lock_funding_result(
    result: Result<(), PlatformWalletError>,
    operation: &str,
) -> PlatformWalletFFIResult {
    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(e @ PlatformWalletError::AssetLockAlreadyConsumed(_)) => e.into(),
        Err(e @ PlatformWalletError::AssetLockInsufficientFunds { .. }) => e.into(),
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("{operation} failed: {e}"),
        ),
    }
}

/// IdentityCreateFromShieldedPool (Type 20): spend `account`'s shielded notes to fund a brand-new
/// Platform identity.
///
/// The host supplies the new identity's public keys (`identity_pubkeys` rows, same
/// [`IdentityPubkeyFFI`] shape as address-funded registration) and a chosen `denomination` (a
/// member of the versioned exit-denomination set, in credits). The whole denomination leaves the
/// pool and the metered fee is taken from it, so the new identity is created holding
/// `denomination - total_fee`; any spent value above the denomination re-enters the pool as a
/// change note to `account`'s default Orchard address.
///
/// Authorization is 100% the Orchard proof + per-action spend-auth signatures (from the bound
/// wallet's own `SpendAuthorizingKey`) + the binding signature (which commits the derived id +
/// denomination + full key set) + a per-key proof-of-possession produced via
/// `signer_identity_handle`. There is NO platform identity signature.
///
/// `identity_index` is the DIP-9 identity-registration slot the new identity occupies. On a
/// successful broadcast the wallet registers the proof-verified identity at this slot in its local
/// `IdentityManager` (mirroring address-funded registration), which drives the host persister's
/// identity-row emit. It carries no decision here — it is marshalled straight through to the wallet.
///
/// On success the 32-byte new identity id (`double_sha256(sorted nullifiers)`) is written to
/// `out_identity_id`. The id is deterministic in the spent notes, so the host can also predict it
/// independently if needed.
///
/// `out_identity_id` is ALSO written on the [`ErrorShieldedBroadcastUnconfirmed`] result code: the
/// broadcast was accepted but its execution result couldn't be confirmed, so the derived id is
/// handed back (the identity may already exist on chain) and the host must hold the slot rather than
/// treat the registration as failed. On every other error code `out_identity_id` is left untouched.
///
/// [`ErrorShieldedBroadcastUnconfirmed`]: crate::error::PlatformWalletFFIResultCode::ErrorShieldedBroadcastUnconfirmed
///
/// `send_to_address_on_creation_failure_bytes` is the REQUIRED fallback platform address, supplied
/// as raw `PlatformAddress` storage bytes (21 bytes: 1-byte variant tag + 20-byte hash — the
/// encoding `PlatformAddress::to_bytes()` produces and `PlatformAddressWasm`/the Swift wrapper
/// expose). If identity creation fails a stateful check (a public-key hash already registered to
/// another identity) the spend is still finalized and the value is credited to this address minus a
/// penalty, exactly like the asset-lock / address-funded identity-create penalties. It is bound into
/// the transition sighash, so it cannot be redirected after signing.
///
/// `mnemonic_resolver_handle` supplies the per-operation Orchard spend authority (see
/// `platform_wallet_manager_shielded_transfer`).
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must come from `dash_sdk_mnemonic_resolver_create` and outlive
///   this call; the caller retains ownership.
/// - `identity_pubkeys` must point to `identity_pubkeys_count` contiguous [`IdentityPubkeyFFI`]
///   rows that outlive this call (each row's pointers per the [`IdentityPubkeyFFI`] contract).
/// - `send_to_address_on_creation_failure_bytes` must point to exactly 21 readable bytes for the
///   duration of this call.
/// - `signer_identity_handle` must be a valid, non-destroyed `*mut SignerHandle` (a
///   `VTableSigner` with the callback variant) that outlives this call; the caller retains
///   ownership.
/// - `out_identity_id` must point to 32 writable bytes. It is written on `Success` AND on the
///   `ErrorShieldedBroadcastUnconfirmed` result code (and only those); on all other codes it is
///   left as the caller initialized it.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_shielded_identity_create_from_pool(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    identity_index: u32,
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
    denomination: u64,
    send_to_address_on_creation_failure_bytes: *const u8,
    signer_identity_handle: *mut SignerHandle,
    out_identity_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(identity_pubkeys);
    check_ptr!(send_to_address_on_creation_failure_bytes);
    check_ptr!(signer_identity_handle);
    check_ptr!(out_identity_id);
    if identity_pubkeys_count == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "`identity_pubkeys_count` must be >= 1",
        );
    }

    // Decode the REQUIRED fallback failure address (raw `PlatformAddress` bytes: 1-byte variant tag +
    // 20-byte hash). The fallback is mandatory for Type 20, so a null / malformed address is a hard
    // error. No companion length arg crosses the C ABI — the helper enforces the 21-byte contract.
    let send_to_address_on_creation_failure = match parse_required_platform_address(
        send_to_address_on_creation_failure_bytes,
        "send_to_address_on_creation_failure_bytes",
    ) {
        Ok(addr) => addr,
        Err(result) => return result,
    };

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    // Decode the host-supplied identity keys into the
    // `Vec<(IdentityPublicKey, IdentityPublicKeyInCreation)>` shape the wallet builder consumes.
    // Reuses the shared registration decoder (key_type / purpose / security_level / contract-bounds
    // validation) so this path can't drift from the address-funded registration path.
    let keys_map = match decode_identity_pubkeys(identity_pubkeys, identity_pubkeys_count) {
        Ok(m) => m,
        Err(result) => return result,
    };
    let public_keys: Vec<(
        dpp::identity::IdentityPublicKey,
        IdentityPublicKeyInCreation,
    )> = keys_map
        .into_values()
        .map(|k| {
            let in_creation: IdentityPublicKeyInCreation = (&k).into();
            (k, in_creation)
        })
        .collect();

    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };

    // Resolve the per-operation Orchard spend authority before entering the worker; the seed
    // rides into the task in its `Zeroizing` buffer and is scrubbed when the task drops it.
    let seed = match crate::identity_keys_from_mnemonic::resolve_seed_from_resolver(
        mnemonic_resolver_handle,
        &wallet_id,
    ) {
        Ok(seed) => seed,
        Err(result) => return result,
    };

    // Round-trip the signer pointer through `usize` so the worker future captures only plain
    // `Send + 'static` data and re-materializes the borrow INSIDE the task — never a fabricated
    // `&'static` borrow of a host-owned vtable across the FFI boundary. The caller's contract is
    // that the handle outlives this call, and `block_on_worker` blocks the calling frame until the
    // task completes, so the borrow is valid for the task's whole lifetime.
    let signer_identity_addr = signer_identity_handle as usize;

    // Run the proof on a worker thread (8 MB stack). Halo 2 circuit synthesis recurses past the
    // ~512 KB iOS dispatch-thread stack and crashes with EXC_BAD_ACCESS when polled on the calling
    // thread.
    let result = block_on_worker(async move {
        // SAFETY: re-materialize the borrow under the caller's documented lifetime contract; valid
        // for the duration of this synchronously-awaited task. `VTableSigner` impls
        // `Signer<IdentityPublicKey>`.
        let identity_signer: &VTableSigner = &*(signer_identity_addr as *const VTableSigner);
        let prover = CachedOrchardProver::new();
        let r = wallet
            .shielded_identity_create_from_pool(
                &coordinator,
                seed.as_ref(),
                account,
                identity_index,
                public_keys,
                denomination,
                send_to_address_on_creation_failure,
                identity_signer,
                &prover,
            )
            .await;
        poke_sync_on_unconfirmed(&r, handle);
        r
    });

    match result {
        Ok(identity_id) => {
            *out_identity_id = identity_id.to_buffer();
            PlatformWalletFFIResult::ok()
        }
        // Broadcast accepted but its execution result couldn't be confirmed and a direct fetch came
        // back empty. The identity MAY exist on chain, so — unlike every other error arm — we still
        // write the derived id to `out_identity_id` (see the `# Safety` note) so the caller can hold
        // the slot against re-submission and surface the pending identity. The notes' reservations
        // were intentionally NOT released wallet-side.
        Err(PlatformWalletError::ShieldedBroadcastUnconfirmed {
            identity_id,
            ref reason,
        }) => {
            *out_identity_id = identity_id.to_buffer();
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorShieldedBroadcastUnconfirmed,
                format!(
                    "shielded identity-create-from-pool broadcast unconfirmed (identity {identity_id} may exist on chain): {reason}"
                ),
            )
        }
        // Retryable: no Platform-recorded anchor covered the selected notes yet
        // (the commitment tree is mid-block after an index-chunk sync). Nothing
        // was broadcast and the notes were released, so the host may retry after
        // the next shielded sync.
        Err(e @ PlatformWalletError::ShieldedNoRecordedAnchor(_)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorShieldedNoRecordedAnchor,
            format!("Wallet is still syncing to a confirmed state — try again shortly. ({e})"),
        ),
        // Definitive failure: the transition was not executed and the spent notes were released.
        Err(e @ PlatformWalletError::ShieldedBroadcastFailed(_)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorShieldedBroadcastFailed,
            format!("shielded identity-create-from-pool failed: {e}"),
        ),
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded identity-create-from-pool failed: {e}"),
        ),
    }
}

/// Sibling of [`platform_wallet_manager_shielded_identity_create_from_pool`], but
/// the Orchard spend authority is a foreign one-time spending key rather than the
/// wallet's own bound `OrchardKeySet`:
/// - `one_time_sk_bytes` — the invitation's single-use 32-byte Orchard spending
///   key. The wallet derives its fvk / ivk / ask, transiently scans the network
///   for the note(s) funded to it, and spends them.
/// - `change_address_raw43` — the claimer's OWN default Orchard address (43 raw
///   bytes: 11-byte diversifier + 32-byte pk_d) that receives any over-funding
///   change note. For a one-time invitation key the change is expected to be
///   zero, but over-funding is handled.
/// - `has_funding_birth_height` / `funding_birth_height` — an advisory birth-height
///   hint (`false` → `None`, following the wallet-create birth-height override
///   convention). The shielded tree has no height→note-index oracle, so the hint
///   cannot seed the scan start today; the scan is value-bounded.
///
/// Everything else matches the pool sibling: `identity_pubkeys` /
/// `identity_pubkeys_count` (same [`IdentityPubkeyFFI`] rows), `denomination` (a
/// member of the versioned exit set), `send_to_address_on_creation_failure_bytes`
/// (REQUIRED 21-byte `PlatformAddress` fallback bound into the sighash),
/// `identity_index` (the local registration slot), and `signer_identity_handle`
/// (the identity PoP signer). Blocks for the ~30 s Halo 2 proof.
///
/// On success the 32-byte new identity id is written to `out_identity_id`. As with
/// the pool sibling, `out_identity_id` is ALSO written on the
/// [`ErrorShieldedBroadcastUnconfirmed`] result code (the broadcast was accepted
/// but its execution result couldn't be confirmed — the identity may exist on
/// chain). On every other error code `out_identity_id` is left untouched.
///
/// [`ErrorShieldedBroadcastUnconfirmed`]: crate::error::PlatformWalletFFIResultCode::ErrorShieldedBroadcastUnconfirmed
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `one_time_sk_bytes` must point to exactly 32 readable bytes.
/// - `change_address_raw43` must point to exactly 43 readable bytes.
/// - `identity_pubkeys` must point to `identity_pubkeys_count` contiguous
///   [`IdentityPubkeyFFI`] rows that outlive this call.
/// - `send_to_address_on_creation_failure_bytes` must point to exactly 21
///   readable bytes for the duration of this call.
/// - `signer_identity_handle` must be a valid, non-destroyed `*mut SignerHandle`
///   (a `VTableSigner` with the callback variant) that outlives this call.
/// - `out_identity_id` must point to 32 writable bytes. Written on `Success` AND
///   on `ErrorShieldedBroadcastUnconfirmed` only. A panic inside the claim is
///   caught by [`catch_one_time_claim_panic`] and reported as `ErrorUnknown`
///   with `out_identity_id` left untouched.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_shielded_identity_create_from_one_time_key(
    handle: Handle,
    wallet_id_bytes: *const u8,
    one_time_sk_bytes: *const u8,
    has_funding_birth_height: bool,
    funding_birth_height: u32,
    change_address_raw43: *const u8,
    identity_index: u32,
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
    denomination: u64,
    send_to_address_on_creation_failure_bytes: *const u8,
    signer_identity_handle: *mut SignerHandle,
    out_identity_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    // Guarded: this export calls `block_on_worker`, which re-panics when its
    // Tokio task panics, so a panic in the transient scan, proof generation,
    // signing, wallet bookkeeping, or SDK processing would otherwise reach this
    // non-unwind `extern "C"` frame and ABORT the host process. The JNI layer's
    // `support::guard` cannot help — it sits on the far side of this export
    // (#4313 review finding 945163f6ed5b). See `catch_panic_to_code`.
    catch_one_time_claim_panic("shielded identity-create-from-one-time-key", || {
        shielded_identity_create_from_one_time_key_inner(
            handle,
            wallet_id_bytes,
            one_time_sk_bytes,
            has_funding_birth_height,
            funding_birth_height,
            change_address_raw43,
            identity_index,
            identity_pubkeys,
            identity_pubkeys_count,
            denomination,
            send_to_address_on_creation_failure_bytes,
            signer_identity_handle,
            out_identity_id,
        )
    })
}

/// Body of
/// [`platform_wallet_manager_shielded_identity_create_from_one_time_key`], as an
/// ordinary Rust function so a panic unwinds into
/// [`catch_one_time_claim_panic`] instead of across the C ABI.
///
/// `out_identity_id` is written only on this function's own return paths, so a
/// panic anywhere inside it leaves the caller's buffer exactly as it was.
///
/// # Safety
/// Identical contract to the export that calls it.
#[allow(clippy::too_many_arguments)]
unsafe fn shielded_identity_create_from_one_time_key_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    one_time_sk_bytes: *const u8,
    has_funding_birth_height: bool,
    funding_birth_height: u32,
    change_address_raw43: *const u8,
    identity_index: u32,
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
    denomination: u64,
    send_to_address_on_creation_failure_bytes: *const u8,
    signer_identity_handle: *mut SignerHandle,
    out_identity_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(one_time_sk_bytes);
    check_ptr!(change_address_raw43);
    check_ptr!(identity_pubkeys);
    check_ptr!(send_to_address_on_creation_failure_bytes);
    check_ptr!(signer_identity_handle);
    check_ptr!(out_identity_id);
    if identity_pubkeys_count == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "`identity_pubkeys_count` must be >= 1",
        );
    }

    // REQUIRED 21-byte fallback PlatformAddress (bound into the sighash).
    let send_to_address_on_creation_failure = match parse_required_platform_address(
        send_to_address_on_creation_failure_bytes,
        "send_to_address_on_creation_failure_bytes",
    ) {
        Ok(addr) => addr,
        Err(result) => return result,
    };

    // Copy the one-time spending key (32 bytes; the caller's safety contract
    // guarantees the length — no companion length arg crosses the C ABI).
    // Bearer spend authority: hold this FFI-layer copy in a `Zeroizing` buffer so
    // it is scrubbed on drop. It is moved into the wallet layer, which likewise
    // carries it in `Zeroizing` (#4204 key-hygiene).
    let mut one_time_sk = zeroize::Zeroizing::new([0u8; 32]);
    std::ptr::copy_nonoverlapping(one_time_sk_bytes, one_time_sk.as_mut_ptr(), 32);

    // Decode the claimer's own 43-byte default Orchard change address.
    let mut change_raw = [0u8; 43];
    std::ptr::copy_nonoverlapping(change_address_raw43, change_raw.as_mut_ptr(), 43);
    let change_address = match OrchardAddress::from_raw_bytes(&change_raw) {
        Ok(a) => a,
        Err(_) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "change_address_raw43 is not a valid 43-byte Orchard address",
            );
        }
    };

    let funding_birth_height = if has_funding_birth_height {
        Some(funding_birth_height)
    } else {
        None
    };

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    let keys_map = match decode_identity_pubkeys(identity_pubkeys, identity_pubkeys_count) {
        Ok(m) => m,
        Err(result) => return result,
    };
    let public_keys: Vec<(
        dpp::identity::IdentityPublicKey,
        IdentityPublicKeyInCreation,
    )> = keys_map
        .into_values()
        .map(|k| {
            let in_creation: IdentityPublicKeyInCreation = (&k).into();
            (k, in_creation)
        })
        .collect();

    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };

    let signer_identity_addr = signer_identity_handle as usize;

    // Run the proof on a worker thread (8 MB stack) — Halo 2 synthesis recurses
    // past the iOS dispatch-thread stack.
    let result = block_on_worker(async move {
        // SAFETY: re-materialize the borrow under the caller's documented lifetime
        // contract; valid for the duration of this synchronously-awaited task.
        let identity_signer: &VTableSigner = &*(signer_identity_addr as *const VTableSigner);
        let prover = CachedOrchardProver::new();
        let r = wallet
            .identity_create_from_one_time_key(
                &coordinator,
                one_time_sk,
                funding_birth_height,
                change_address,
                identity_index,
                public_keys,
                denomination,
                send_to_address_on_creation_failure,
                identity_signer,
                &prover,
            )
            .await;
        poke_sync_on_unconfirmed(&r, handle);
        r
    });

    let (identity_id_to_write, ffi_result) = map_one_time_claim_result(result);
    if let Some(identity_id) = identity_id_to_write {
        *out_identity_id = identity_id.to_buffer();
    }
    ffi_result
}

/// Classify a one-time-key claim outcome into its FFI result, plus the identity
/// id (if any) the entry point must write to `out_identity_id`.
///
/// Split out of
/// [`platform_wallet_manager_shielded_identity_create_from_one_time_key`] so the
/// code split below is reachable from a unit test without a live manager handle
/// — the same shape `map_spend_result` uses for the spend entry points. The
/// `Some(id)` return is the ONLY channel that writes `out_identity_id`, so the
/// "written on Success and on `ErrorShieldedBroadcastUnconfirmed` only" contract
/// in that function's safety docs is decided here and nowhere else.
fn map_one_time_claim_result(
    result: Result<Identifier, PlatformWalletError>,
) -> (Option<Identifier>, PlatformWalletFFIResult) {
    match result {
        Ok(identity_id) => (Some(identity_id), PlatformWalletFFIResult::ok()),
        Err(PlatformWalletError::ShieldedBroadcastUnconfirmed {
            identity_id,
            ref reason,
        }) => (
            Some(identity_id),
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorShieldedBroadcastUnconfirmed,
                format!(
                    "shielded identity-create-from-one-time-key broadcast unconfirmed (identity {identity_id} may exist on chain): {reason}"
                ),
            ),
        ),
        Err(e @ PlatformWalletError::ShieldedNoRecordedAnchor(_)) => (
            None,
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorShieldedNoRecordedAnchor,
                format!("Wallet is still syncing to a confirmed state — try again shortly. ({e})"),
            ),
        ),
        Err(e @ PlatformWalletError::ShieldedBroadcastFailed(_)) => (
            None,
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorShieldedBroadcastFailed,
                format!("shielded identity-create-from-one-time-key failed: {e}"),
            ),
        ),
        // Every variant that owns a typed code goes through the blanket
        // `From<PlatformWalletError>` conversion, because the catch-all below
        // would flatten it to the generic `ErrorWalletOperation` (6) and destroy
        // the retry-semantics discriminator the host classifies on:
        //
        // * `ShieldedInviteAlreadyClaimed` → 43, TERMINAL — the one signal that
        //   tells a claimer the invitation can never be claimed again
        //   (#4204 review finding 7be05fde0d09).
        // * `ShieldedForeignScanBudgetExhausted` → 44, RETRYABLE and cheap —
        //   the scan simply paused at its per-attempt budget with progress
        //   checkpointed. Flattened to 6 it reads as a hard failure, which
        //   strands a genuinely funded claim whose note sits deep in the tree
        //   (#4313 review finding, this entry point).
        // * `ShieldedLifecycleBusy` → 45, RETRYABLE — the claim was refused
        //   admission at the store (a purge holds it, or another claimant owns
        //   this invitation's claim-record key). Nothing was scanned, built or
        //   broadcast, so the host should simply retry.
        //
        // The blanket conversion is the single source of truth for all three;
        // this arm only keeps them from reaching the catch-all.
        Err(
            e @ (PlatformWalletError::ShieldedInviteAlreadyClaimed { .. }
            | PlatformWalletError::ShieldedForeignScanBudgetExhausted { .. }
            | PlatformWalletError::ShieldedLifecycleBusy { .. }),
        ) => (None, e.into()),
        Err(e) => (
            None,
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("shielded identity-create-from-one-time-key failed: {e}"),
            ),
        ),
    }
}

/// Preflight the maximum credits the cached state can shield from one Platform
/// Payment account.
///
/// Uses the exact same Rust planner as
/// [`platform_wallet_manager_shielded_shield`]: candidates are ordered by
/// lexicographic `PlatformAddress`, the leading prefix before the first address
/// whose balance is strictly greater than the shared fee reserve is excluded,
/// later addresses below the protocol version's minimum input amount are
/// omitted, and the lexicographically earliest usable set is truncated to the
/// versioned maximum address-input count. The reserve is retained only on input
/// 0. Capacity is therefore executable under the wallet's deterministic policy,
/// not globally optimized over later balances. No DAPI request, signing, proof
/// construction, or broadcast is performed.
///
/// A normal no-capacity result writes all numeric fields (including the total
/// account balance and zero usable/max capacity), returns `Success`, and carries
/// an advisory reason in the result message. Bad handles, missing wallets or
/// accounts, and arithmetic overflow remain FFI errors with `out` untouched.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `out` must point to a writable `ShieldedShieldPreflightFFI`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_shield_preflight(
    handle: Handle,
    wallet_id_bytes: *const u8,
    payment_account: u32,
    out: *mut ShieldedShieldPreflightFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(out);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let wallet = match resolve_wallet(handle, &wallet_id) {
        Ok(wallet) => wallet,
        Err(result) => return result,
    };

    let result =
        block_on_worker(async move { wallet.shielded_shield_preflight(payment_account).await });
    match result {
        Ok(preflight) => {
            *out = ShieldedShieldPreflightFFI {
                can_shield: preflight.can_shield,
                account_balance_credits: preflight.account_balance_credits,
                usable_balance_credits: preflight.usable_balance_credits,
                fee_reserve_credits: preflight.fee_reserve_credits,
                max_shieldable_credits: preflight.max_shieldable_credits,
            };
            match preflight.reason {
                Some(reason) => PlatformWalletFFIResult::success_with_message(reason),
                None => PlatformWalletFFIResult::ok(),
            }
        }
        Err(error) => error.into(),
    }
}

/// Shield: spend credits from a Platform Payment account into
/// the bound shielded sub-wallet's pool.
///
/// `shielded_account` selects which ZIP-32 Orchard account on
/// the bound shielded sub-wallet receives the new note.
/// `payment_account` selects which Platform Payment account on
/// the transparent side funds the shield (auto-selects input
/// addresses in lexicographic Platform-address order until the cumulative
/// balance covers `amount + fee buffer`).
///
/// `signer_address_handle` is a `*mut SignerHandle` produced by
/// `dash_sdk_signer_create_with_ctx` (typically Swift's
/// `KeychainSigner.handle`). The caller retains ownership; this
/// function does not destroy the handle.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `signer_address_handle` must be a valid, non-destroyed
///   `*const SignerHandle` that outlives this call and points at a
///   `VTableSigner` with the callback variant (the native variant
///   doesn't satisfy `Signer<PlatformAddress>`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_shield(
    handle: Handle,
    wallet_id_bytes: *const u8,
    shielded_account: u32,
    payment_account: u32,
    amount: u64,
    signer_address_handle: *const SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(signer_address_handle);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    // Shield writes its live activity entry to the coordinator's shared
    // in-memory store, so resolve the coordinator alongside the wallet
    // (same resolver the transfer / unshield / withdraw spends use).
    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };

    // Round-trip the signer pointer through `usize` so the worker
    // future captures only plain `Send + 'static` data and
    // re-materializes the borrow INSIDE the task — never a
    // fabricated `&'static` borrow of a host-owned vtable across
    // the FFI boundary. The caller's documented contract is that
    // the handle outlives this call, and `block_on_worker` blocks
    // the calling frame until the task completes, so the borrow is
    // valid for the task's whole lifetime. Avoids the latent UAF /
    // signing-oracle hazard if `block_on_worker` ever stops being
    // synchronous (cancellation, timeout, alternate executor).
    let signer_addr = signer_address_handle as usize;

    // Run the proof on a worker thread (8 MB stack). Halo 2 circuit
    // synthesis recurses past the ~512 KB iOS dispatch-thread stack
    // and crashes with EXC_BAD_ACCESS at the first
    // `synthesize(... measure(pass))` call when polled on the
    // calling thread.
    let result = block_on_worker(async move {
        // SAFETY: re-materialize the borrow under the caller's
        // documented lifetime contract; valid for the duration of
        // this synchronously-awaited task.
        let address_signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_shield_from_account(
                &coordinator,
                shielded_account,
                payment_account,
                amount,
                address_signer,
                &prover,
            )
            .await
    });
    map_spend_result(result, "shielded shield")
}

/// Shield: spend credits from a Platform Payment account into a
/// THIRD-PARTY shielded pool — the Type 15 shield with the note
/// assigned to `recipient_raw_43` (the recipient's raw 43-byte
/// Orchard payment address, same shape
/// `platform_wallet_manager_shielded_transfer` takes) instead of the
/// wallet's own default address.
///
/// Input selection, fees, and error shapes are identical to
/// [`platform_wallet_manager_shielded_shield`]; the wallet still needs
/// a bound shielded sub-wallet at `shielded_account` because the send
/// is OVK-encrypted to (and its activity recorded under) that account.
///
/// The recipient must actually be a third party: an address the
/// account's own IVK recognizes (default or any diversified index) is
/// rejected with a wallet-operation error — self-shields go through
/// [`platform_wallet_manager_shielded_shield`].
///
/// `memo_text` is an optional NUL-terminated UTF-8 string attached to
/// the recipient's note — same rules as
/// `platform_wallet_manager_shielded_transfer`: `null` or empty means
/// no memo; a non-empty memo's UTF-8 byte length must be ≤ 32.
///
/// `signer_address_handle` is a `*mut SignerHandle` produced by
/// `dash_sdk_signer_create_with_ctx` (typically Swift's
/// `KeychainSigner.handle`). The caller retains ownership; this
/// function does not destroy the handle.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `recipient_raw_43` must point to 43 readable bytes.
/// - `memo_text`, when non-null, must be a valid NUL-terminated UTF-8
///   C string for the duration of the call.
/// - `signer_address_handle` must be a valid, non-destroyed
///   `*const SignerHandle` that outlives this call and points at a
///   `VTableSigner` with the callback variant (the native variant
///   doesn't satisfy `Signer<PlatformAddress>`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_shield_to_recipient(
    handle: Handle,
    wallet_id_bytes: *const u8,
    shielded_account: u32,
    payment_account: u32,
    recipient_raw_43: *const u8,
    amount: u64,
    memo_text: *const c_char,
    signer_address_handle: *const SignerHandle,
) -> PlatformWalletFFIResult {
    // The whole body runs under `catch_unwind`: a panic (most concretely `block_on_worker`'s
    // `.expect` on a panicking proving task) must NOT reach this `extern "C"` frame, where it
    // would abort the process instead of surfacing to the host as a typed error. A shield
    // reserves no notes, but the transition may already have been broadcast when the panic
    // struck, so the same ambiguous spend-unconfirmed contract applies (matching
    // `map_spend_result`'s mapping for this operation); a later manual retry self-heals through
    // the address-nonce check.
    catch_spend_panic("shielded shield to recipient", || {
        shielded_shield_to_recipient_inner(
            handle,
            wallet_id_bytes,
            shielded_account,
            payment_account,
            recipient_raw_43,
            amount,
            memo_text,
            signer_address_handle,
        )
    })
}

/// Body of [`platform_wallet_manager_shielded_shield_to_recipient`], as an ordinary Rust
/// function so a panic unwinds into [`catch_spend_panic`] instead of across the C ABI.
///
/// # Safety
/// Identical contract to the export that calls it.
#[allow(clippy::too_many_arguments)]
unsafe fn shielded_shield_to_recipient_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    shielded_account: u32,
    payment_account: u32,
    recipient_raw_43: *const u8,
    amount: u64,
    memo_text: *const c_char,
    signer_address_handle: *const SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(recipient_raw_43);
    check_ptr!(signer_address_handle);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let mut recipient = [0u8; 43];
    std::ptr::copy_nonoverlapping(recipient_raw_43, recipient.as_mut_ptr(), 43);

    // Decode the optional memo string before resolving the wallet so a
    // malformed memo fails fast without touching wallet state.
    let memo_str = if memo_text.is_null() {
        None
    } else {
        match CStr::from_ptr(memo_text).to_str() {
            Ok(s) => Some(s),
            Err(e) => {
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                    format!("memo_text is not valid UTF-8: {e}"),
                );
            }
        }
    };
    let memo = match encode_memo_text(memo_str) {
        Ok(m) => m,
        Err(result) => return result,
    };

    // Shield writes its live activity entry to the coordinator's shared
    // in-memory store, so resolve the coordinator alongside the wallet
    // (same resolver the transfer / unshield / withdraw spends use).
    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };

    // Signer pointer round-trip through `usize` — same rationale as
    // `platform_wallet_manager_shielded_shield`.
    let signer_addr = signer_address_handle as usize;

    // Run the proof on a worker thread (8 MB stack). Halo 2 circuit
    // synthesis recurses past the ~512 KB iOS dispatch-thread stack
    // and crashes with EXC_BAD_ACCESS at the first
    // `synthesize(... measure(pass))` call when polled on the
    // calling thread.
    let result = block_on_worker(async move {
        // SAFETY: re-materialize the borrow under the caller's
        // documented lifetime contract; valid for the duration of
        // this synchronously-awaited task.
        let address_signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_shield_from_account_to_recipient(
                &coordinator,
                shielded_account,
                payment_account,
                &recipient,
                amount,
                memo,
                address_signer,
                &prover,
            )
            .await
    });
    map_spend_result(result, "shielded shield to recipient")
}

/// Fund the shielded pool from a Core L1 asset lock, orchestrated
/// through the wallet's `AssetLockManager` (build → IS-or-CL →
/// submit → consume). The asset-lock-proof signature is produced
/// by a `MnemonicResolverHandle` — the raw key never crosses the
/// FFI boundary.
///
/// `account_index` addresses the standard Core families: the asset
/// lock POOLS the BIP44 and BIP32 accounts at that index together
/// with every DashPay receiving account (change returns to BIP44);
/// the index does not restrict which DashPay receiving accounts
/// contribute. `amount_duffs` is the L1 amount to lock.
/// The wallet derives the shielded credit amount internally
/// (`lock_value − pool_fee`, where `pool_fee = shielded fee +
/// asset_lock_base_cost`) — callers don't need to know about
/// Type 18's Halo 2 fee math.
///
/// `recipient_raw_43` is the single Orchard recipient (same shape
/// `platform_wallet_manager_shielded_default_address` returns); it
/// receives the full `lock_value − pool_fee` credits.
///
/// `surplus_output_ptr` / `surplus_output_len` optionally supply a
/// platform address (raw `PlatformAddress` bytes: 1-byte variant tag +
/// 20-byte hash) to receive the asset-lock surplus
/// (`lock_value − shield_amount − pool_fee`). Pass `null` / `0` for
/// none. In this single-recipient "remainder" flow the wallet derives
/// `shield_amount = lock_value − pool_fee`, so the surplus is always
/// **zero** and a `null` surplus output is always valid; the parameter
/// is plumbed for API completeness and forward-compatibility with
/// multi-output / explicit-amount bundles.
///
/// Multi-recipient with explicit per-recipient amounts is reserved
/// for a future DPP-side Orchard multi-output bundle change; today
/// the orchestration rejects anything but a single recipient.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `recipient_raw_43` must point to 43 readable bytes (raw
///   Orchard payment address: 11-byte diversifier + 32-byte pk_d).
/// - `surplus_output_ptr`, when non-null, must point to
///   `surplus_output_len` readable bytes for the duration of the call.
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle` produced by
///   `dash_sdk_mnemonic_resolver_create`. The caller retains
///   ownership.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_shielded_fund_from_asset_lock(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account_index: u32,
    amount_duffs: u64,
    recipient_raw_43: *const u8,
    surplus_output_ptr: *const u8,
    surplus_output_len: usize,
    core_signer_handle: *mut MnemonicResolverHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(recipient_raw_43);
    check_ptr!(core_signer_handle);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    let mut recipient_bytes = [0u8; 43];
    std::ptr::copy_nonoverlapping(recipient_raw_43, recipient_bytes.as_mut_ptr(), 43);
    let recipient = match OrchardAddress::from_raw_bytes(&recipient_bytes) {
        Ok(a) => a,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("invalid Orchard recipient address: {e}"),
            );
        }
    };

    let surplus_output = match parse_optional_platform_address(
        surplus_output_ptr,
        surplus_output_len,
        "surplus_output",
    ) {
        Ok(s) => s,
        Err(result) => return result,
    };

    // The Type 18 live activity recorder writes to the coordinator's
    // shared in-memory store, so resolve the coordinator alongside the
    // wallet.
    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };
    let network = wallet.network();

    // Round-trip the resolver handle through `usize` so the worker
    // future's capture is `Send + 'static`.
    let core_signer_addr = core_signer_handle as usize;

    // Run the proof on a worker thread (8 MB stack). Halo 2 circuit
    // synthesis recurses past the ~512 KB iOS dispatch-thread stack
    // and crashes with EXC_BAD_ACCESS at the first
    // `synthesize(... measure(pass))` call when polled on the
    // calling thread.
    let result = block_on_worker(async move {
        // SAFETY: see the fn-level safety doc — the resolver handle
        // is pinned alive for the duration of this FFI call.
        let asset_lock_signer = unsafe {
            MnemonicResolverCoreSigner::new(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_fund_from_asset_lock(
                &coordinator,
                AssetLockFunding::FromWalletBalance {
                    amount_duffs,
                    account_index,
                },
                vec![(recipient, None)],
                &asset_lock_signer,
                &prover,
                surplus_output,
                // Single real note, no anonymity-set fillers (the multi-note
                // pool-seeding path uses its own dedicated FFI entry point).
                0,
                None,
                // User-facing funding: wait for the ChainLock indefinitely —
                // a broadcast asset lock is pending finality, never failed.
                None,
            )
            .await
    });
    map_asset_lock_funding_result(result, "shielded fund-from-asset-lock")
}

/// Fund the shielded pool by DRAINING the wallet's CoinJoin account
/// (`m/9'/coinType'/4'/account_index'`) into a single asset lock.
///
/// Sister to [`platform_wallet_manager_shielded_fund_from_asset_lock`],
/// with two differences:
///
/// 1. **Funding**: instead of coin-selecting an exact amount from a BIP44
///    account, every final CoinJoin UTXO is consumed and the lock value is
///    `Σ inputs − L1 fee`, computed by the builder. There is no amount
///    parameter, and the mixed coins never hop through a transparent BIP44
///    address — this is the CoinJoin → Shielded migration path.
/// 2. **No surplus output**: the single-recipient remainder flow pins the
///    consensus surplus to zero (see the resume sibling's doc), so the
///    parameter is omitted rather than plumbed.
///
/// The recipient receives `lock_value − pool_fee` credits. A stuck lock is
/// resumable via
/// [`platform_wallet_manager_shielded_resume_fund_from_asset_lock`] exactly
/// like a BIP44-funded one. The preflight rejects a drain whose balance
/// could not clear the Type 18 pool fee, so an unrecoverable dust lock is
/// never broadcast.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `recipient_raw_43` must point to 43 readable bytes (raw Orchard
///   payment address: 11-byte diversifier + 32-byte pk_d).
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle` produced by
///   `dash_sdk_mnemonic_resolver_create`. The caller retains ownership.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_fund_from_asset_lock_coinjoin_drain(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account_index: u32,
    recipient_raw_43: *const u8,
    core_signer_handle: *mut MnemonicResolverHandle,
) -> PlatformWalletFFIResult {
    // The whole body runs under `catch_unwind`: a panic (most concretely `block_on_worker`'s
    // `.expect` on a panicking funding/proving task) must NOT reach this `extern "C"` frame,
    // where it would abort the process instead of surfacing to the host as a typed error — on
    // Android that abort strikes before the JNI layer's `support::guard`, which sits on the far
    // side of this export. A panic can land after the whole-account lock was broadcast, so the
    // ambiguous broadcast-unconfirmed contract applies (the same code the flow's own
    // ambiguous-outcome errors use); the host resumes a tracked lock via
    // `platform_wallet_manager_shielded_resume_fund_from_asset_lock` rather than re-draining.
    catch_funding_panic("shielded CoinJoin-drain fund-from-asset-lock", || {
        shielded_fund_from_asset_lock_coinjoin_drain_inner(
            handle,
            wallet_id_bytes,
            account_index,
            recipient_raw_43,
            core_signer_handle,
        )
    })
}

/// Body of [`platform_wallet_manager_shielded_fund_from_asset_lock_coinjoin_drain`], as an
/// ordinary Rust function so a panic unwinds into [`catch_funding_panic`] instead of across
/// the C ABI.
///
/// # Safety
/// Identical contract to the export that calls it.
unsafe fn shielded_fund_from_asset_lock_coinjoin_drain_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account_index: u32,
    recipient_raw_43: *const u8,
    core_signer_handle: *mut MnemonicResolverHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(recipient_raw_43);
    check_ptr!(core_signer_handle);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    let mut recipient_bytes = [0u8; 43];
    std::ptr::copy_nonoverlapping(recipient_raw_43, recipient_bytes.as_mut_ptr(), 43);
    let recipient = match OrchardAddress::from_raw_bytes(&recipient_bytes) {
        Ok(a) => a,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("invalid Orchard recipient address: {e}"),
            );
        }
    };

    // The Type 18 live activity recorder writes to the coordinator's
    // shared in-memory store, so resolve the coordinator alongside the
    // wallet (same as the BIP44-funded sibling).
    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };
    let network = wallet.network();

    // Round-trip the resolver handle through `usize` so the worker
    // future's capture is `Send + 'static`.
    let core_signer_addr = core_signer_handle as usize;

    // Run the proof on a worker thread (8 MB stack) — see the sibling for
    // why the Halo 2 synthesis cannot run on the calling thread.
    let result = block_on_worker(async move {
        // SAFETY: see the fn-level safety doc — the resolver handle
        // is pinned alive for the duration of this FFI call.
        let asset_lock_signer = unsafe {
            MnemonicResolverCoreSigner::new(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_fund_from_asset_lock(
                &coordinator,
                AssetLockFunding::DrainAccountBalance {
                    account:
                        key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount::CoinJoin {
                            account_index,
                        },
                    // The shielded fund flow stamps the authoritative
                    // pool-fee floor before resolving the funding.
                    minimum_lock_duffs: None,
                },
                vec![(recipient, None)],
                &asset_lock_signer,
                &prover,
                // Single-recipient remainder flow: surplus is structurally
                // zero, so no surplus output.
                None,
                // Single real note, no anonymity-set fillers.
                0,
                None,
                // User-facing funding: wait for the ChainLock indefinitely —
                // a broadcast asset lock is pending finality, never failed.
                None,
            )
            .await
    });
    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        // Typed conversion — preserves the broadcast-outcome distinction
        // (ErrorTransactionBroadcastUnconfirmed vs ...Rejected) so the host
        // can choose resume/do-not-redrain for a possibly-broadcast
        // whole-account lock vs safely retrying a rejected build.
        Err(e) => e.into(),
    }
}

/// Resume a shielded fund-from-asset-lock by outpoint.
///
/// Sister to [`platform_wallet_manager_shielded_fund_from_asset_lock`]:
/// instead of building a fresh asset-lock transaction, pick up an
/// existing tracked lock at `out_point` and drive whatever stages
/// remain (broadcast, IS/CL wait, Platform submit, consume). Use
/// case mirrors the platform-address resume path — a prior attempt
/// left the lock in storage at `Broadcast` / `InstantSendLocked` /
/// `ChainLocked` but the shield ST never completed.
///
/// ## Resume / surplus-output desync — why no extra persistence is needed
///
/// The surplus destination is **signed over** on-chain (it sits before
/// the ECDSA `signature` in the signable bytes), so two resume attempts
/// that disagreed on the surplus would produce two different
/// transitions. We avoid that desync by construction rather than by
/// persisting the address with the in-flight lock:
///
/// - The orchestrated single-recipient flow always sets
///   `shield_amount = lock_value − pool_fee`, which pins the consensus
///   surplus (`lock_value − shield_amount − pool_fee`) to exactly
///   **zero** on every attempt — fresh build or resume.
/// - `shield_amount` is re-derived deterministically from the on-chain
///   lock value (read back from the tracked lock / IS proof) and the
///   versioned fee constants, so it is identical across restarts and
///   independent of any per-call input.
/// - With a zero surplus the `surplus_output` has no on-chain effect
///   (the action routes 0 credits to it), and `null` is always
///   consensus-valid (`0 ≤ shielded_implicit_fee_cap`). Each resume
///   re-signs a freshly-randomized bundle anyway (the Halo 2 proof draws
///   `OsRng` per build), so there is no "original signed transition" to
///   replay — only a stream of consensus-equivalent ones.
///
/// Net: a resume cannot strand or misdirect a surplus regardless of the
/// `surplus_output` passed here, so the surplus address is *not*
/// persisted on the `TrackedAssetLock`. The parameter is accepted for
/// signature parity with the fresh-build entry point; pass the same
/// value (typically `null`) on resume. If a future change introduces a
/// non-zero residual (e.g. explicit recipient amounts), the surplus
/// address would have to be persisted on the tracked lock and read back
/// here instead of trusted from the resume call.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `out_point` must be a valid, non-null pointer to an
///   `OutPointFFI` for the duration of the call.
/// - `surplus_output_ptr`, when non-null, must point to
///   `surplus_output_len` readable bytes for the duration of the call.
/// - `recipient_raw_43` / `core_signer_handle` — see
///   [`platform_wallet_manager_shielded_fund_from_asset_lock`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_shielded_resume_fund_from_asset_lock(
    handle: Handle,
    wallet_id_bytes: *const u8,
    out_point: *const OutPointFFI,
    recipient_raw_43: *const u8,
    surplus_output_ptr: *const u8,
    surplus_output_len: usize,
    core_signer_handle: *mut MnemonicResolverHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(out_point);
    check_ptr!(recipient_raw_43);
    check_ptr!(core_signer_handle);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    let mut recipient_bytes = [0u8; 43];
    std::ptr::copy_nonoverlapping(recipient_raw_43, recipient_bytes.as_mut_ptr(), 43);
    let recipient = match OrchardAddress::from_raw_bytes(&recipient_bytes) {
        Ok(a) => a,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("invalid Orchard recipient address: {e}"),
            );
        }
    };

    let surplus_output = match parse_optional_platform_address(
        surplus_output_ptr,
        surplus_output_len,
        "surplus_output",
    ) {
        Ok(s) => s,
        Err(result) => return result,
    };

    let out_point_ffi = *out_point;
    let resume_outpoint = dashcore::OutPoint {
        txid: dashcore::Txid::from_byte_array(out_point_ffi.txid),
        vout: out_point_ffi.vout,
    };

    // The Type 18 live activity recorder writes to the coordinator's
    // shared in-memory store, so resolve the coordinator alongside the
    // wallet.
    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };
    let network = wallet.network();

    let core_signer_addr = core_signer_handle as usize;

    let result = block_on_worker(async move {
        // SAFETY: see the fn-level safety doc — the resolver handle
        // is pinned alive for the duration of this FFI call.
        let asset_lock_signer = unsafe {
            MnemonicResolverCoreSigner::new(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_fund_from_asset_lock(
                &coordinator,
                AssetLockFunding::FromExistingAssetLock {
                    out_point: resume_outpoint,
                    consume_invitation_voucher: false,
                },
                vec![(recipient, None)],
                &asset_lock_signer,
                &prover,
                surplus_output,
                // Resuming a single-note fund (not a seeding batch).
                0,
                None,
                // User-facing funding: wait for the ChainLock indefinitely —
                // a broadcast asset lock is pending finality, never failed.
                None,
            )
            .await
    });
    map_asset_lock_funding_result(result, "shielded resume fund-from-asset-lock")
}

/// Seed the shielded pool's anonymity set up to `target_total_notes` by
/// submitting a series of `ShieldFromAssetLock` (Type 18) batches, each
/// adding up to 6 notes (1 real note to the wallet's own default
/// shielded address + up to 5 zero-value anonymity-set fillers). 6 is
/// `MAX_ACTIONS_PER_BATCH` in rs-platform-wallet's `seed_pool.rs` — the
/// most that fits the 20 KiB `max_state_transition_size`, NOT the
/// 16-action consensus cap.
///
/// Devnet/testnet ONLY — the Rust side hard-errors on `Network::Mainnet`
/// (the mainnet pool is seeded at genesis via `DRIVE_SHIELDED_SNAPSHOT`).
/// This exists so a freshly-reset devnet can satisfy the 250-note
/// outgoing-transition minimum from the example app in one action.
///
/// The asset-lock-proof signature for each batch is produced by a
/// `MnemonicResolverHandle` — the raw key never crosses the FFI boundary.
///
/// Batches run serially; each waits for proven execution before the next
/// starts (so a 250-note seed is roughly 42 batches and can take an hour
/// or more).
/// `progress_fn`, when non-null, is invoked before and after each batch
/// with the live counters so the host can render a progress UI. It is
/// called from a background worker thread — the host trampoline is
/// responsible for hopping to its own UI executor.
///
/// `account` is the shielded BIP44 account whose default address receives
/// each real note (must be bound via `bind_shielded`). `funding_account_index`
/// is the standard-family source index each per-batch asset lock funds from —
/// it POOLS the BIP44 and BIP32 accounts at that index with every DashPay
/// receiving account (change returns to BIP44) and does not restrict which
/// DashPay receiving accounts contribute.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle` produced by
///   `dash_sdk_mnemonic_resolver_create`. The caller retains ownership and
///   must keep it alive for the duration of this (blocking) call.
/// - `progress_fn`, when non-null, must be a valid C function pointer for
///   the duration of the call; `progress_ctx` is passed to it opaquely and
///   must remain valid for the duration of the call (or be null).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_shielded_seed_pool_notes(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account: u32,
    target_total_notes: u64,
    funding_account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    progress_fn: Option<
        unsafe extern "C" fn(
            context: *mut std::os::raw::c_void,
            batch_index: u64,
            batches_total_estimate: u64,
            pool_notes_now: u64,
            target: u64,
        ),
    >,
    progress_ctx: *mut std::os::raw::c_void,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(core_signer_handle);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    // Each seeding batch's Type 18 live activity recorder writes to the
    // coordinator's shared in-memory store, so resolve the coordinator
    // alongside the wallet.
    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };
    let network = wallet.network();

    // Round-trip the resolver handle and the progress context through
    // `usize` so the worker future's capture is `Send + 'static`. The
    // caller's documented contract pins both alive for the (blocking)
    // duration of this call, and `block_on_worker` blocks the calling
    // frame until the task completes.
    let core_signer_addr = core_signer_handle as usize;
    let progress_ctx_addr = progress_ctx as usize;

    // Run the proof + broadcast loop on a worker thread (8 MB stack):
    // Halo 2 circuit synthesis recurses past the ~512 KB iOS dispatch
    // thread stack.
    let result = block_on_worker(async move {
        // SAFETY: see the fn-level safety doc — the resolver handle is
        // pinned alive for the duration of this synchronously-awaited task.
        let asset_lock_signer = unsafe {
            MnemonicResolverCoreSigner::new(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };

        // Bridge the C progress callback into the Rust `Fn(SeedPoolProgress)`.
        // The fn pointer is `Send` and the context is moved as a `usize`;
        // both are re-materialized inside this task. A null `progress_fn`
        // makes the closure a no-op.
        let progress = move |p: platform_wallet::wallet::shielded::SeedPoolProgress| {
            if let Some(cb) = progress_fn {
                // SAFETY: `progress_ctx` (re-materialized from `progress_ctx_addr`)
                // and `cb` are valid for the duration of this call per the
                // fn-level contract.
                unsafe {
                    cb(
                        progress_ctx_addr as *mut std::os::raw::c_void,
                        p.batch_index,
                        p.batches_total_estimate,
                        p.pool_notes_now,
                        p.target,
                    );
                }
            }
        };

        wallet
            .shielded_seed_pool_notes(
                &coordinator,
                &wallet_id,
                account,
                target_total_notes,
                funding_account_index,
                &asset_lock_signer,
                progress,
                None,
            )
            .await
    });

    match result {
        Ok(_outcome) => PlatformWalletFFIResult::ok(),
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded seed-pool-notes failed: {e}"),
        ),
    }
}

/// Resolve a wallet without requiring shielded coordinator configuration.
///
/// Cached capacity preflight needs only the wallet's Platform Payment account;
/// requiring a bound/configured shielded coordinator here would turn an
/// otherwise valid balance query into a structural setup error.
fn resolve_wallet(
    handle: Handle,
    wallet_id: &[u8; 32],
) -> Result<std::sync::Arc<platform_wallet::PlatformWallet>, PlatformWalletFFIResult> {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.get_wallet(wallet_id))
    });
    match option {
        Some(Some(wallet)) => Ok(wallet),
        Some(None) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("wallet not found: {}", hex::encode(wallet_id)),
        )),
        None => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            format!("invalid manager handle: {handle}"),
        )),
    }
}

/// Resolve both the wallet `Arc` and the network-scoped shielded
/// coordinator `Arc` for the given manager handle. Shielded
/// spend operations need the coordinator's shared store, so this
/// is the right resolver for transfer/unshield/withdraw FFIs.
fn resolve_wallet_and_coordinator(
    handle: Handle,
    wallet_id: &[u8; 32],
) -> Result<
    (
        std::sync::Arc<platform_wallet::PlatformWallet>,
        std::sync::Arc<platform_wallet::wallet::shielded::NetworkShieldedCoordinator>,
    ),
    PlatformWalletFFIResult,
> {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(async {
            let wallet = manager.get_wallet(wallet_id).await;
            let coordinator = manager.shielded_coordinator().await;
            (wallet, coordinator)
        })
    });
    let (wallet_opt, coord_opt) = match option {
        Some(v) => v,
        None => {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                format!("invalid manager handle: {handle}"),
            ));
        }
    };
    let wallet = wallet_opt.ok_or_else(|| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("wallet not found: {}", hex::encode(wallet_id)),
        )
    })?;
    let coordinator = coord_opt.ok_or_else(|| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "shielded support not configured — call platform_wallet_manager_configure_shielded first",
        )
    })?;
    Ok((wallet, coordinator))
}

// ---------------------------------------------------------------------------
// One-time Orchard key generation (inviter side of L2 shielded invitations)
// ---------------------------------------------------------------------------

/// Generate a fresh one-time Orchard spending key and its default payment
/// address — the *inviter* side of an L2 shielded invitation.
///
/// Handle-less: a one-time key is process-local Orchard crypto, not bound
/// to any wallet. Writes the 32-byte spending key to `out_sk_32` and the 43
/// raw bytes of its default Orchard address (11-byte diversifier + 32-byte
/// `pk_d`, the same encoding
/// [`platform_wallet_manager_shielded_default_address`] returns) to
/// `out_address_43`.
///
/// The inviter funds a note to `out_address_43`; a claimer handed the 32
/// bytes in `out_sk_32` spends it via
/// [`platform_wallet_manager_shielded_identity_create_from_one_time_key`]
/// (which accepts exactly these spending-key bytes).
///
/// The generator re-rolls until it draws a valid scalar, so an invalid key is
/// never returned — but the call itself can still fail: an OS entropy failure
/// in the underlying RNG surfaces as [`ErrorWalletOperation`] (never a panic
/// across the C ABI). Always check the result code.
///
/// [`ErrorWalletOperation`]: crate::error::PlatformWalletFFIResultCode::ErrorWalletOperation
/// [`platform_wallet_manager_shielded_default_address`]: crate::platform_wallet_manager_shielded_default_address
///
/// # Safety
/// - `out_sk_32` must point at 32 writable bytes.
/// - `out_address_43` must point at 43 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_generate_one_time_orchard_key(
    out_sk_32: *mut u8,
    out_address_43: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_sk_32);
    check_ptr!(out_address_43);

    // `generate_one_time_orchard_key` uses `try_fill_bytes`, so an OS entropy
    // failure returns a typed error here rather than panicking. That matters:
    // this is a `#[no_mangle] extern "C"` export, so a panic would abort the
    // process across the C ABI before any JNI panic guard could convert it —
    // an OS RNG failure must surface as a normal error, never a hard abort.
    // `sk` is a `Zeroizing<[u8; 32]>`: the generator now scrubs every draw it
    // makes (including rejected ones) and hands the accepted key out still
    // wrapped, so this native copy is wiped on drop once it has been handed to
    // the caller's `out_sk_32` buffer — no explicit `zeroize()` needed, and the
    // scrub also covers the early-return paths (#4204 key-hygiene).
    let (sk, address) = match generate_one_time_orchard_key() {
        Ok(pair) => pair,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                e.to_string(),
            );
        }
    };
    std::ptr::copy_nonoverlapping(sk.as_ptr(), out_sk_32, 32);
    std::ptr::copy_nonoverlapping(address.as_ptr(), out_address_43, 43);
    PlatformWalletFFIResult::ok()
}

/// Derive the default raw Orchard payment address (43 bytes) from a 32-byte
/// Orchard spending key — the RNG-free counterpart of
/// [`platform_wallet_generate_one_time_orchard_key`].
///
/// Handle-less. On success the 43 raw address bytes (11-byte diversifier +
/// 32-byte `pk_d`) are written to `out_address_43`. Returns
/// [`ErrorInvalidParameter`] if `sk_bytes_32` is not a valid Orchard
/// `SpendingKey` scalar. Used for round-trip validation and to recompute
/// the recipient an inviter must fund for a given one-time key.
///
/// [`ErrorInvalidParameter`]: crate::error::PlatformWalletFFIResultCode::ErrorInvalidParameter
///
/// # Safety
/// - `sk_bytes_32` must point at 32 readable bytes.
/// - `out_address_43` must point at 43 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_orchard_address_from_spending_key(
    sk_bytes_32: *const u8,
    out_address_43: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(sk_bytes_32);
    check_ptr!(out_address_43);

    // Carry the caller-supplied bearer spending key in `Zeroizing` so THIS
    // frame's copy is scrubbed on drop, on every return path (#4204 key
    // hygiene). `orchard_address_from_spending_key` now takes the key BY
    // REFERENCE and contains its own derived `SpendingKey` in a scrub-on-drop
    // guard, so no unsanitized copy of the scalar is repeated at this
    // boundary (#4204 finding 1ee08ba70627).
    let mut sk = zeroize::Zeroizing::new([0u8; 32]);
    std::ptr::copy_nonoverlapping(sk_bytes_32, sk.as_mut_ptr(), 32);

    match orchard_address_from_spending_key(&sk) {
        Ok(address) => {
            std::ptr::copy_nonoverlapping(address.as_ptr(), out_address_43, 43);
            PlatformWalletFFIResult::ok()
        }
        // An invalid scalar is a bad caller-supplied key, not an internal
        // fault — surface it as an invalid parameter (the typed
        // `ShieldedKeyDerivation` message is preserved verbatim).
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            e.to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::shielded::MEMO_PAYLOAD_SIZE;

    #[test]
    fn encode_memo_text_none_is_empty() {
        let bytes = encode_memo_text(None).expect("None must encode");
        assert_eq!(bytes, [0u8; 36], "None must produce the all-zero memo");
        assert_eq!(ShieldedMemo::from_bytes(&bytes), ShieldedMemo::Empty);
    }

    #[test]
    fn encode_memo_text_empty_string_is_empty() {
        let bytes = encode_memo_text(Some("")).expect("empty string must encode");
        assert_eq!(
            bytes, [0u8; 36],
            "an empty string must produce the all-zero memo, not a kind-1 text memo"
        );
        assert_eq!(ShieldedMemo::from_bytes(&bytes), ShieldedMemo::Empty);
    }

    #[test]
    fn encode_memo_text_roundtrips_text() {
        let bytes = encode_memo_text(Some("thanks for lunch")).expect("text must encode");
        assert_eq!(
            ShieldedMemo::from_bytes(&bytes),
            ShieldedMemo::Text("thanks for lunch".to_string())
        );
    }

    #[test]
    fn encode_memo_text_max_length_multibyte_is_accepted() {
        // 8 × 🍕 = 32 bytes, exactly the payload limit.
        let s = "🍕".repeat(8);
        assert_eq!(s.len(), MEMO_PAYLOAD_SIZE);
        let bytes = encode_memo_text(Some(&s)).expect("a 32-byte memo must be accepted");
        assert_eq!(ShieldedMemo::from_bytes(&bytes), ShieldedMemo::Text(s));
    }

    #[test]
    fn encode_memo_text_over_limit_is_rejected() {
        let s = "a".repeat(MEMO_PAYLOAD_SIZE + 1);
        let err = encode_memo_text(Some(&s)).expect_err("a 33-byte memo must be rejected");
        assert_eq!(
            err.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "over-length memo must surface as an invalid-parameter error"
        );
    }

    /// Resolve the 2-action fee for `kind` at an explicit protocol version
    /// through the same formula table the FFI dispatches on.
    fn estimate_at(kind: u8, protocol_version: u32) -> u64 {
        let version = dpp::version::PlatformVersion::get(protocol_version)
            .expect("protocol version must exist");
        shielded_fee_formula(kind).expect("known kind")(2, version)
            .expect("fee formula must not overflow at 2 actions")
    }

    /// Pin the fee estimator to the on-chain ground-truth values observed at protocol 13 (the
    /// released fee constants) with 2 actions (single-note spend + change). The estimator resolves
    /// the version from the manager's network-tracked `sdk.version()`, so on a protocol-13 network
    /// these are the exact credits the builder carves and the consensus gate validates.
    #[test]
    fn estimate_fee_matches_observed_onchain_values_at_protocol_13() {
        // kind 0 — ShieldedTransfer / Shield base.
        assert_eq!(
            estimate_at(0, 13),
            162_851_200,
            "shielded transfer fee (2 actions, protocol 13)"
        );
        // kind 1 — Unshield.
        assert_eq!(
            estimate_at(1, 13),
            168_934_000,
            "unshield fee (2 actions, protocol 13)"
        );
        // kind 2 — ShieldedWithdrawal.
        assert_eq!(
            estimate_at(2, 13),
            275_191_200,
            "shielded withdrawal fee (2 actions, protocol 13)"
        );
    }

    /// The protocol-14 side of the boundary: the rebalanced constants
    /// (40M proof verification + 550 storage bytes/action). A network that
    /// has activated protocol 14 must quote these, and a network still on
    /// protocol 13 must NOT — the pre-fix estimator pinned
    /// `PlatformVersion::latest()` and silently under-quoted protocol-13
    /// networks by ~30%.
    #[test]
    fn estimate_fee_matches_rebalanced_values_at_protocol_14() {
        assert_eq!(
            estimate_at(0, 14),
            114_140_000,
            "shielded transfer fee (2 actions, protocol 14)"
        );
        assert_eq!(
            estimate_at(1, 14),
            120_222_800,
            "unshield fee (2 actions, protocol 14)"
        );
        assert_eq!(
            estimate_at(2, 14),
            226_480_000,
            "shielded withdrawal fee (2 actions, protocol 14)"
        );
    }

    /// The heart of the fix, exercised end-to-end through the exported
    /// estimator: the version is resolved from the manager handle's
    /// network-tracked `sdk.version()`, so managers pinned to protocol 13
    /// and protocol 14 quote different fees through the SAME entry point.
    /// Reverting the lookup to `PlatformVersion::latest()` fails the
    /// protocol-13 half of this test.
    #[test]
    fn estimate_fee_resolves_version_through_manager_handle() {
        unsafe extern "C" fn begin_changeset(
            _context: *mut std::os::raw::c_void,
            _wallet_id: *const u8,
        ) -> i32 {
            0
        }
        unsafe extern "C" fn end_changeset(
            _context: *mut std::os::raw::c_void,
            _wallet_id: *const u8,
            _success: bool,
        ) -> i32 {
            0
        }

        for (protocol_version, expected_transfer_fee) in
            [(13u32, 162_851_200u64), (14, 114_140_000)]
        {
            let version = dpp::version::PlatformVersion::get(protocol_version)
                .expect("protocol version must exist");
            let sdk = dash_sdk::SdkBuilder::new_mock()
                .with_version(version)
                .build()
                .expect("mock sdk");

            let persistence = crate::persistence::PersistenceCallbacks {
                on_changeset_begin_fn: Some(begin_changeset),
                on_changeset_end_fn: Some(end_changeset),
                ..Default::default()
            };
            let events = crate::event_handler::EventHandlerCallbacks {
                context: std::ptr::null_mut(),
                on_wallet_event_fn: None,
                on_error_fn: None,
                on_platform_address_sync_completed_fn: None,
                on_shielded_sync_completed_fn: None,
                on_shielded_sync_progress_fn: None,
                on_shielded_tree_progress_fn: None,
                release_fn: None,
            };
            let mut handle: Handle = 0;
            let create = unsafe {
                crate::manager::platform_wallet_manager_create(
                    &sdk as *const dash_sdk::Sdk as *const std::os::raw::c_void,
                    &persistence,
                    &events,
                    &mut handle,
                )
            };
            assert_eq!(
                create.code,
                PlatformWalletFFIResultCode::Success,
                "manager create must succeed at protocol {protocol_version}"
            );

            let mut fee: u64 = 0;
            let result = unsafe { platform_wallet_shielded_estimate_fee(handle, 0, 2, &mut fee) };
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::Success,
                "estimate must succeed at protocol {protocol_version}"
            );
            assert_eq!(
                fee, expected_transfer_fee,
                "2-action transfer fee quoted through a protocol-{protocol_version} manager"
            );

            let destroy = unsafe { crate::manager::platform_wallet_manager_destroy(handle) };
            assert_eq!(destroy.code, PlatformWalletFFIResultCode::Success);
        }
    }

    #[test]
    fn estimate_fee_rejects_unknown_kind() {
        unsafe {
            let mut fee: u64 = 0;
            // The kind check runs before handle resolution, so a bogus kind
            // fails identically with or without a live manager handle.
            let result = platform_wallet_shielded_estimate_fee(0, 7, 2, &mut fee);
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
        }
    }

    #[test]
    fn estimate_fee_rejects_unknown_manager_handle() {
        unsafe {
            let mut fee: u64 = 0;
            let result = platform_wallet_shielded_estimate_fee(0, 0, 2, &mut fee);
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                "a versionless fallback here would silently mis-quote — an \
                 unknown handle must be a hard error"
            );
        }
    }

    #[test]
    fn shield_preflight_rejects_null_abi_pointers() {
        unsafe {
            let mut out = ShieldedShieldPreflightFFI::default();
            let missing_wallet_id =
                platform_wallet_manager_shielded_shield_preflight(0, std::ptr::null(), 0, &mut out);
            assert_eq!(
                missing_wallet_id.code,
                PlatformWalletFFIResultCode::ErrorNullPointer
            );

            let wallet_id = [0u8; 32];
            let missing_out = platform_wallet_manager_shielded_shield_preflight(
                0,
                wallet_id.as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            assert_eq!(
                missing_out.code,
                PlatformWalletFFIResultCode::ErrorNullPointer
            );
        }
    }

    /// Read the Rust-owned message out of an FFI result for assertions.
    fn message_of(result: &PlatformWalletFFIResult) -> String {
        assert!(
            !result.message.is_null(),
            "error result must carry a message"
        );
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    }

    /// A non-panicking body passes its result straight through — the guard must be invisible on
    /// the happy path.
    #[test]
    fn catch_spend_panic_passes_results_through() {
        let ok = catch_spend_panic("test", PlatformWalletFFIResult::ok);
        assert_eq!(ok.code, PlatformWalletFFIResultCode::Success);

        let err = catch_spend_panic("test", || {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "bad input",
            )
        });
        assert_eq!(err.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
        assert_eq!(message_of(&err), "bad input");
    }

    /// A panic inside a shielded-spend export must NOT unwind into the `extern "C"` frame (that
    /// aborts the process). It becomes `ErrorShieldedSpendUnconfirmed` — the conservative
    /// "may have been broadcast, do NOT retry" contract, because a panic can strike after the
    /// transition is submitted.
    ///
    /// The panic hook is deliberately left alone: it is process-global, `cargo test` runs tests
    /// concurrently, and `take_hook` + restore from two tests can interleave so that one restores
    /// the other's temporary hook last — suppressing panic diagnostics for the rest of the
    /// process, including unrelated concurrent panics. The libtest harness already captures this
    /// test's output, so the deliberate panic's backtrace does not reach the console anyway.
    #[test]
    fn catch_spend_panic_maps_a_panic_to_the_unconfirmed_contract() {
        let result = catch_spend_panic("shielded shield to recipient", || {
            panic!("tokio worker panicked");
        });

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedSpendUnconfirmed,
            "a panic must map to the ambiguous, do-not-retry code"
        );
        let message = message_of(&result);
        assert!(
            message.contains("shielded shield to recipient panicked")
                && message.contains("tokio worker panicked"),
            "the panic payload must survive into the FFI message: {message}"
        );
        assert!(
            message.contains("do NOT retry"),
            "the message must carry the do-not-retry guidance: {message}"
        );
    }

    /// A panic inside the CoinJoin-drain funding export must NOT unwind into the `extern "C"`
    /// frame (that aborts the Android process before the JNI layer's own guard can translate it
    /// into a Java exception). It becomes `ErrorTransactionBroadcastUnconfirmed` — the
    /// conservative "may already be on the wire, do NOT retry" contract — because a panic can
    /// strike after the whole-account lock was broadcast. Same panic-hook note as the spend
    /// sibling above.
    #[test]
    fn catch_funding_panic_maps_a_panic_to_the_broadcast_unconfirmed_contract() {
        let result = catch_funding_panic("shielded CoinJoin-drain fund-from-asset-lock", || {
            panic!("tokio worker panicked");
        });

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
            "a funding panic must map to the ambiguous, do-not-retry code"
        );
        let message = message_of(&result);
        assert!(
            message.contains("shielded CoinJoin-drain fund-from-asset-lock panicked")
                && message.contains("tokio worker panicked"),
            "the panic payload must survive into the FFI message: {message}"
        );
        assert!(
            message.contains("do NOT retry"),
            "the message must carry the do-not-retry guidance: {message}"
        );
    }

    /// A panic inside the one-time-key CLAIM export must not unwind into the `extern "C"` frame
    /// either (#4313 review finding 945163f6ed5b). That export calls `block_on_worker`, which
    /// re-panics on a panicking Tokio task, so a panic in the transient scan, Halo 2 synthesis,
    /// signing, or SDK processing reached the C ABI and aborted the host process — the JNI
    /// `guard` is on the far side of the export and never saw it.
    ///
    /// The code must be `ErrorUnknown`, and the ambiguity contract must survive: a panic can
    /// strike after the broadcast, so the message must neither claim the invitation is spent
    /// (the terminal 43) nor that nothing happened.
    #[test]
    fn catch_one_time_claim_panic_keeps_the_claim_ambiguity_contract() {
        let result =
            catch_one_time_claim_panic("shielded identity-create-from-one-time-key", || {
                panic!("halo2 synthesis panicked");
            });

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUnknown,
            "a claim panic must map to the generic code — every richer code this export uses \
             promises something a panic cannot"
        );
        assert_ne!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedInviteAlreadyClaimed,
            "a panic must never be reported as the TERMINAL already-claimed outcome"
        );
        let message = message_of(&result);
        assert!(
            message.contains("shielded identity-create-from-one-time-key panicked")
                && message.contains("halo2 synthesis panicked"),
            "the panic payload must survive into the FFI message: {message}"
        );
        assert!(
            message.contains("may or may not have been broadcast"),
            "the message must preserve the ambiguity: {message}"
        );
        assert!(
            message.contains("out_identity_id was NOT written"),
            "the message must tell the host its out param is untouched: {message}"
        );
        assert!(
            message.contains("recovery record is retained"),
            "the message must point at the recovery path that makes this survivable: {message}"
        );
    }

    /// The pre-fix shape, side by side with the fixed one, so the difference the guard makes is
    /// pinned rather than asserted in prose (#4313 review finding 945163f6ed5b).
    ///
    /// `block_on_worker` `.expect`s on the Tokio `JoinError`, so a panicking claim task re-panics
    /// in the export's own frame. UNGUARDED, that unwind leaves the body and reaches the caller —
    /// which for a `#[no_mangle] extern "C"` function is a non-unwind C ABI frame, i.e. an
    /// immediate `abort` of the host process, with no result and no Java exception. GUARDED, the
    /// identical panic becomes a typed result the host can act on.
    ///
    /// The export's entire body is now the guard call, and
    /// `shielded_identity_create_from_one_time_key_inner` is private with exactly that one caller,
    /// so the wiring cannot be bypassed.
    #[test]
    fn an_unguarded_claim_body_lets_the_panic_escape_to_the_c_abi_frame() {
        // Pre-fix: the body was invoked directly by the export.
        let escaped = std::panic::catch_unwind(|| -> PlatformWalletFFIResult {
            panic!("halo2 synthesis panicked");
        });
        assert!(
            escaped.is_err(),
            "RED: the panic leaves the body and reaches the export frame, where it aborts"
        );

        // Post-fix: the same panic is contained and typed.
        let contained =
            catch_one_time_claim_panic("shielded identity-create-from-one-time-key", || {
                panic!("halo2 synthesis panicked");
            });
        assert_eq!(
            contained.code,
            PlatformWalletFFIResultCode::ErrorUnknown,
            "GREEN: the guard converts the identical panic into a result the host receives"
        );
    }

    /// The guard must be transparent to the ordinary outcomes — it wraps the whole export, so a
    /// bug here would corrupt every non-panicking claim result, including the
    /// `ErrorShieldedBroadcastUnconfirmed` path whose contract WRITES `out_identity_id`.
    #[test]
    fn catch_one_time_claim_panic_passes_results_through() {
        let ok = catch_one_time_claim_panic("test", PlatformWalletFFIResult::ok);
        assert_eq!(ok.code, PlatformWalletFFIResultCode::Success);

        let terminal = catch_one_time_claim_panic("test", || {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorShieldedInviteAlreadyClaimed,
                "already claimed",
            )
        });
        assert_eq!(
            terminal.code,
            PlatformWalletFFIResultCode::ErrorShieldedInviteAlreadyClaimed,
            "the guard must not rewrite a real terminal outcome"
        );
    }

    /// `map_spend_result` pins the retry-relevant code split the three spend
    /// entry points depend on:
    /// - `ShieldedSpendUnconfirmed` → `ErrorShieldedSpendUnconfirmed` (host
    ///   must NOT retry — the notes stay reserved; a retry could select other
    ///   unreserved notes and double-send),
    /// - `ShieldedBroadcastFailed` → `ErrorShieldedBroadcastFailed`
    ///   (definitive failure; reservations released; safe to retry),
    /// - any other variant → the generic `ErrorWalletOperation`.
    ///
    /// The typed `Display` rendering must survive into the result message in
    /// every error arm so callers keep diagnostics across the boundary.
    #[test]
    fn map_spend_result_pins_retry_relevant_codes() {
        let unconfirmed: Result<(), PlatformWalletError> =
            Err(PlatformWalletError::ShieldedSpendUnconfirmed {
                operation: "unshield",
                reason: "transient proof fetch failed".to_string(),
            });
        let result = map_spend_result(unconfirmed, "shielded unshield");
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedSpendUnconfirmed
        );
        assert!(
            message_of(&result).contains("transient proof fetch failed"),
            "unconfirmed message must carry the wallet Display payload"
        );

        let failed: Result<(), PlatformWalletError> = Err(
            PlatformWalletError::ShieldedBroadcastFailed("relay rejected".to_string()),
        );
        let result = map_spend_result(failed, "shielded transfer");
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedBroadcastFailed
        );
        assert!(
            message_of(&result).contains("relay rejected"),
            "broadcast-failed message must carry the wallet Display payload"
        );

        // No Platform-recorded anchor yet → its own retryable code, distinct
        // from the "was broadcast, do NOT retry" unconfirmed code above.
        let no_anchor: Result<(), PlatformWalletError> = Err(
            PlatformWalletError::ShieldedNoRecordedAnchor("mid-block".to_string()),
        );
        let result = map_spend_result(no_anchor, "shielded withdraw");
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedNoRecordedAnchor
        );
        assert!(
            message_of(&result).contains("try again shortly"),
            "no-recorded-anchor message must be the retryable guidance"
        );

        let other: Result<(), PlatformWalletError> =
            Err(PlatformWalletError::ShieldedNoUnspentNotes);
        let result = map_spend_result(other, "shielded withdraw");
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );

        assert_eq!(
            map_spend_result(Ok(()), "shielded transfer").code,
            PlatformWalletFFIResultCode::Success
        );
    }

    /// A shield (Type 15) definitively rejected on an address-nonce race must
    /// map to the dedicated `ErrorAddressNonceMismatch` — NOT regress to the
    /// generic `ErrorWalletOperation` — so hosts keep the safe-to-retry signal.
    /// The submitted/expected nonce values must survive in the message.
    #[test]
    fn map_spend_result_maps_address_nonce_mismatch_to_dedicated_code() {
        let mismatch: Result<(), PlatformWalletError> =
            Err(PlatformWalletError::AddressNonceMismatch {
                address: PlatformAddress::P2pkh([7u8; 20]),
                provided_nonce: 1,
                expected_nonce: 2,
            });
        let result = map_spend_result(mismatch, "shielded shield");
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorAddressNonceMismatch,
            "shield nonce rejection must not regress to ErrorWalletOperation"
        );
        let msg = message_of(&result);
        // Pin the EXACT rendered substrings, not bare digits, so a
        // provided/expected transposition would fail the test.
        assert!(
            msg.contains("submitted nonce 1"),
            "submitted (provided) nonce must render exactly: {msg}"
        );
        assert!(
            msg.contains("Platform expected 2"),
            "expected nonce must render exactly: {msg}"
        );
    }

    #[test]
    fn map_spend_result_maps_shield_capacity_race_to_dedicated_code() {
        let shield_result = map_spend_result(
            Err(PlatformWalletError::PlatformShieldCapacityExceeded {
                available: 3_623_849_220,
                required: 3_623_849_221,
            }),
            "shielded shield",
        );

        assert_eq!(
            shield_result.code,
            PlatformWalletFFIResultCode::ErrorShieldedInsufficientBalance
        );
        let message = message_of(&shield_result);
        assert!(message.contains("available 3623849220"));
        assert!(message.contains("required 3623849221"));

        let transfer_result = map_spend_result(
            Err(PlatformWalletError::ShieldedInsufficientBalance {
                available: 3_623_849_220,
                required: 3_623_849_221,
            }),
            "shielded transfer",
        );

        assert_eq!(
            transfer_result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "the dedicated code is a Platform-to-shielded contract only"
        );
    }

    #[test]
    fn map_asset_lock_funding_result_preserves_typed_funding_codes() {
        let out_point = dashcore::OutPoint {
            txid: dashcore::Txid::all_zeros(),
            vout: 7,
        };
        let result = map_asset_lock_funding_result(
            Err(PlatformWalletError::AssetLockAlreadyConsumed(out_point)),
            "shielded fund-from-asset-lock",
        );
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorAssetLockAlreadyConsumed
        );
        assert!(message_of(&result).contains("Platform completion is unconfirmed"));

        let unrelated = map_asset_lock_funding_result(
            Err(PlatformWalletError::ShieldedNoUnspentNotes),
            "shielded fund-from-asset-lock",
        );
        assert_eq!(
            unrelated.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );

        assert_eq!(
            map_asset_lock_funding_result(Ok(()), "shielded fund-from-asset-lock").code,
            PlatformWalletFFIResultCode::Success
        );
    }

    /// The one-time-key claim entry point must let every typed retry-semantics
    /// code through — not just the terminal one.
    ///
    /// `ShieldedForeignScanBudgetExhausted` has a blanket conversion to code 44
    /// (`ErrorShieldedScanBudgetExhausted`), which Kotlin maps to the RETRYABLE
    /// `ShieldedScanBudgetExhausted`. This entry point used to reach it only via
    /// the catch-all, flattening it to `ErrorWalletOperation` (6) — a
    /// non-retryable generic — so the host rendered a paused scan as a failed
    /// claim and stranded a funded invitation whose note sits deep in the tree.
    /// The polarity is the whole contract, so it is pinned here at the boundary
    /// the host actually calls, not only at the blanket conversion.
    #[test]
    fn map_one_time_claim_result_pins_the_retryable_scan_budget_code() {
        let (identity_id, result) = map_one_time_claim_result(Err(
            PlatformWalletError::ShieldedForeignScanBudgetExhausted {
                scanned_through: 262_144,
            },
        ));
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedScanBudgetExhausted,
            "a budget-paused claim scan must reach the host as 44, never as the \
             generic ErrorWalletOperation (6)"
        );
        assert!(
            identity_id.is_none(),
            "nothing was built or broadcast, so no identity id may be written"
        );
        assert!(
            message_of(&result).contains("262144"),
            "the checkpointed scan position must survive in the message"
        );
    }

    /// The neighbours of the arm above, pinned in the same test so a future
    /// edit cannot silently re-flatten one of them.
    #[test]
    fn map_one_time_claim_result_pins_the_terminal_and_unconfirmed_codes() {
        let claimed =
            map_one_time_claim_result(Err(PlatformWalletError::ShieldedInviteAlreadyClaimed {
                reason: "nullifier already spent".to_string(),
            }));
        assert_eq!(
            claimed.1.code,
            PlatformWalletFFIResultCode::ErrorShieldedInviteAlreadyClaimed
        );
        assert!(
            claimed.0.is_none(),
            "a consumed invitation must NOT write an identity id — that is the \
             false-ownership claim code 43 exists to prevent"
        );

        // The one code that DOES write `out_identity_id`.
        let expected = Identifier::from([7u8; 32]);
        let unconfirmed =
            map_one_time_claim_result(Err(PlatformWalletError::ShieldedBroadcastUnconfirmed {
                identity_id: expected,
                reason: "result proof fetch failed".to_string(),
            }));
        assert_eq!(
            unconfirmed.1.code,
            PlatformWalletFFIResultCode::ErrorShieldedBroadcastUnconfirmed
        );
        assert_eq!(
            unconfirmed.0,
            Some(expected),
            "the unconfirmed code must hand back the derived id so the host can hold the slot"
        );

        // Anything without a typed code still flattens, deliberately.
        let generic = map_one_time_claim_result(Err(PlatformWalletError::ShieldedNoUnspentNotes)).1;
        assert_eq!(
            generic.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
    }

    /// The asset-lock coin-selection shortfall must reach hosts as the
    /// dedicated `ErrorAssetLockInsufficientFunds` (29) through THIS entry
    /// point — `platform_wallet_manager_shielded_fund_from_asset_lock`, the
    /// exact-amount funding form, whose whole result path is this helper.
    /// The blanket `From` impl has always produced 29
    /// (`error::tests::asset_lock_insufficient_funds_maps_to_dedicated_code`),
    /// but the helper's catch-all used to flatten the variant to
    /// `ErrorWalletOperation` (6) before it ever got there, so the typed code
    /// never actually crossed the boundary on this call. Kotlin already
    /// mirrors 29 as
    /// `DashSdkError.PlatformWallet.AssetLockInsufficientFunds`
    /// (`DashSdkError.kt`) — this pins the Rust side that feeds it.
    #[test]
    fn map_asset_lock_funding_result_preserves_shortfall_code_29() {
        let err = PlatformWalletError::AssetLockInsufficientFunds {
            available: 18_000_000,
            required: 100_000_000,
        };
        let rendered = err.to_string();
        let result = map_asset_lock_funding_result(Err(err), "shielded fund-from-asset-lock");

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorAssetLockInsufficientFunds,
            "must not flatten into the generic ErrorWalletOperation catch-all \
             (rendered: {rendered})"
        );
        assert_ne!(
            result.code as i32,
            PlatformWalletFFIResultCode::ErrorWalletOperation as i32
        );
        // The number, not the name, is what Swift/Kotlin mirror by hand.
        assert_eq!(result.code as i32, 29);

        // The structured available/required duffs have no out-params, so they
        // only survive if the arm hands the typed error to the blanket impl
        // verbatim instead of re-wrapping it behind an operation prefix.
        assert_eq!(
            message_of(&result),
            rendered,
            "structured available/required duffs must cross the boundary verbatim"
        );
        assert!(message_of(&result).contains("asset lock coin selection is short"));
    }

    /// The resume sibling shares the helper, so the shortfall stays typed on
    /// `platform_wallet_manager_shielded_resume_fund_from_asset_lock` too — a
    /// host must not have to classify the same failure two different ways
    /// depending on which funding entry point it came in through.
    #[test]
    fn map_asset_lock_funding_result_shortfall_is_typed_on_resume_too() {
        let result = map_asset_lock_funding_result(
            Err(PlatformWalletError::AssetLockInsufficientFunds {
                available: 0,
                required: 100_000_000,
            }),
            "shielded resume fund-from-asset-lock",
        );
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorAssetLockInsufficientFunds
        );
    }
}

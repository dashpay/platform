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
use dpp::shielded::{
    compute_minimum_shielded_fee, compute_shielded_unshield_fee, compute_shielded_withdrawal_fee,
    ShieldedMemo,
};
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_wallet::wallet::asset_lock::AssetLockFunding;
use platform_wallet::wallet::shielded::CachedOrchardProver;
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
/// build (a single-note spend with change is 2 actions). The version is
/// pinned to [`PlatformVersion::latest()`] — the same version the shielded
/// builders in `platform-wallet` resolve via `sdk.version()`, so the
/// estimate can't drift from the fee the builder carves and the consensus
/// gate validates.
///
/// Pure computation: no wallet handle, no network. Writes the fee to
/// `out_fee` and returns `ok()`. An unknown `kind` returns
/// `ErrorInvalidParameter`; a fee-formula overflow returns
/// `ErrorArithmeticOverflow`.
///
/// # Safety
/// `out_fee` must point to 8 writable bytes (a `u64`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_shielded_estimate_fee(
    kind: u8,
    num_actions: usize,
    out_fee: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_fee);

    let platform_version = dpp::version::PlatformVersion::latest();
    let fee = match kind {
        0 => compute_minimum_shielded_fee(num_actions, platform_version),
        1 => compute_shielded_unshield_fee(num_actions, platform_version),
        2 => compute_shielded_withdrawal_fee(num_actions, platform_version),
        other => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("unknown shielded fee kind {other} (expected 0/1/2)"),
            );
        }
    };
    match fee {
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

/// Preserve the typed "already consumed" funding report across the FFI
/// boundary while keeping every other funding failure on the existing generic
/// error path. The wallet retains nonterminal consumption-unknown state; the
/// host must not interpret this code as authenticated completion.
///
/// The double-spend verdicts ride the same typed conversion (both the
/// fresh-build and resume entry points funnel through here, and the resume is
/// where the pre-broadcast conflict screen actually fires).
/// `ErrorAssetLockInputConflict` (42) is the only code that authorises a host
/// to discard a tracked lock, and `ErrorAssetLockInputContested` (43) is its
/// provisional keep-and-retry sibling; flattening either to
/// `ErrorWalletOperation` would strand the user on a lock the host cannot
/// classify.
fn map_asset_lock_funding_result(
    result: Result<(), PlatformWalletError>,
    operation: &str,
) -> PlatformWalletFFIResult {
    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(e @ PlatformWalletError::AssetLockAlreadyConsumed(_)) => e.into(),
        Err(
            e @ (PlatformWalletError::AssetLockInputConflict { .. }
            | PlatformWalletError::AssetLockInputContested { .. }),
        ) => e.into(),
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

    /// Pin the fee estimator to the on-chain ground-truth values observed at the current platform
    /// version with 2 actions (single-note spend + change). These are the exact credits the
    /// builder carves and the consensus gate validates, so the host's "Estimated Fee" must match.
    #[test]
    fn estimate_fee_matches_observed_onchain_values_for_2_actions() {
        unsafe {
            let estimate = |kind: u8| {
                let mut fee: u64 = 0;
                let result = platform_wallet_shielded_estimate_fee(kind, 2, &mut fee);
                assert_eq!(
                    result.code,
                    PlatformWalletFFIResultCode::Success,
                    "kind {kind} must succeed"
                );
                fee
            };
            // kind 0 — ShieldedTransfer / Shield base.
            assert_eq!(
                estimate(0),
                162_851_200,
                "shielded transfer fee (2 actions)"
            );
            // kind 1 — Unshield.
            assert_eq!(estimate(1), 168_934_000, "unshield fee (2 actions)");
            // kind 2 — ShieldedWithdrawal.
            assert_eq!(
                estimate(2),
                275_191_200,
                "shielded withdrawal fee (2 actions)"
            );
        }
    }

    #[test]
    fn estimate_fee_rejects_unknown_kind() {
        unsafe {
            let mut fee: u64 = 0;
            let result = platform_wallet_shielded_estimate_fee(7, 2, &mut fee);
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
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

    /// The two terminal asset-lock verdicts keep their own codes through
    /// this wrapper — both funding entry points (fresh build and resume)
    /// flatten everything else to `ErrorWalletOperation`, and a host that
    /// saw the flattened code could neither hold the consumption-unknown
    /// state nor offer to discard a lock that can never confirm.
    #[test]
    fn map_asset_lock_funding_result_preserves_terminal_asset_lock_codes() {
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

        // The resume endpoint is where the pre-broadcast conflict screen
        // fires, and it funnels through this same wrapper.
        let conflict = map_asset_lock_funding_result(
            Err(PlatformWalletError::AssetLockInputConflict {
                out_point,
                input: dashcore::OutPoint {
                    txid: dashcore::Txid::all_zeros(),
                    vout: 3,
                },
                spent_by: dashcore::Txid::all_zeros(),
                height: Some(1_234),
            }),
            "shielded resume fund-from-asset-lock",
        );
        assert_eq!(
            conflict.code,
            PlatformWalletFFIResultCode::ErrorAssetLockInputConflict
        );
        let conflict_message = message_of(&conflict);
        assert!(
            conflict_message.contains("can never confirm"),
            "the typed Display must survive the wrapper: {conflict_message}"
        );
        assert!(
            conflict_message.contains("chainlocked: true"),
            "the spender's finality must reach the host: {conflict_message}"
        );

        // The provisional sibling rides the same wrapper under its own code:
        // a merely-in-block spender stops the wait but must not surface as
        // the terminal, discard-licensing 42.
        let contested = map_asset_lock_funding_result(
            Err(PlatformWalletError::AssetLockInputContested {
                out_point,
                input: dashcore::OutPoint {
                    txid: dashcore::Txid::all_zeros(),
                    vout: 3,
                },
                spent_by: dashcore::Txid::all_zeros(),
                height: Some(1_234),
            }),
            "shielded resume fund-from-asset-lock",
        );
        assert_eq!(
            contested.code,
            PlatformWalletFFIResultCode::ErrorAssetLockInputContested
        );
        assert!(
            message_of(&contested).contains("provisional"),
            "the contested Display must say the verdict is provisional"
        );

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
}

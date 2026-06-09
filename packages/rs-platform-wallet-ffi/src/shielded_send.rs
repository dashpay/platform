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
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_wallet::wallet::asset_lock::AssetLockFunding;
use platform_wallet::wallet::shielded::CachedOrchardProver;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::{decode_identity_pubkeys, IdentityPubkeyFFI};
use crate::runtime::{block_on_worker, runtime};

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

/// Send a shielded → shielded transfer.
///
/// Spends notes from `wallet_id`'s shielded balance and creates a
/// new note for `recipient_raw_43`. `amount` is in credits
/// (1 DASH = 1e11 credits). Errors if the wallet has no bound
/// shielded sub-wallet, no spendable notes, or insufficient
/// shielded balance to cover `amount + estimated_fee`.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `recipient_raw_43` must point to 43 readable bytes (the
///   recipient's raw Orchard payment address — same shape
///   `platform_wallet_manager_shielded_default_address` returns).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_transfer(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account: u32,
    recipient_raw_43: *const u8,
    amount: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(recipient_raw_43);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let mut recipient = [0u8; 43];
    std::ptr::copy_nonoverlapping(recipient_raw_43, recipient.as_mut_ptr(), 43);

    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(p) => p,
        Err(result) => return result,
    };

    // Run the proof on a worker thread (8 MB stack). Halo 2 circuit
    // synthesis recurses past the ~512 KB iOS dispatch-thread stack
    // and crashes with EXC_BAD_ACCESS at the first
    // `synthesize(... measure(pass))` call when polled on the
    // calling thread.
    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_transfer_to(&coordinator, account, &recipient, amount, &prover)
            .await
    });
    if let Err(e) = result {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded transfer failed: {e}"),
        );
    }
    PlatformWalletFFIResult::ok()
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
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `to_platform_addr_cstr` must be a valid NUL-terminated UTF-8
///   C string for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_unshield(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account: u32,
    to_platform_addr_cstr: *const c_char,
    amount: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
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

    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_unshield_to(&coordinator, account, &to_addr_str, amount, &prover)
            .await
    });
    if let Err(e) = result {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded unshield failed: {e}"),
        );
    }
    PlatformWalletFFIResult::ok()
}

/// Withdraw: spend shielded notes and send `amount` credits to a
/// Core L1 address. `to_core_address_cstr` is the address as a
/// Base58Check NUL-terminated UTF-8 string (e.g.
/// `"yL...."` on testnet); the Rust side parses it and verifies
/// the network matches the wallet's. `core_fee_per_byte` is the
/// L1 fee rate in duffs/byte (`1` is the dashmate default).
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `to_core_address_cstr` must be a valid NUL-terminated UTF-8
///   C string for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_withdraw(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account: u32,
    to_core_address_cstr: *const c_char,
    amount: u64,
    core_fee_per_byte: u32,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
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

    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_withdraw_to(
                &coordinator,
                account,
                &to_core,
                amount,
                core_fee_per_byte,
                &prover,
            )
            .await
    });
    if let Err(e) = result {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded withdraw failed: {e}"),
        );
    }
    PlatformWalletFFIResult::ok()
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
/// On success the 32-byte new identity id (`double_sha256(sorted nullifiers)`) is written to
/// `out_identity_id`. The id is deterministic in the spent notes, so the host can also predict it
/// independently if needed.
///
/// `send_to_address_on_creation_failure_bytes` is the REQUIRED fallback platform address, supplied
/// as raw `PlatformAddress` storage bytes (21 bytes: 1-byte variant tag + 20-byte hash — the
/// encoding `PlatformAddress::to_bytes()` produces and `PlatformAddressWasm`/the Swift wrapper
/// expose). If identity creation fails a stateful check (a public-key hash already registered to
/// another identity) the spend is still finalized and the value is credited to this address minus a
/// penalty, exactly like the asset-lock / address-funded identity-create penalties. It is bound into
/// the transition sighash, so it cannot be redirected after signing.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `identity_pubkeys` must point to `identity_pubkeys_count` contiguous [`IdentityPubkeyFFI`]
///   rows that outlive this call (each row's pointers per the [`IdentityPubkeyFFI`] contract).
/// - `send_to_address_on_creation_failure_bytes` must point to exactly 21 readable bytes for the
///   duration of this call.
/// - `signer_identity_handle` must be a valid, non-destroyed `*mut SignerHandle` (a
///   `VTableSigner` with the callback variant) that outlives this call; the caller retains
///   ownership.
/// - `out_identity_id` must point to 32 writable bytes.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_shielded_identity_create_from_pool(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account: u32,
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
    denomination: u64,
    send_to_address_on_creation_failure_bytes: *const u8,
    signer_identity_handle: *mut SignerHandle,
    out_identity_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
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
        wallet
            .shielded_identity_create_from_pool(
                &coordinator,
                account,
                public_keys,
                denomination,
                send_to_address_on_creation_failure,
                identity_signer,
                &prover,
            )
            .await
    });

    match result {
        Ok(identity_id) => {
            *out_identity_id = identity_id.to_buffer();
            PlatformWalletFFIResult::ok()
        }
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded identity-create-from-pool failed: {e}"),
        ),
    }
}

/// Shield: spend credits from a Platform Payment account into
/// the bound shielded sub-wallet's pool.
///
/// `shielded_account` selects which ZIP-32 Orchard account on
/// the bound shielded sub-wallet receives the new note.
/// `payment_account` selects which Platform Payment account on
/// the transparent side funds the shield (auto-selects input
/// addresses in ascending derivation order until the cumulative
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

    let wallet = match resolve_wallet(handle, &wallet_id) {
        Ok(w) => w,
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
                shielded_account,
                payment_account,
                amount,
                address_signer,
                &prover,
            )
            .await
    });
    if let Err(e) = result {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded shield failed: {e}"),
        );
    }
    PlatformWalletFFIResult::ok()
}

/// Fund the shielded pool from a Core L1 asset lock, orchestrated
/// through the wallet's `AssetLockManager` (build → IS-or-CL →
/// submit → consume). The asset-lock-proof signature is produced
/// by a `MnemonicResolverHandle` — the raw key never crosses the
/// FFI boundary.
///
/// `account_index` selects the BIP44 Core account whose UTXOs
/// fund the asset lock. `amount_duffs` is the L1 amount to lock.
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

    let wallet = match resolve_wallet(handle, &wallet_id) {
        Ok(w) => w,
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
                AssetLockFunding::FromWalletBalance {
                    amount_duffs,
                    account_index,
                },
                vec![(recipient, None)],
                &asset_lock_signer,
                &prover,
                surplus_output,
                None,
            )
            .await
    });
    if let Err(e) = result {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded fund-from-asset-lock failed: {e}"),
        );
    }
    PlatformWalletFFIResult::ok()
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

    let wallet = match resolve_wallet(handle, &wallet_id) {
        Ok(w) => w,
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
                AssetLockFunding::FromExistingAssetLock {
                    out_point: resume_outpoint,
                },
                vec![(recipient, None)],
                &asset_lock_signer,
                &prover,
                surplus_output,
                None,
            )
            .await
    });
    if let Err(e) = result {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded resume fund-from-asset-lock failed: {e}"),
        );
    }
    PlatformWalletFFIResult::ok()
}

/// Resolve the wallet `Arc` for the given manager handle, or
/// produce a `PlatformWalletFFIResult` describing why we couldn't.
fn resolve_wallet(
    handle: Handle,
    wallet_id: &[u8; 32],
) -> Result<std::sync::Arc<platform_wallet::PlatformWallet>, PlatformWalletFFIResult> {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.get_wallet(wallet_id))
    });
    let inner_option = match option {
        Some(v) => v,
        None => {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                format!("invalid manager handle: {handle}"),
            ));
        }
    };
    inner_option.ok_or_else(|| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("wallet not found: {}", hex::encode(wallet_id)),
        )
    })
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

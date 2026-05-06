//! FFI bindings for the shielded spend pipeline (transitions
//! 15/16/17/19 — shield, transfer, unshield, withdraw).
//!
//! Transitions 16/17/19 sign with the bound shielded wallet's
//! Orchard `SpendAuthorizingKey`, which lives on the
//! `OrchardKeySet` cached after [`platform_wallet_manager_bind_shielded`].
//! No host-side `Signer<PlatformAddress>` is required — the host
//! only supplies the recipient + amount (+ core fee rate for
//! withdrawal) and the resulting Halo 2 proof + state transition
//! is built and broadcast on the Rust side.
//!
//! Transition 15 (`shield` — Platform→Shielded) additionally
//! takes a host-supplied `Signer<PlatformAddress>` because the
//! input addresses' ECDSA signatures live in the host keychain.
//! Per-input nonces are fetched from Platform inside
//! [`ShieldedWallet::shield`] before building.
//!
//! Type 18 (`shield_from_asset_lock` — Core L1→Shielded) lives on
//! [`ShieldedWallet`] but isn't wired here yet — it needs the
//! asset-lock proof + private key threaded through.
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

use platform_wallet::wallet::shielded::CachedOrchardProver;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::{block_on_worker, runtime};

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
    recipient_raw_43: *const u8,
    amount: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(recipient_raw_43);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let mut recipient = [0u8; 43];
    std::ptr::copy_nonoverlapping(recipient_raw_43, recipient.as_mut_ptr(), 43);

    let wallet = match resolve_wallet(handle, &wallet_id) {
        Ok(w) => w,
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
            .shielded_transfer_to(&recipient, amount, &prover)
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

    let wallet = match resolve_wallet(handle, &wallet_id) {
        Ok(w) => w,
        Err(result) => return result,
    };

    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_unshield_to(&to_addr_str, amount, &prover)
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

    let wallet = match resolve_wallet(handle, &wallet_id) {
        Ok(w) => w,
        Err(result) => return result,
    };

    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_withdraw_to(&to_core, amount, core_fee_per_byte, &prover)
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

/// Shield: spend credits from a Platform Payment account into
/// the bound shielded sub-wallet's pool. `account_index` selects
/// which Platform Payment account to draw from; the wallet
/// auto-selects input addresses in ascending derivation order
/// until the cumulative balance covers `amount + fee buffer`.
///
/// `signer_address_handle` is a `*mut SignerHandle` produced by
/// `dash_sdk_signer_create_with_ctx` (typically Swift's
/// `KeychainSigner.handle`) — same shape
/// `platform_address_wallet_transfer` expects. The caller retains
/// ownership; this function does not destroy the handle.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `signer_address_handle` must be a valid, non-destroyed
///   `*mut SignerHandle` that outlives this call and points at a
///   `VTableSigner` with the callback variant (the native variant
///   doesn't satisfy `Signer<PlatformAddress>`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_shield(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account_index: u32,
    amount: u64,
    signer_address_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(signer_address_handle);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    let wallet = match resolve_wallet(handle, &wallet_id) {
        Ok(w) => w,
        Err(result) => return result,
    };

    // SAFETY: the caller retains ownership of the signer handle
    // and guarantees it outlives this call. We block until the
    // worker future completes, so the `'static` lifetime we paint
    // on the borrow does not actually outlive the host's handle.
    // `VTableSigner` is `Send + Sync` per its `unsafe impl` in
    // rs-sdk-ffi, so `&'static VTableSigner` is automatically
    // `Send + 'static` — exactly what `block_on_worker` needs.
    let address_signer: &'static VTableSigner =
        std::mem::transmute::<&VTableSigner, &'static VTableSigner>(
            &*(signer_address_handle as *const VTableSigner),
        );

    // Run the proof on a worker thread (8 MB stack). Halo 2 circuit
    // synthesis recurses past the ~512 KB iOS dispatch-thread stack
    // and crashes with EXC_BAD_ACCESS at the first
    // `synthesize(... measure(pass))` call when polled on the
    // calling thread.
    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        wallet
            .shielded_shield_from_account(account_index, amount, address_signer, &prover)
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

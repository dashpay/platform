//! FFI bindings for the shielded spend pipeline (transitions
//! 16/17/19 — transfer, unshield, withdraw).
//!
//! These three transitions sign with the bound shielded wallet's
//! Orchard `SpendAuthorizingKey`, which lives on the
//! `OrchardKeySet` cached after [`platform_wallet_manager_bind_shielded`].
//! No host-side `Signer<PlatformAddress>` is required — the host
//! only supplies the recipient + amount (+ core fee rate for
//! withdrawal) and the resulting Halo 2 proof + state transition
//! is built and broadcast on the Rust side.
//!
//! The fourth transition (Type 15 `shield` — Platform→Shielded)
//! and Type 18 (`shield_from_asset_lock` — Core L1→Shielded) live
//! elsewhere in `platform-wallet`'s [`ShieldedWallet`] surface but
//! aren't wired here yet — they need a host-supplied
//! `Signer<PlatformAddress>` (or asset-lock proof + private key)
//! plus per-input nonce fetching that the Rust spend builder
//! today stubs to zero.
//!
//! Feature-gated behind `shielded`. The accompanying
//! [`platform_wallet_shielded_warm_up_prover`] entry-point is
//! also defined here so hosts can pre-build the Halo 2 proving
//! key on a background thread at app startup.
//!
//! [`ShieldedWallet`]: platform_wallet::wallet::shielded::ShieldedWallet

use std::ffi::CStr;
use std::os::raw::c_char;

use platform_wallet::wallet::shielded::CachedOrchardProver;

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;

/// Build the Halo 2 proving key now if it hasn't been built yet.
///
/// First-call latency is ~30 seconds; subsequent calls return
/// immediately. Hosts should fire this on a background thread at
/// app startup so the first shielded send doesn't block the user.
/// Safe to call repeatedly and from any thread.
///
/// Independent of any manager — the cache is a process-global
/// `OnceLock`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_shielded_warm_up_prover() {
    CachedOrchardProver::new().warm_up();
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
    let prover = CachedOrchardProver::new();
    let prover_ref: &CachedOrchardProver = &prover;

    if let Err(e) = runtime().block_on(wallet.shielded_transfer_to(&recipient, amount, prover_ref))
    {
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
/// `to_platform_addr_bytes` is the bincode-encoded
/// `PlatformAddress` — `0x00 ‖ 20-byte hash` for P2PKH,
/// `0x01 ‖ 20-byte hash` for P2SH. `to_platform_addr_len` is
/// typically 21.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `to_platform_addr_bytes` must point to `to_platform_addr_len`
///   readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_unshield(
    handle: Handle,
    wallet_id_bytes: *const u8,
    to_platform_addr_bytes: *const u8,
    to_platform_addr_len: usize,
    amount: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(to_platform_addr_bytes);
    if to_platform_addr_len == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "to_platform_addr_len must be > 0",
        );
    }

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let to_addr = std::slice::from_raw_parts(to_platform_addr_bytes, to_platform_addr_len).to_vec();

    let wallet = match resolve_wallet(handle, &wallet_id) {
        Ok(w) => w,
        Err(result) => return result,
    };
    let prover = CachedOrchardProver::new();
    let prover_ref: &CachedOrchardProver = &prover;

    if let Err(e) = runtime().block_on(wallet.shielded_unshield_to(&to_addr, amount, prover_ref)) {
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
    let prover = CachedOrchardProver::new();
    let prover_ref: &CachedOrchardProver = &prover;

    if let Err(e) = runtime().block_on(wallet.shielded_withdraw_to(
        &to_core,
        amount,
        core_fee_per_byte,
        prover_ref,
    )) {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded withdraw failed: {e}"),
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

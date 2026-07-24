//! FFI bindings for the deferred build → broadcast/release core-send lifecycle
//! (BIP70 / BIP270 "sign now, submit on merchant ack").
//!
//! The one-shot [`core_wallet_broadcast_transaction`](super::broadcast) sends a
//! just-built transaction immediately. BIP70-style flows must split that: build
//! and sign now (reserving the funding UTXOs), hand the raw bytes to a merchant
//! server, then broadcast only on ack — or release the reservation on a nack /
//! abandonment. These entry points wrap a single process-global
//! [`SignedPaymentRegistry`] pinned to the production `SpvBroadcaster`; the
//! registry owns the built transaction and its held reservation between build
//! and submission and enforces the lifecycle invariants (no double-broadcast,
//! idempotent release, tokens bound to their originating wallet instance).
//!
//! These are ADDITIVE to the existing `core_wallet_tx_builder_*` /
//! `core_wallet_broadcast_transaction` surface — the immediate send path is
//! unchanged.

use crate::error::*;
use crate::handle::{Handle, CORE_WALLET_STORAGE};
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return};
use once_cell::sync::Lazy;
use platform_wallet::broadcaster::SpvBroadcaster;
use platform_wallet::{ReservationToken, SignedPaymentError, SignedPaymentRegistry};
use std::ffi::CString;
use std::os::raw::c_char;

/// Process-global registry of signed-but-unsent payments, keyed by an opaque
/// [`ReservationToken`]. In-memory only: an app crash between build and
/// broadcast drops the registry entry and the underlying UTXO reservation
/// together, so nothing leaks across a restart.
pub(crate) static SIGNED_PAYMENT_REGISTRY: Lazy<SignedPaymentRegistry<SpvBroadcaster>> =
    Lazy::new(SignedPaymentRegistry::new);

/// Broadcast the payment behind `token` (built earlier via
/// [`core_wallet_signed_payment_finalize`](super::transaction_builder::core_wallet_signed_payment_finalize)),
/// reconciling its UTXO reservation on
/// failure, and consume the token.
///
/// The token is consumed atomically before the send, so a repeated or
/// concurrent broadcast of the same token gets `ErrorReservationTokenConsumed`
/// (28) rather than a second send. `core_handle` must resolve to the same wallet
/// *generation* the token was minted against; a wallet re-created under the same
/// id yields `ErrorReservationWalletMismatch` (29). A token whose reservation
/// may already have aged out of key-wallet's TTL yields
/// `ErrorStaleReservationToken` (27). These three deferred-token failures are
/// distinct codes so a host can message each precisely. Writes `out_txid` (a
/// heap C string freed with `core_wallet_free_address`) on success.
///
/// # Safety
/// `core_handle` must be a valid core-wallet handle; `out_txid` must be writable.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_signed_payment_broadcast(
    core_handle: Handle,
    token: u64,
    out_txid: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_txid);

    let core = unwrap_option_or_return!(CORE_WALLET_STORAGE.with_item(core_handle, |w| w.clone()));

    let result =
        runtime().block_on(SIGNED_PAYMENT_REGISTRY.broadcast(ReservationToken::from(token), &core));

    match result {
        Ok(txid) => {
            let c_txid = match CString::new(txid.to_string()) {
                Ok(s) => s,
                Err(_) => {
                    return PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                        "txid string contained an interior NUL".to_string(),
                    );
                }
            };
            *out_txid = c_txid.into_raw();
            PlatformWalletFFIResult::ok()
        }
        // Split the three deferred-token failures into distinct sibling codes so
        // a host can message each precisely. All are non-retryable-in-place and
        // none touched the network.
        Err(e @ SignedPaymentError::StaleReservationToken(_)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorStaleReservationToken,
            e.to_string(),
        ),
        Err(e @ SignedPaymentError::StaleToken(_)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorReservationTokenConsumed,
            e.to_string(),
        ),
        Err(e @ SignedPaymentError::WalletMismatch(_)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorReservationWalletMismatch,
            e.to_string(),
        ),
        // Preserve the typed underlying wallet error (keeps the ambiguous
        // "may already be on the network" retry semantics intact).
        Err(SignedPaymentError::Broadcast(e)) => PlatformWalletFFIResult::from(e),
    }
}

/// Release the funding reservation behind `token` and drop it — the "payment
/// abandoned / merchant server nacked" arm. Idempotent: releasing an unknown /
/// already-consumed token is a silent success, so it never surfaces
/// `ErrorStaleReservationToken`. Needs no wallet handle: the release acts on the
/// wallet instance the token was minted against.
///
/// # Safety
/// Always safe to call; `token` is a plain value.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_signed_payment_release(token: u64) -> PlatformWalletFFIResult {
    runtime().block_on(SIGNED_PAYMENT_REGISTRY.release(ReservationToken::from(token)));
    PlatformWalletFFIResult::ok()
}

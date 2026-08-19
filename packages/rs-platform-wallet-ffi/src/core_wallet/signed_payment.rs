//! FFI bindings for the deferred build → broadcast/release core-send lifecycle
//! (BIP70 / BIP270 "sign now, submit on merchant ack").
//!
//! The one-shot immediate send path (`core_wallet_tx_builder_finalize` +
//! `core_wallet_broadcast_signed_transaction`) sends a just-built
//! transaction immediately. BIP70-style flows must split that: build and sign
//! now (reserving the funding UTXOs), hand the raw bytes to a merchant server,
//! then broadcast only on ack — or release the reservation on a nack /
//! abandonment. These entry points wrap a single process-global
//! [`SignedPaymentRegistry`] pinned to the production `SpvBroadcaster`; the
//! registry owns the built transaction and its held reservation between build
//! and submission and enforces the lifecycle invariants (no double-broadcast,
//! idempotent release, tokens bound to their originating wallet instance).
//!
//! These are ADDITIVE to the `core_wallet_tx_builder_*` surface — the
//! immediate send path is unchanged.

use crate::error::*;
use crate::handle::{Handle, CORE_WALLET_STORAGE};
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
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

/// Serializes tests that reason about the process-global registry's *contents*.
///
/// [`SIGNED_PAYMENT_REGISTRY`] is one static shared by every test in the binary,
/// and the harness runs tests in parallel threads by default. Any test that
/// captures an `outstanding()` baseline and then asserts a delta against it is
/// therefore racing every other test that mints or consumes a token — the
/// baseline can be captured while a sibling's token is outstanding and compared
/// after that sibling consumed it.
///
/// Tests take this around their whole body. Poisoning is recovered rather than
/// propagated (mirroring `SignedPaymentRegistry`'s own lock): a panic in one
/// test should fail that test, not cascade into every sibling.
#[cfg(test)]
pub(crate) static REGISTRY_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Take [`REGISTRY_TEST_LOCK`], recovering from poisoning.
#[cfg(test)]
pub(crate) fn registry_test_guard() -> std::sync::MutexGuard<'static, ()> {
    REGISTRY_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Broadcast the payment behind `token` (built earlier via
/// [`core_wallet_signed_payment_finalize`](super::transaction_builder::core_wallet_signed_payment_finalize)),
/// reconciling its UTXO reservation on
/// failure, and consume the token.
///
/// The token is consumed atomically before the send, so a repeated or
/// concurrent broadcast of the same token gets `ErrorReservationTokenConsumed`
/// (35) rather than a second send. `core_handle` must resolve to the same wallet
/// *generation* the token was minted against; a wallet re-created under the same
/// id yields `ErrorReservationWalletMismatch` (36). A token whose reservation
/// may already have aged out of key-wallet's TTL yields
/// `ErrorStaleReservationToken` (34). These three deferred-token failures are
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
    // Publish the null sentinel before any fallible step, so an invalid
    // handle / consumed token / mismatch / stale token never leaves the
    // caller's previous output value in place to be mistaken for a txid.
    *out_txid = std::ptr::null_mut();

    let core = unwrap_option_or_return!(CORE_WALLET_STORAGE.with_item(core_handle, |w| w.clone()));

    // `try_block_on`, deliberately NOT a `FromCaughtPanicError` impl on
    // `SignedPaymentError`: its only generic-ish variant is `Broadcast(..)`,
    // whose payload carries the typed retry semantics of a REAL broadcast
    // outcome. A panic must not be dressed up as one — it reaches the host as
    // the generic ErrorWalletOperation with the panic text instead.
    let result = match runtime()
        .try_block_on(SIGNED_PAYMENT_REGISTRY.broadcast(ReservationToken::from(token), &core))
    {
        Ok(result) => result,
        Err(error) => return error.into(),
    };

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
        // The wallet was REMOVED from the manager, so there is no live
        // generation to broadcast through. Reported as the existing `NotFound`
        // (98) rather than a new code: it is exactly the "the thing you named
        // does not exist" case 98 already means, and both hosts already map it.
        // Distinct from `ErrorReservationWalletMismatch` (36), where a DIFFERENT
        // live generation answers to the same id. Did NOT touch the network and
        // is NOT retryable — the wallet is gone.
        Err(e @ SignedPaymentError::WalletRemoved(_)) => {
            PlatformWalletFFIResult::err(PlatformWalletFFIResultCode::NotFound, e.to_string())
        }
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
    // `try_block_on`: releasing IS this entry point's job, so a panic must
    // not come back as a success the host records as "reservation released".
    unwrap_result_or_return!(
        runtime().try_block_on(SIGNED_PAYMENT_REGISTRY.release(ReservationToken::from(token)))
    );
    PlatformWalletFFIResult::ok()
}

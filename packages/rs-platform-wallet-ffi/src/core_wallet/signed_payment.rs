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

    let result =
        runtime().block_on(SIGNED_PAYMENT_REGISTRY.broadcast(ReservationToken::from(token), &core));

    match result {
        Ok(txid) => {
            let c_txid = match CString::new(txid.to_string()) {
                Ok(s) => s,
                Err(_) => {
                    // The broadcast went out; only its txid could not be
                    // handed back.
                    return PlatformWalletFFIResult::handback_failure(
                        PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                        "txid string contained an interior NUL",
                    );
                }
            };
            *out_txid = c_txid.into_raw();
            PlatformWalletFFIResult::ok()
        }
        // Split the deferred-token refusals into distinct sibling codes so a
        // host can message each precisely. None is retryable in place.
        Err(e @ SignedPaymentError::StaleReservationToken(_)) => {
            token_refusal(PlatformWalletFFIResultCode::ErrorStaleReservationToken, &e)
        }
        Err(e @ SignedPaymentError::StaleToken(_)) => token_refusal(
            PlatformWalletFFIResultCode::ErrorReservationTokenConsumed,
            &e,
        ),
        Err(e @ SignedPaymentError::WalletMismatch(_)) => token_refusal(
            PlatformWalletFFIResultCode::ErrorReservationWalletMismatch,
            &e,
        ),
        // The wallet was REMOVED from the manager, so there is no live
        // generation to broadcast through. Reported as the existing `NotFound`
        // (98) rather than a new code: it is exactly the "the thing you named
        // does not exist" case 98 already means, and both hosts already map it.
        // Distinct from `ErrorReservationWalletMismatch` (36), where a DIFFERENT
        // live generation answers to the same id. Did NOT touch the network.
        Err(e @ SignedPaymentError::WalletRemoved(_)) => {
            token_refusal(PlatformWalletFFIResultCode::NotFound, &e)
        }
        // Preserve the typed underlying wallet error (keeps the ambiguous
        // "may already be on the network" retry semantics intact).
        Err(SignedPaymentError::Broadcast(e)) => PlatformWalletFFIResult::from(e),
    }
}

/// A deferred-token refusal under `code`, carrying the class the wallet gives
/// it ([`SignedPaymentError::payment_delivery`] and
/// [`SignedPaymentError::is_retryable`]).
fn token_refusal(
    code: PlatformWalletFFIResultCode,
    error: &SignedPaymentError,
) -> PlatformWalletFFIResult {
    PlatformWalletFFIResult::err(code, error.to_string())
        .with_payment_class((error.payment_delivery(), error.is_retryable()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use platform_wallet::test_support::test_platform_wallet_manager;

    /// A token the registry no longer holds may already have been broadcast,
    /// so the refusal crosses as the wallet classes it: `MaybeSent`, never
    /// retryable, under its own code. Before, it crossed unclassified.
    #[test]
    fn should_class_an_unknown_token_broadcast_as_maybe_sent() {
        let (_manager, core) = runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            let core = wallet.core().clone();
            (manager, core)
        });
        let core_handle = crate::handle::CORE_WALLET_STORAGE.insert(core);
        let mut out_txid: *mut c_char = std::ptr::null_mut();

        let result =
            unsafe { core_wallet_signed_payment_broadcast(core_handle, u64::MAX, &mut out_txid) };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorReservationTokenConsumed
        );
        assert_eq!(
            result.payment_delivery,
            PlatformWalletFFIPaymentDelivery::MaybeSent
        );
        assert!(!result.retryable);
        assert!(out_txid.is_null());
        crate::handle::CORE_WALLET_STORAGE.remove(core_handle);
    }

    /// A payment whose result could not be handed back crosses with the
    /// wallet's class for it: `MaybeSent`, not retryable, naming no lock
    /// (the one helper every binding's hand-back guard uses).
    #[test]
    fn should_class_a_result_that_cannot_be_handed_back_as_maybe_sent() {
        let result = PlatformWalletFFIResult::handback_failure(
            PlatformWalletFFIResultCode::ErrorUtf8Conversion,
            "txid string contained an interior NUL",
        );
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUtf8Conversion
        );
        assert_eq!(
            result.payment_delivery,
            PlatformWalletFFIPaymentDelivery::MaybeSent
        );
        assert!(!result.retryable);
        assert_eq!(result.held_asset_lock.txid, [0; 32]);
    }

    /// The refusals that sent nothing cross as `NotSent`, not retryable, each
    /// under its own code.
    #[test]
    fn should_class_the_refusals_that_sent_nothing_as_not_sent() {
        let token = ReservationToken::from(7);
        for (code, error) in [
            (
                PlatformWalletFFIResultCode::ErrorReservationWalletMismatch,
                SignedPaymentError::WalletMismatch(token),
            ),
            (
                PlatformWalletFFIResultCode::NotFound,
                SignedPaymentError::WalletRemoved(token),
            ),
            (
                PlatformWalletFFIResultCode::ErrorStaleReservationToken,
                SignedPaymentError::StaleReservationToken(token),
            ),
        ] {
            let result = token_refusal(code, &error);
            assert_eq!(result.code, code);
            assert_eq!(
                result.payment_delivery,
                PlatformWalletFFIPaymentDelivery::NotSent,
                "{error:?}"
            );
            assert!(!result.retryable, "{error:?}");
        }
    }
}

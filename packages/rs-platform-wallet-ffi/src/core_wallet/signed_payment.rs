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

use super::transaction_builder::{CoreAccountTypeFFI, FFICoreTransaction};
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

/// Register a built, signed transaction for deferred submission and return a
/// reservation token.
///
/// `core_wallet_tx_builder_build_signed` already reserved the funding UTXOs; the
/// registry takes its own copy of the transaction and holds the reservation
/// (via the captured wallet instance behind `core_handle`) until a later
/// [`core_wallet_signed_payment_broadcast`] or
/// [`core_wallet_signed_payment_release`]. The passed `tx` is NOT consumed — the
/// caller still frees it with `core_wallet_transaction_free`.
///
/// `account_type`/`account_index` identify the funding account handed to
/// `set_funding`, so the reservation can be released on rejection/abandonment.
/// Writes `out_token`, `out_fee` (the build's fee in duffs), `out_txid` (a
/// heap-allocated lowercase-hex C string the caller frees with
/// `core_wallet_free_address`), and `out_bytes_ptr`/`out_bytes_len` (the
/// consensus-serialized transaction bytes, returned in the same call so the
/// caller needs no second native round trip).
///
/// The `out_bytes_ptr` buffer borrows the `FFICoreTransaction`'s own storage —
/// it is valid only until `tx` is freed with `core_wallet_transaction_free`, so
/// the caller must copy the bytes out immediately and must not retain the
/// pointer.
///
/// # Safety
/// `tx` must be a valid, non-freed `FFICoreTransaction`; `core_handle` a valid
/// core-wallet handle; all out-pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_signed_payment_register(
    core_handle: Handle,
    tx: *const FFICoreTransaction,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    out_token: *mut u64,
    out_fee: *mut u64,
    out_txid: *mut *mut c_char,
    out_bytes_ptr: *mut *const u8,
    out_bytes_len: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(tx);
    check_ptr!(out_token);
    check_ptr!(out_fee);
    check_ptr!(out_txid);
    check_ptr!(out_bytes_ptr);
    check_ptr!(out_bytes_len);

    let core = unwrap_option_or_return!(CORE_WALLET_STORAGE.with_item(core_handle, |w| w.clone()));

    let bytes = (*tx).bytes();
    let transaction: dashcore::Transaction = match dashcore::consensus::deserialize(bytes) {
        Ok(t) => t,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorDeserialization,
                format!("failed to deserialize signed transaction: {e}"),
            );
        }
    };
    let txid = transaction.txid();
    let fee = (*tx).fee();

    // Do all fallible/pure marshalling BEFORE the registry insert — that insert
    // mints a token and holds the funding reservation, so a later failure would
    // orphan the reservation with no token to release it. txid hex never
    // contains a NUL, but handle the impossible case anyway.
    let c_txid = match CString::new(txid.to_string()) {
        Ok(s) => s,
        Err(_) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                "txid string contained an interior NUL".to_string(),
            );
        }
    };

    let token = runtime().block_on(SIGNED_PAYMENT_REGISTRY.register(
        core,
        transaction,
        account_type.as_standard_account_type(),
        account_index,
    ));

    *out_token = token;
    *out_fee = fee;
    *out_txid = c_txid.into_raw();
    // Borrowed view into the still-live `tx` buffer; the caller copies it out
    // before freeing `tx` (mirrors the retired `core_wallet_transaction_get_bytes`).
    *out_bytes_ptr = bytes.as_ptr();
    *out_bytes_len = bytes.len();
    PlatformWalletFFIResult::ok()
}

/// Broadcast the payment behind `token` (built earlier via
/// [`core_wallet_signed_payment_register`]), reconciling its UTXO reservation on
/// failure, and consume the token.
///
/// The token is consumed atomically before the send, so a repeated or
/// concurrent broadcast of the same token gets `ErrorStaleReservationToken`
/// rather than a second send. `core_handle` must resolve to the same wallet
/// instance the token was minted against; a re-created wallet yields
/// `ErrorStaleReservationToken`. Writes `out_txid` (a heap C string freed with
/// `core_wallet_free_address`) on success.
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
        runtime().block_on(SIGNED_PAYMENT_REGISTRY.broadcast(token as ReservationToken, &core));

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
        Err(
            e @ (SignedPaymentError::StaleToken(_)
            | SignedPaymentError::WalletMismatch(_)
            | SignedPaymentError::StaleReservationToken(_)),
        ) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorStaleReservationToken,
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
    runtime().block_on(SIGNED_PAYMENT_REGISTRY.release(token as ReservationToken));
    PlatformWalletFFIResult::ok()
}

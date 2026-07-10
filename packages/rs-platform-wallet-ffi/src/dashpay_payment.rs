//! FFI getter for per-contact DashPay payment history.
//!
//! Swift's `ContactDetailView` renders a payment list per contact
//! (`PaymentEntry` on the managed identity's `DashPayState.payments`, keyed by
//! txid). This module exposes that map off an existing
//! [`ManagedIdentity`](platform_wallet::ManagedIdentity) handle (the
//! one the host already obtains via
//! [`crate::platform_wallet_get_managed_identity`]) as a flat array of
//! POD-plus-C-string rows.
//!
//! ## Why a getter, not a persister callback
//!
//! The `dashpay_payments` map is already part of the persisted
//! `ManagedIdentity` state (it round-trips through `IdentityEntry` and
//! the `dashpay_payments_overlay` changeset field), and the FFI already
//! hands the host a live `ManagedIdentity` handle from which DashPay
//! fields are read directly (e.g.
//! [`crate::established_contact_is_payment_channel_broken`]). A
//! getter therefore lands the smaller, lower-risk diff: no new
//! persister callback, no new SwiftData rehydration path. It mirrors the
//! handle-based array-return pattern already used by
//! [`ContactRequestHandleArray`](crate::dashpay::ContactRequestHandleArray)
//! and [`IdentifierArray`](crate::IdentifierArray).
//!
//! ## Ownership
//!
//! Each [`DashpayPaymentFFI`] owns its `txid` and (optional) `memo`
//! C-strings. [`dashpay_payment_array_free`] releases every string
//! across the array and the array backing buffer itself.

use std::os::raw::c_char;

use platform_wallet::wallet::identity::{PaymentDirection, PaymentStatus};

use crate::error::*;
use crate::handle::*;
use crate::{check_ptr, unwrap_option_or_return};

/// Direction of a DashPay payment from the owner's perspective.
/// Matches [`PaymentDirection`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashpayPaymentDirectionFFI {
    /// The owner sent this payment to the counterparty.
    Sent = 0,
    /// The owner received this payment from the counterparty.
    Received = 1,
}

impl From<PaymentDirection> for DashpayPaymentDirectionFFI {
    fn from(d: PaymentDirection) -> Self {
        match d {
            PaymentDirection::Sent => DashpayPaymentDirectionFFI::Sent,
            PaymentDirection::Received => DashpayPaymentDirectionFFI::Received,
        }
    }
}

/// Status of a DashPay payment on Core chain. Matches [`PaymentStatus`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashpayPaymentStatusFFI {
    /// Broadcast but not yet confirmed.
    Pending = 0,
    /// Confirmed on Core chain.
    Confirmed = 1,
    /// Broadcast failed or the transaction was dropped.
    Failed = 2,
}

impl From<PaymentStatus> for DashpayPaymentStatusFFI {
    fn from(s: PaymentStatus) -> Self {
        match s {
            PaymentStatus::Pending => DashpayPaymentStatusFFI::Pending,
            PaymentStatus::Confirmed => DashpayPaymentStatusFFI::Confirmed,
            PaymentStatus::Failed => DashpayPaymentStatusFFI::Failed,
        }
    }
}

/// Flat C mirror of one [`PaymentEntry`](platform_wallet::wallet::identity::PaymentEntry)
/// row on a [`ManagedIdentity`](platform_wallet::ManagedIdentity).
///
/// The `PaymentEntry` value carries no timestamp field (the underlying
/// model keys history by txid and does not record a wall-clock time), so
/// none is exposed here — ordering on the Swift side is by txid /
/// arrival, matching the Rust map. `txid` is the
/// `dashpay_payments` map key, surfaced as a C-string.
#[repr(C)]
// Deliberately NOT Clone/Copy: this struct owns its `txid` / `memo`
// heap pointers (freed by `dashpay_payment_array_free`). A bitwise Copy
// or a derived Clone would duplicate those raw pointers, so freeing both
// the original and the copy double-frees. Build rows in place instead.
#[derive(Debug)]
pub struct DashpayPaymentFFI {
    /// The other identity in this payment (`counterparty_id`). Whether
    /// they are the sender or the receiver is encoded in `direction`.
    pub counterparty_id: [u8; 32],
    /// Amount in duffs. Always positive; `direction` carries the sign.
    pub amount_duffs: u64,
    /// Payment direction from the owner's perspective.
    pub direction: DashpayPaymentDirectionFFI,
    /// Core-chain status.
    pub status: DashpayPaymentStatusFFI,
    /// NUL-terminated transaction id (hex), the `dashpay_payments` map
    /// key. Always non-null. Owned — released by
    /// [`dashpay_payment_array_free`].
    pub txid: *mut c_char,
    /// NUL-terminated sender memo, or null when the source `Option` was
    /// `None`. Owned — released by [`dashpay_payment_array_free`].
    pub memo: *mut c_char,
}

/// Heap-allocated array of [`DashpayPaymentFFI`] rows returned by
/// [`managed_identity_get_dashpay_payments`]; released via
/// [`dashpay_payment_array_free`].
#[repr(C)]
pub struct DashpayPaymentArray {
    pub items: *mut DashpayPaymentFFI,
    pub count: usize,
}

impl DashpayPaymentArray {
    fn empty() -> Self {
        Self {
            items: std::ptr::null_mut(),
            count: 0,
        }
    }
}

/// Convert a `&str` into an owned C-string pointer, or null on an
/// interior NUL. txids are hex and memos are user text — neither should
/// carry a NUL, but a defensive null keeps the FFI total rather than
/// panicking on malformed input.
fn cstring_or_null(s: &str) -> *mut c_char {
    match std::ffi::CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Read the DashPay payment history off a `ManagedIdentity` handle as a
/// flat array, keyed by txid in the underlying map's iteration order
/// (BTreeMap → lexicographic by txid hex).
///
/// On success `*out_array` is populated; an identity with no recorded
/// payments yields an empty array (null `items`, `count == 0`) and still
/// returns `Success`. Release via [`dashpay_payment_array_free`].
///
/// # Safety
/// - `identity_handle` must be a live handle from
///   [`crate::platform_wallet_get_managed_identity`] (or another
///   `MANAGED_IDENTITY_STORAGE` producer).
/// - `out_array` must point at writable `DashpayPaymentArray` storage.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_dashpay_payments(
    identity_handle: Handle,
    out_array: *mut DashpayPaymentArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);
    unsafe { *out_array = DashpayPaymentArray::empty() };

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity
            .dashpay()
            .payments
            .iter()
            .map(|(txid, entry)| DashpayPaymentFFI {
                counterparty_id: entry.counterparty_id.to_buffer(),
                amount_duffs: entry.amount_duffs,
                direction: entry.direction.into(),
                status: entry.status.into(),
                txid: cstring_or_null(txid),
                memo: entry
                    .memo
                    .as_deref()
                    .map(cstring_or_null)
                    .unwrap_or(std::ptr::null_mut()),
            })
            .collect::<Vec<DashpayPaymentFFI>>()
    });
    let rows = unwrap_option_or_return!(option);

    if rows.is_empty() {
        // `*out_array` already holds the empty sentinel.
        return PlatformWalletFFIResult::ok();
    }
    let count = rows.len();
    let boxed = rows.into_boxed_slice();
    let ptr = Box::into_raw(boxed) as *mut DashpayPaymentFFI;
    unsafe { *out_array = DashpayPaymentArray { items: ptr, count } };
    PlatformWalletFFIResult::ok()
}

/// Release an array returned by [`managed_identity_get_dashpay_payments`],
/// including every owned `txid` / `memo` C-string.
///
/// Pointer-only signature (the array is a 16-byte aggregate at the
/// Swift-ABI cliff): pass `&mut array`. Idempotent — fields are reset to
/// the empty sentinel after free so a second call no-ops.
///
/// # Safety
/// `array` must point at a `DashpayPaymentArray` produced by
/// [`managed_identity_get_dashpay_payments`] and not previously freed.
#[no_mangle]
pub unsafe extern "C" fn dashpay_payment_array_free(array: *mut DashpayPaymentArray) {
    if array.is_null() {
        return;
    }
    let array = unsafe { &mut *array };
    if array.items.is_null() || array.count == 0 {
        array.items = std::ptr::null_mut();
        array.count = 0;
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(array.items, array.count) };
    for row in slice.iter_mut() {
        if !row.txid.is_null() {
            let _ = unsafe { std::ffi::CString::from_raw(row.txid) };
            row.txid = std::ptr::null_mut();
        }
        if !row.memo.is_null() {
            let _ = unsafe { std::ffi::CString::from_raw(row.memo) };
            row.memo = std::ptr::null_mut();
        }
    }
    let _ = unsafe { Box::from_raw(slice as *mut [DashpayPaymentFFI]) };
    array.items = std::ptr::null_mut();
    array.count = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;
    use platform_wallet::wallet::identity::PaymentEntry;
    use platform_wallet::ManagedIdentity;

    /// Build a `ManagedIdentity` handle carrying a couple of payments so
    /// the getter has real rows to project. Uses the same
    /// `ManagedIdentity::new(identity, 0)` construction the other
    /// `managed_identity_*` tests use.
    fn managed_identity_with_payments() -> Handle {
        // A minimal valid identity is awkward to build here; the
        // payments map is an open-tier cache, so we mutate it directly
        // on a default-constructed identity via the same path the
        // persister load uses.
        let identity = dpp::identity::Identity::V0(dpp::identity::v0::IdentityV0::default());
        let mut managed = ManagedIdentity::new(identity, 0);
        managed.dashpay_payments_mut().insert(
            "aa".repeat(32),
            PaymentEntry::new_sent(Identifier::from([1u8; 32]), 12_000, Some("lunch".into())),
        );
        managed.dashpay_payments_mut().insert(
            "bb".repeat(32),
            PaymentEntry::new_received(Identifier::from([2u8; 32]), 7_500, None),
        );
        MANAGED_IDENTITY_STORAGE.insert(managed)
    }

    /// The getter must project every `dashpay_payments` row with its
    /// direction/status/amount and the txid map key, surface the memo
    /// (and null it when absent), and the paired free must reclaim every
    /// owned string without a double-free. Pins the per-contact payment
    /// history surface Swift renders in `ContactDetailView`.
    #[test]
    fn get_dashpay_payments_projects_rows_and_frees_clean() {
        let handle = managed_identity_with_payments();

        let mut array = DashpayPaymentArray::empty();
        let r = unsafe { managed_identity_get_dashpay_payments(handle, &mut array) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(array.count, 2);
        assert!(!array.items.is_null());

        let rows = unsafe { std::slice::from_raw_parts(array.items, array.count) };
        // BTreeMap order: "aa…" (sent, lunch) before "bb…" (received).
        let sent = &rows[0];
        assert_eq!(sent.direction, DashpayPaymentDirectionFFI::Sent);
        assert_eq!(sent.status, DashpayPaymentStatusFFI::Pending);
        assert_eq!(sent.amount_duffs, 12_000);
        assert_eq!(sent.counterparty_id, [1u8; 32]);
        assert!(!sent.txid.is_null());
        let txid = unsafe { std::ffi::CStr::from_ptr(sent.txid) }
            .to_str()
            .unwrap();
        assert_eq!(txid, "aa".repeat(32));
        let memo = unsafe { std::ffi::CStr::from_ptr(sent.memo) }
            .to_str()
            .unwrap();
        assert_eq!(memo, "lunch");

        let received = &rows[1];
        assert_eq!(received.direction, DashpayPaymentDirectionFFI::Received);
        assert_eq!(received.status, DashpayPaymentStatusFFI::Confirmed);
        assert_eq!(received.amount_duffs, 7_500);
        // No memo on the received entry → null pointer.
        assert!(received.memo.is_null());

        unsafe { dashpay_payment_array_free(&mut array) };
        assert!(array.items.is_null());
        assert_eq!(array.count, 0);
        // Idempotent — second free must not double-free.
        unsafe { dashpay_payment_array_free(&mut array) };

        let _ = MANAGED_IDENTITY_STORAGE.remove(handle);
    }

    /// An identity with no recorded payments yields an empty array and
    /// still returns `Success` — empty is not an error (matches the
    /// sibling array getters' contract).
    #[test]
    fn get_dashpay_payments_empty_is_success() {
        let identity = dpp::identity::Identity::V0(dpp::identity::v0::IdentityV0::default());
        let managed = ManagedIdentity::new(identity, 0);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        // Non-null sentinel so we can assert the getter resets it to
        // the empty sentinel; the pointer is never dereferenced.
        let mut array = DashpayPaymentArray {
            items: std::ptr::NonNull::<DashpayPaymentFFI>::dangling().as_ptr(),
            count: 99,
        };
        let r = unsafe { managed_identity_get_dashpay_payments(handle, &mut array) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        assert!(array.items.is_null());
        assert_eq!(array.count, 0);

        let _ = MANAGED_IDENTITY_STORAGE.remove(handle);
    }

    /// Unknown handle → `NotFound`, and the out-array is left at the
    /// empty sentinel rather than carrying stale caller-supplied junk.
    #[test]
    fn get_dashpay_payments_unknown_handle_is_not_found() {
        // Non-null sentinel so we can assert the getter resets it to
        // the empty sentinel; the pointer is never dereferenced.
        let mut array = DashpayPaymentArray {
            items: std::ptr::NonNull::<DashpayPaymentFFI>::dangling().as_ptr(),
            count: 99,
        };
        let r = unsafe { managed_identity_get_dashpay_payments(0xDEAD_BEEF, &mut array) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
        // Reset to the empty sentinel before the handle lookup failed.
        assert!(array.items.is_null());
        assert_eq!(array.count, 0);
    }
}

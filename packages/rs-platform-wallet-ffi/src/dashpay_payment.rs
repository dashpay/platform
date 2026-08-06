//! FFI surface for per-contact DashPay payment history: the persister
//! callback's row type ([`DashpayPaymentPersistEntryFFI`]) and an
//! on-demand getter over a live handle.
//!
//! Swift's `ContactDetailView` renders a payment list per contact
//! (`PaymentEntry` on the managed identity's `DashPayState.payments`, keyed by
//! txid). This module exposes that map off an existing
//! [`ManagedIdentity`](platform_wallet::ManagedIdentity) handle (the
//! one the host already obtains via
//! [`crate::platform_wallet_get_managed_identity`]) as a flat array of
//! POD-plus-C-string rows.
//!
//! ## Persistence: the callback is authoritative, the getter reconciles
//!
//! Payment history persists event-driven through
//! `on_persist_dashpay_payments_fn` on the persister vtable, exactly
//! like contact requests and profiles: every `store()` round whose
//! changeset carries payment rows (an `IdentityEntry.dashpay_payments`
//! snapshot from `record_dashpay_payment`, or a merged
//! `dashpay_payments_overlay`) projects them to the host. This closes
//! the write half of the durability loop whose read half — the
//! `payments` array on `IdentityRestoreEntryFFI` — already rehydrates
//! the map at load. (An earlier revision shipped only the
//! [`managed_identity_get_dashpay_payments`] getter, on the rationale
//! that the map "already persists through the changeset" — which was
//! true of the desktop SQLite persister but never of FFI hosts, whose
//! vtable had no payments slot. A host-side `store()` returned Ok while
//! dropping every Sent entry + memo unless the app happened to call the
//! getter-backed refresh path first.)
//!
//! The getter remains as (a) the on-demand read Swift's
//! `refreshDashPayPayments` uses to reconcile persisted rows against
//! live state — belt-and-suspenders over the callback — and (b) the
//! per-contact history read for UI surfaces that want current in-memory
//! state without a persistence round-trip.
//!
//! ## Ownership
//!
//! Each [`DashpayPaymentFFI`] owns its `txid` and (optional) `memo`
//! C-strings. [`dashpay_payment_array_free`] releases every string
//! across the array and the array backing buffer itself.
//! [`DashpayPaymentPersistEntryFFI`] rows are Rust-owned for the
//! duration of the persist callback only (the caller keeps the backing
//! `CString`s alive across the call and drops them after — no paired
//! free function, matching the other persist-direction callbacks).

use std::collections::BTreeMap;
use std::ffi::CString;
use std::os::raw::c_char;

use dpp::prelude::Identifier;
use platform_wallet::wallet::identity::{PaymentDirection, PaymentEntry, PaymentStatus};

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

/// One DashPay payment-history row forwarded to the host by the
/// `on_persist_dashpay_payments_fn` persister callback.
///
/// Field set mirrors the load-side
/// [`PaymentRestoreEntryFFI`](crate::wallet_restore_types::PaymentRestoreEntryFFI)
/// — same raw `u8` direction/status discriminants, same
/// txid/memo C-string shape — plus the leading `owner_identity_id`,
/// because the persist callback is wallet-scoped while the restore
/// rows already ride inside a per-identity buffer. Keeping the write
/// and restore shapes field-for-field means a host handler and its
/// restore assembler agree by construction.
///
/// All pointers are Rust-owned and valid only for the callback window
/// — the host must copy before returning. Persist direction needs no
/// paired free function (Rust drops the backing `CString`s after the
/// call), matching the other `on_persist_*` callbacks.
#[repr(C)]
pub struct DashpayPaymentPersistEntryFFI {
    /// The identity that owns this payment-history row (the
    /// `ManagedIdentity` whose `dashpay_payments` map carries it).
    pub owner_identity_id: [u8; 32],
    /// The other identity in this payment. Whether they are the sender
    /// or the receiver is encoded in `direction_raw`.
    pub counterparty_id: [u8; 32],
    /// Amount in duffs. Always positive; `direction_raw` carries the sign.
    pub amount_duffs: u64,
    /// `PaymentDirection` discriminant: 0=Sent, 1=Received.
    pub direction_raw: u8,
    /// `PaymentStatus` discriminant: 0=Pending, 1=Confirmed, 2=Failed.
    pub status_raw: u8,
    /// NUL-terminated transaction id (hex) — the `dashpay_payments`
    /// map key. Always non-null (rows whose txid cannot form a
    /// C-string are dropped at build time).
    pub txid: *const c_char,
    /// NUL-terminated sender memo, or null when the source `Option`
    /// was `None`.
    pub memo: *const c_char,
}

/// Flatten per-identity payment maps into persist-callback rows.
///
/// Returns the row array plus the `CString` storage backing every
/// `txid` / `memo` pointer — the caller must keep the storage alive
/// until the callback returns. Rows whose txid contains an interior
/// NUL are dropped (unreachable for hex txids; defensive rather than
/// panicking); a memo with an interior NUL degrades to null, matching
/// [`cstring_or_null`]'s contract on the getter side.
pub(crate) fn build_payment_persist_entries(
    payments: &BTreeMap<(Identifier, &str), &PaymentEntry>,
) -> (Vec<DashpayPaymentPersistEntryFFI>, Vec<CString>) {
    let mut storage: Vec<CString> = Vec::new();
    let mut rows: Vec<DashpayPaymentPersistEntryFFI> = Vec::with_capacity(payments.len());
    for ((owner_id, txid), entry) in payments {
        let Ok(txid_c) = CString::new(*txid) else {
            continue;
        };
        storage.push(txid_c);
        let txid_ptr = storage.last().expect("pushed txid CString above").as_ptr();
        let memo_ptr = match entry.memo.as_deref().map(CString::new) {
            Some(Ok(memo_c)) => {
                storage.push(memo_c);
                storage.last().expect("pushed memo CString above").as_ptr()
            }
            _ => std::ptr::null(),
        };
        rows.push(DashpayPaymentPersistEntryFFI {
            owner_identity_id: owner_id.to_buffer(),
            counterparty_id: entry.counterparty_id.to_buffer(),
            amount_duffs: entry.amount_duffs,
            direction_raw: DashpayPaymentDirectionFFI::from(entry.direction) as u8,
            status_raw: DashpayPaymentStatusFFI::from(entry.status) as u8,
            txid: txid_ptr,
            memo: memo_ptr,
        });
    }
    (rows, storage)
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

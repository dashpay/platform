//! FFI types for forwarding
//! [`DpnsNameStateChangeSet`](platform_wallet::changeset::DpnsNameStateChangeSet)
//! — the DPNS username-marketplace rows — out of
//! [`FFIPersister`](crate::persistence::FFIPersister) to the host.
//!
//! Shaped like [`crate::invitation_persistence`], with one difference:
//! [`DpnsNameStateEntry`] carries three owned strings (the display label,
//! its homograph-normalized form, and the normalized parent domain), so
//! each row owns `CString` allocations that MUST be released with
//! [`free_dpns_name_state_entries`] after the callback returns — exactly
//! the allocate/free discipline `IdentityEntryFFI`'s DPNS label arrays
//! use in [`crate::identity_persistence`].
//!
//! The strings are Rust-owned and valid only for the callback window;
//! the host must copy anything it keeps before returning.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use platform_wallet::changeset::{DpnsNameSaleStatus, DpnsNameStateEntry};

/// C mirror of one [`DpnsNameStateEntry`]: a DPNS `domain` document
/// tracked for a wallet identity, with its sale state.
///
/// The three `*const c_char` fields are NUL-terminated UTF-8 owned by
/// this struct for the duration of the persistence callback. Optional
/// values travel as a `has_*` flag plus the value — never as a sentinel,
/// so "not for sale" stays distinguishable from "listed at 0 credits"
/// and "no `$updatedAt`" from "updated at the epoch".
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DpnsNameStateFFI {
    /// The DPNS `domain` document id — this row's key, stable across
    /// transfers and purchases.
    pub document_id: [u8; 32],
    /// The wallet identity this row is tracked for. For `Owned` rows the
    /// document's `$ownerId`; for `Sold`/`Transferred` rows the previous
    /// owner (ours).
    pub wallet_identity_id: [u8; 32],
    /// Whether `counterparty_id` is populated — true exactly when
    /// `status != 0`.
    pub has_counterparty: bool,
    /// The buyer (`status == 1`) or recipient (`status == 2`). Ignore
    /// unless `has_counterparty`.
    pub counterparty_id: [u8; 32],
    /// Display label, e.g. "Alice".
    pub label: *const c_char,
    /// Homograph-normalized label, e.g. "a11ce".
    pub normalized_label: *const c_char,
    /// Normalized parent domain — "dash" today. Carried (rather than
    /// defaulted host-side) because it is part of the host's row
    /// uniqueness key alongside the normalized label.
    pub normalized_parent_domain_name: *const c_char,
    /// Whether `price` is populated. `false` = not listed for sale.
    pub has_price: bool,
    /// Listed sale price in credits (`$price`). Ignore unless `has_price`.
    pub price: u64,
    /// Ownership status relative to `wallet_identity_id`:
    /// `0` = owned, `1` = sold, `2` = transferred.
    pub status: u8,
    /// Document `$createdAt` in ms. `0` = unknown.
    pub created_at_ms: u64,
    /// Document `$updatedAt` in ms. `0` = unknown.
    pub updated_at_ms: u64,
    /// Document `$transferredAt` in ms. `0` = unknown.
    pub transferred_at_ms: u64,
    /// Wall-clock ms of the sync pass / confirmed transition that wrote
    /// this row.
    pub last_synced_at_ms: u64,
}

// Pin the ABI size so a future field reorder/add that changes the layout
// is a compile error rather than a silent desync (the layout-assert
// convention every other `*EntryFFI` follows).
// [u8;32]@0, [u8;32]@32, bool@64, [u8;32]@65..97, 3 ptrs@104..128,
// bool@128, u64@136, u8@144, 4 u64@152..184 → align 8 → size 184.
const _: [u8; 184] = [0u8; std::mem::size_of::<DpnsNameStateFFI>()];

/// Discriminant mapping for [`DpnsNameSaleStatus`], plus the
/// counterparty it carries. Wildcard-free so adding a variant is a
/// compile error rather than a silent mis-map. Pinned by a test.
fn status_and_counterparty(status: &DpnsNameSaleStatus) -> (u8, bool, [u8; 32]) {
    match status {
        DpnsNameSaleStatus::Owned => (0, false, [0u8; 32]),
        DpnsNameSaleStatus::Sold { to } => (1, true, to.to_buffer()),
        DpnsNameSaleStatus::Transferred { to } => (2, true, to.to_buffer()),
    }
}

/// Heap-allocate `s` as an owned C string, or `null` if it contains an
/// interior NUL (unreachable for DPNS-validated labels, but a null is
/// far better than a panic across the boundary). Released by
/// [`free_dpns_name_state_entries`].
fn owned_c_string(s: &str) -> *const c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw() as *const c_char,
        Err(_) => ptr::null(),
    }
}

/// Build the flat FFI rows from the changeset entries.
///
/// Every returned row owns three `CString` allocations — the caller MUST
/// pass the Vec to [`free_dpns_name_state_entries`] once the persistence
/// callback has returned.
pub fn build_dpns_name_state_entries(entries: &[&DpnsNameStateEntry]) -> Vec<DpnsNameStateFFI> {
    entries
        .iter()
        .map(|entry| {
            let (status, has_counterparty, counterparty_id) =
                status_and_counterparty(&entry.status);
            let (has_price, price) = match entry.price {
                Some(p) => (true, p),
                None => (false, 0),
            };
            DpnsNameStateFFI {
                document_id: entry.document_id.to_buffer(),
                wallet_identity_id: entry.wallet_identity_id.to_buffer(),
                has_counterparty,
                counterparty_id,
                label: owned_c_string(&entry.label),
                normalized_label: owned_c_string(&entry.normalized_label),
                normalized_parent_domain_name: owned_c_string(&entry.normalized_parent_domain_name),
                has_price,
                price,
                status,
                created_at_ms: entry.created_at_ms.unwrap_or(0),
                updated_at_ms: entry.updated_at_ms.unwrap_or(0),
                transferred_at_ms: entry.transferred_at_ms.unwrap_or(0),
                last_synced_at_ms: entry.last_synced_at_ms,
            }
        })
        .collect()
}

/// Release the three owned C strings on every row and null the slots.
/// Idempotent — a second call is a no-op.
///
/// # Safety
///
/// Every row must have been produced by [`build_dpns_name_state_entries`]
/// and not previously freed; the pointers must reference allocations
/// owned by these rows.
pub unsafe fn free_dpns_name_state_entries(entries: &mut [DpnsNameStateFFI]) {
    for entry in entries.iter_mut() {
        unsafe {
            free_owned_c_string(&mut entry.label);
            free_owned_c_string(&mut entry.normalized_label);
            free_owned_c_string(&mut entry.normalized_parent_domain_name);
        }
    }
}

/// Release one C string produced by [`owned_c_string`] and null the slot
/// in place, so repeated frees no-op.
///
/// # Safety
/// The pointer must be null or a `CString::into_raw` allocation.
unsafe fn free_owned_c_string(slot: &mut *const c_char) {
    if !slot.is_null() {
        let _ = unsafe { CString::from_raw(*slot as *mut c_char) };
        *slot = ptr::null();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;
    use std::ffi::CStr;

    fn entry(status: DpnsNameSaleStatus, price: Option<u64>) -> DpnsNameStateEntry {
        DpnsNameStateEntry {
            document_id: Identifier::from([1u8; 32]),
            wallet_identity_id: Identifier::from([2u8; 32]),
            label: "Alice".to_string(),
            normalized_label: "a11ce".to_string(),
            normalized_parent_domain_name: "dash".to_string(),
            price,
            status,
            created_at_ms: Some(10),
            updated_at_ms: None,
            transferred_at_ms: Some(30),
            last_synced_at_ms: 99,
        }
    }

    /// The status discriminants are the ABI contract with the host's
    /// mirror; pin all three plus the counterparty they carry.
    #[test]
    fn status_discriminants_are_pinned() {
        let to = Identifier::from([7u8; 32]);
        assert_eq!(
            status_and_counterparty(&DpnsNameSaleStatus::Owned),
            (0, false, [0u8; 32])
        );
        assert_eq!(
            status_and_counterparty(&DpnsNameSaleStatus::Sold { to }),
            (1, true, [7u8; 32])
        );
        assert_eq!(
            status_and_counterparty(&DpnsNameSaleStatus::Transferred { to }),
            (2, true, [7u8; 32])
        );
    }

    #[test]
    fn build_entries_round_trips_every_field() {
        let owned = entry(DpnsNameSaleStatus::Owned, Some(5_000));
        let sold = entry(
            DpnsNameSaleStatus::Sold {
                to: Identifier::from([7u8; 32]),
            },
            None,
        );
        let refs = [&owned, &sold];
        let mut ffi = build_dpns_name_state_entries(&refs);
        assert_eq!(ffi.len(), 2);

        assert_eq!(ffi[0].document_id, [1u8; 32]);
        assert_eq!(ffi[0].wallet_identity_id, [2u8; 32]);
        assert_eq!(ffi[0].status, 0);
        assert!(!ffi[0].has_counterparty);
        assert!(ffi[0].has_price);
        assert_eq!(ffi[0].price, 5_000);
        assert_eq!(ffi[0].created_at_ms, 10);
        // Absent `$updatedAt` must arrive as 0-and-unknown, not fabricated.
        assert_eq!(ffi[0].updated_at_ms, 0);
        assert_eq!(ffi[0].transferred_at_ms, 30);
        assert_eq!(ffi[0].last_synced_at_ms, 99);
        let label = unsafe { CStr::from_ptr(ffi[0].label) }
            .to_string_lossy()
            .into_owned();
        let normalized = unsafe { CStr::from_ptr(ffi[0].normalized_label) }
            .to_string_lossy()
            .into_owned();
        let parent = unsafe { CStr::from_ptr(ffi[0].normalized_parent_domain_name) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(label, "Alice");
        assert_eq!(normalized, "a11ce");
        assert_eq!(parent, "dash");

        assert_eq!(ffi[1].status, 1);
        assert!(ffi[1].has_counterparty);
        assert_eq!(ffi[1].counterparty_id, [7u8; 32]);
        // Not listed: flagged, never rendered as a 0-credit listing.
        assert!(!ffi[1].has_price);
        assert_eq!(ffi[1].price, 0);

        unsafe { free_dpns_name_state_entries(&mut ffi) };
        assert!(ffi[0].label.is_null());
        assert!(ffi[0].normalized_label.is_null());
        assert!(ffi[0].normalized_parent_domain_name.is_null());
        // Idempotent — the dispatcher frees on every path, including the
        // one where the callback returned an error.
        unsafe { free_dpns_name_state_entries(&mut ffi) };
    }

    /// An empty changeset produces an empty Vec, and freeing it is a
    /// no-op — the shape `store()` hits when a round carries only
    /// tombstones.
    #[test]
    fn empty_entries_build_and_free_cleanly() {
        let mut ffi = build_dpns_name_state_entries(&[]);
        assert!(ffi.is_empty());
        unsafe { free_dpns_name_state_entries(&mut ffi) };
    }
}

//! C-compatible types for the wallet-registration round persistence
//! callbacks (`on_persist_account_registrations_fn`,
//! `on_persist_account_address_pools_fn`).
//!
//! These ride on `FFIPersister::store(wallet_id, changeset)` — the
//! whole-changeset entry point — rather than dedicated trait methods,
//! so the registration round (metadata + per-account specs + per-pool
//! snapshots) is one atomic round from the backend's perspective.
//!
//! The per-account-spec callback reuses [`AccountSpecFFI`] from
//! [`crate::wallet_restore_types`] verbatim — the same flat shape
//! Swift already consumes on the load path, so no new account-shape
//! mirror lands on the Swift side.
//!
//! The per-pool callback wraps an array of [`AccountAddressPoolFFI`]
//! values; each entry carries the owning account spec, the pool-type
//! discriminant, and a slice of [`CoreAddressEntryFFI`] rows. Swift
//! iterates and persists, the same row shape it already knows from
//! the legacy `on_persist_account_addresses_fn` path.

use crate::core_address_types::CoreAddressEntryFFI;
use crate::wallet_restore_types::AccountSpecFFI;

/// A single (account, pool, addresses) snapshot inside the
/// `on_persist_account_address_pools_fn` array.
///
/// `spec` borrows from a Rust-owned heap allocation that lives until
/// after the callback returns; the xpub-bytes pointer on `spec` is
/// `null` / `0` for this path because the receiver doesn't need it
/// (the spec is used purely as a lookup key into the per-account
/// SwiftData row).
///
/// `addresses_ptr` borrows a contiguous `[CoreAddressEntryFFI]` slice
/// for the duration of the callback. The strings each entry points
/// at are likewise borrowed from a Rust-owned `CString` pool kept
/// alive in the dispatcher until the callback returns.
#[repr(C)]
pub struct AccountAddressPoolFFI {
    /// Account this pool belongs to. The receiver matches by the
    /// same fields it uses for `on_persist_account_registrations_fn`
    /// (type tag, index, registration index, key class, identity-id
    /// pair). The `account_xpub_bytes` pointer is `null` and length
    /// is 0 on this path; consumers must not dereference it.
    pub account: AccountSpecFFI,
    /// Pool variant — `AddressPoolTypeTagFFI` raw value
    /// (0 = External, 1 = Internal, 2 = Absent, 3 = AbsentHardened).
    pub pool_type_tag: u8,
    /// Pointer to a contiguous `[CoreAddressEntryFFI]` array of
    /// `addresses_count` entries. Borrowed; valid only for the
    /// callback window.
    pub addresses_ptr: *const CoreAddressEntryFFI,
    pub addresses_count: usize,
}

// SAFETY: pointers are Rust-owned and outlive the callback window;
// the struct itself is plain data. Send/Sync to match the rest of
// the FFI surface.
unsafe impl Send for AccountAddressPoolFFI {}
unsafe impl Sync for AccountAddressPoolFFI {}

// Compile-time guard — if anyone reshapes `AccountAddressPoolFFI`
// without also updating the Swift side, cargo builds fail with an
// obvious error rather than producing a dylib that the Swift side
// will mis-parse at runtime (which surfaces as a random
// EXC_BAD_ACCESS in the persistAccountAddressPools callback).
//
// Expected layout on 64-bit targets (all fields in declaration
// order under `#[repr(C)]`):
//
//   0..=95    account                   AccountSpecFFI (96 bytes)
//   ...
//
// We only pin the outer struct size here. On 64-bit targets the
// trailing pool fields add `1 + 7 (pad) + 8 (ptr) + 8 (len) = 24`
// bytes after the 96-byte account spec, for a total of 120.
//
// Recompute via `std::mem::size_of` if the spec layout changes.
const _: [u8; 120] = [0u8; std::mem::size_of::<AccountAddressPoolFFI>()];
const _: [u8; 8] = [0u8; std::mem::align_of::<AccountAddressPoolFFI>()];

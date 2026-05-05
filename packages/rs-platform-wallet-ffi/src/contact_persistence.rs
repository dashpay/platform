//! FFI types + helpers for forwarding
//! [`ContactChangeSet`](platform_wallet::changeset::ContactChangeSet)
//! out of [`FFIPersister`](crate::persistence::FFIPersister) to Swift.
//!
//! `ContactChangeSet` is a top-level (not per-identity) changeset
//! carrying sent / incoming / removed-sent / removed-incoming /
//! established contact requests. The Swift mirror is one row per
//! `(networkRaw, owner_id, contact_id, is_outgoing)` quad in
//! `PersistentDashpayContactRequest` — outgoing and incoming rows for
//! the same `(owner, contact)` pair coexist as distinct rows because
//! the encrypted payload differs per direction.
//!
//! ## Wire shape
//!
//! - Upserts ride as a single flat array of [`ContactRequestFFI`]
//!   regardless of which underlying field on `ContactChangeSet` they
//!   came from. Each row carries an explicit `is_outgoing` bit so the
//!   Swift handler can route it to the correct uniqueness bucket.
//! - Removals split into two parallel arrays — sent vs incoming — to
//!   match the two `BTreeSet<...Key>` fields on `ContactChangeSet`
//!   and so the Swift handler can delete the right `is_outgoing` row
//!   without ambiguity.
//!
//! ## `established` projection
//!
//! Each `EstablishedContact` carries both the outgoing and the
//! incoming `ContactRequest` that built it, so the persister projects
//! the established map as **two** [`ContactRequestFFI`] rows per entry
//! (one with `is_outgoing == true`, one with `is_outgoing == false`).
//! The unique constraint on the Swift side means these upsert cleanly
//! over any prior sent / incoming row for the same `(owner, contact)`
//! pair — establishment promotes the row in place rather than
//! requiring an explicit tombstone.
//!
//! ## Ownership
//!
//! Each [`ContactRequestFFI`] owns its `encrypted_public_key`,
//! `encrypted_account_label`, and `auto_accept_proof` byte buffers
//! (heap-allocated via `Box::into_raw`). [`free_contact_requests_ffi`]
//! releases every allocation across an array — the persister
//! callsite calls it in a final loop after the Swift handler returns.

use std::os::raw::c_void;
use std::ptr;

/// Flat C mirror of a single contact request entry — used for both
/// pending (`sent_requests` / `incoming_requests`) and established
/// (`established`) cases.
///
/// `owner_id` is the wallet-owned identity (the [`ManagedIdentity`]
/// owner the request belongs to). `contact_id` is the other party.
/// The direction bit `is_outgoing` distinguishes "owner sent this
/// request to contact" from "contact sent this request to owner".
///
/// Per-direction key indices, account reference, and the encrypted
/// payload are carried straight through from
/// [`ContactRequest`](platform_wallet::ContactRequest).
///
/// [`ManagedIdentity`]: platform_wallet::ManagedIdentity
#[repr(C)]
pub struct ContactRequestFFI {
    /// Owning identity (the wallet's identity). For sent / outgoing
    /// rows this is the `sender_id`; for incoming rows this is the
    /// `recipient_id`.
    pub owner_id: [u8; 32],
    /// The other party's identity. Mirror image of `owner_id`.
    pub contact_id: [u8; 32],
    /// Direction bit. `true` ⇒ owner sent to contact (the underlying
    /// `ContactRequest` has `sender_id == owner_id`). `false` ⇒
    /// contact sent to owner.
    pub is_outgoing: bool,
    /// `ContactRequest::sender_key_index` — index of the sender's
    /// identity public key used for the ECDH that encrypted the
    /// payload.
    pub sender_key_index: u32,
    /// `ContactRequest::recipient_key_index`.
    pub recipient_key_index: u32,
    /// `ContactRequest::account_reference`.
    pub account_reference: u32,
    /// Heap-allocated copy of `ContactRequest::encrypted_public_key`.
    /// Released by [`free_contact_requests_ffi`].
    pub encrypted_public_key: *const u8,
    /// Length of [`Self::encrypted_public_key`] in bytes.
    pub encrypted_public_key_len: usize,
    /// Heap-allocated copy of `ContactRequest::encrypted_account_label`,
    /// or `null` when the source `Option` was `None`. Released by
    /// [`free_contact_requests_ffi`].
    pub encrypted_account_label: *const u8,
    /// Length of [`Self::encrypted_account_label`] in bytes; `0`
    /// when the pointer is null.
    pub encrypted_account_label_len: usize,
    /// Heap-allocated copy of `ContactRequest::auto_accept_proof`,
    /// or `null` when the source `Option` was `None`. Released by
    /// [`free_contact_requests_ffi`].
    pub auto_accept_proof: *const u8,
    /// Length of [`Self::auto_accept_proof`] in bytes; `0` when the
    /// pointer is null.
    pub auto_accept_proof_len: usize,
    /// `ContactRequest::core_height_created_at` — the Core block
    /// height when the request landed on Platform.
    pub core_height_created_at: u32,
    /// `ContactRequest::created_at` — Unix-millis timestamp.
    pub created_at: u64,
}

/// Composite identifier for [`ContactChangeSet::removed_sent`] and
/// [`ContactChangeSet::removed_incoming`] entries on the FFI boundary.
///
/// A flat `[u8; 32]` pair so Swift can iterate an array directly
/// without a secondary indirection. `owner_id` is always the
/// wallet-owned identity (per the changeset's keyed-by-owner
/// invariant); `contact_id` is the other party (recipient for sent,
/// sender for incoming).
///
/// [`ContactChangeSet::removed_sent`]: platform_wallet::changeset::ContactChangeSet::removed_sent
/// [`ContactChangeSet::removed_incoming`]: platform_wallet::changeset::ContactChangeSet::removed_incoming
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ContactRequestRemovalFFI {
    pub owner_id: [u8; 32],
    pub contact_id: [u8; 32],
}

// Compile-time guards. Pin the expected layouts so any reshape on
// the Rust side fails the cargo build before it can ship a dylib
// the Swift side will mis-parse at runtime.
//
// Expected `ContactRequestFFI` layout on 64-bit targets:
//
//   0..=31    owner_id                       [u8; 32]
//   32..=63   contact_id                     [u8; 32]
//   64        is_outgoing                    bool
//   65..=67   (padding to 4)
//   68..=71   sender_key_index               u32
//   72..=75   recipient_key_index            u32
//   76..=79   account_reference              u32
//   80..=87   encrypted_public_key           *const u8
//   88..=95   encrypted_public_key_len       usize
//   96..=103  encrypted_account_label        *const u8
//   104..=111 encrypted_account_label_len    usize
//   112..=119 auto_accept_proof              *const u8
//   120..=127 auto_accept_proof_len          usize
//   128..=131 core_height_created_at         u32
//   132..=135 (padding to 8)
//   136..=143 created_at                     u64
//
// Total size = 144, alignment = 8 (from u64 / pointer fields).
const _: [u8; 144] = [0u8; std::mem::size_of::<ContactRequestFFI>()];
const _: [u8; 8] = [0u8; std::mem::align_of::<ContactRequestFFI>()];

// Expected `ContactRequestRemovalFFI` layout: 64 bytes, alignment 1.
const _: [u8; 64] = [0u8; std::mem::size_of::<ContactRequestRemovalFFI>()];
const _: [u8; 1] = [0u8; std::mem::align_of::<ContactRequestRemovalFFI>()];

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

impl ContactRequestFFI {
    /// Build a `ContactRequestFFI` from a [`ContactRequest`] for the
    /// outgoing direction (owner sent the request to contact). The
    /// `owner_id` and `is_outgoing == true` are stamped from the
    /// caller; the rest of the fields come straight from the request.
    ///
    /// Heap-allocates the three byte payloads (`encrypted_public_key`
    /// always, the optional `encrypted_account_label` and
    /// `auto_accept_proof` when `Some(_)`). Released by
    /// [`free_contact_requests_ffi`].
    ///
    /// [`ContactRequest`]: platform_wallet::ContactRequest
    pub fn from_outgoing(
        owner_id: [u8; 32],
        contact_id: [u8; 32],
        request: &platform_wallet::ContactRequest,
    ) -> Self {
        Self::from_parts(owner_id, contact_id, true, request)
    }

    /// Sibling of [`Self::from_outgoing`] for the incoming direction
    /// (contact sent the request to owner). `is_outgoing == false`.
    pub fn from_incoming(
        owner_id: [u8; 32],
        contact_id: [u8; 32],
        request: &platform_wallet::ContactRequest,
    ) -> Self {
        Self::from_parts(owner_id, contact_id, false, request)
    }

    fn from_parts(
        owner_id: [u8; 32],
        contact_id: [u8; 32],
        is_outgoing: bool,
        request: &platform_wallet::ContactRequest,
    ) -> Self {
        let (encrypted_public_key, encrypted_public_key_len) =
            allocate_byte_buffer(&request.encrypted_public_key);
        let (encrypted_account_label, encrypted_account_label_len) =
            match request.encrypted_account_label.as_deref() {
                Some(bytes) => allocate_byte_buffer(bytes),
                None => (ptr::null(), 0),
            };
        let (auto_accept_proof, auto_accept_proof_len) = match request.auto_accept_proof.as_deref()
        {
            Some(bytes) => allocate_byte_buffer(bytes),
            None => (ptr::null(), 0),
        };
        Self {
            owner_id,
            contact_id,
            is_outgoing,
            sender_key_index: request.sender_key_index,
            recipient_key_index: request.recipient_key_index,
            account_reference: request.account_reference,
            encrypted_public_key,
            encrypted_public_key_len,
            encrypted_account_label,
            encrypted_account_label_len,
            auto_accept_proof,
            auto_accept_proof_len,
            core_height_created_at: request.core_height_created_at,
            created_at: request.created_at,
        }
    }
}

/// Heap-allocate a `Box<[u8]>` from `bytes` and return a `(ptr, len)`
/// pair owned by the caller. Empty slices return `(null, 0)` so the
/// receiver can avoid an empty allocation walk; the matching free
/// helper checks the length before reclaiming.
fn allocate_byte_buffer(bytes: &[u8]) -> (*const u8, usize) {
    if bytes.is_empty() {
        return (ptr::null(), 0);
    }
    let boxed: Box<[u8]> = bytes.to_vec().into_boxed_slice();
    let len = boxed.len();
    (Box::into_raw(boxed) as *const u8, len)
}

// ---------------------------------------------------------------------------
// Destructors
// ---------------------------------------------------------------------------

/// Release every heap allocation owned by an array of
/// [`ContactRequestFFI`] rows produced by [`ContactRequestFFI::from_outgoing`]
/// / [`ContactRequestFFI::from_incoming`].
///
/// Idempotent on a per-row basis: each pointer is checked for null
/// before reclaim and nulled afterwards.
///
/// # Safety
///
/// `entries` must point to `count` contiguous [`ContactRequestFFI`]
/// values produced by this module's allocators and not previously
/// freed. Mixing in pointers Swift owns (or pointers from a different
/// allocator) will corrupt the heap.
pub unsafe fn free_contact_requests_ffi(entries: *mut ContactRequestFFI, count: usize) {
    if entries.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(entries, count) };
    for entry in slice.iter_mut() {
        free_byte_buffer(
            &mut entry.encrypted_public_key,
            &mut entry.encrypted_public_key_len,
        );
        free_byte_buffer(
            &mut entry.encrypted_account_label,
            &mut entry.encrypted_account_label_len,
        );
        free_byte_buffer(
            &mut entry.auto_accept_proof,
            &mut entry.auto_accept_proof_len,
        );
    }
}

/// Reclaim a `Box<[u8]>` previously published via
/// [`allocate_byte_buffer`]. Idempotent on null / zero-length slots.
fn free_byte_buffer(slot: &mut *const u8, len_slot: &mut usize) {
    if !slot.is_null() && *len_slot > 0 {
        let slice = unsafe { std::slice::from_raw_parts_mut(*slot as *mut u8, *len_slot) };
        let _ = unsafe { Box::from_raw(slice as *mut [u8]) };
    }
    *slot = ptr::null();
    *len_slot = 0;
}

// ---------------------------------------------------------------------------
// Callback signature
// ---------------------------------------------------------------------------

/// C-ABI function pointer type for the contact persistence callback.
/// Defined as a typedef so [`crate::persistence::PersistenceCallbacks`]
/// stays terse.
///
/// Parameters:
/// - `ctx`: opaque context pointer set by the FFI consumer.
/// - `wallet_id`: 32-byte wallet identifier scoping this changeset
///   (matches the parameter on every other per-kind callback). Used
///   by the Swift side to resolve the network for the contact rows.
/// - `upserts` / `upserts_count`: rows to insert-or-refresh, with the
///   per-row `is_outgoing` bit determining which direction the row
///   covers. Pointer is valid only for the duration of the callback.
/// - `removed_sent` / `removed_sent_count`: tombstones for outgoing
///   rows (sent requests explicitly removed by the owner).
/// - `removed_incoming` / `removed_incoming_count`: tombstones for
///   incoming rows.
///
/// Return code: `0` on success, non-zero to flag the round as failed
/// for the bracketing changeset begin/end transaction.
pub type OnPersistContactsFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    wallet_id: *const u8,
    upserts: *const ContactRequestFFI,
    upserts_count: usize,
    removed_sent: *const ContactRequestRemovalFFI,
    removed_sent_count: usize,
    removed_incoming: *const ContactRequestRemovalFFI,
    removed_incoming_count: usize,
) -> i32;

#[cfg(test)]
mod tests {
    use super::*;
    use platform_wallet::ContactRequest;

    fn sample_request() -> ContactRequest {
        ContactRequest {
            sender_id: dpp::prelude::Identifier::from([1u8; 32]),
            recipient_id: dpp::prelude::Identifier::from([2u8; 32]),
            sender_key_index: 7,
            recipient_key_index: 9,
            account_reference: 11,
            encrypted_account_label: Some(vec![0xAA, 0xBB, 0xCC]),
            encrypted_public_key: vec![0x01; 96],
            auto_accept_proof: Some(vec![0xDE, 0xAD, 0xBE, 0xEF]),
            core_height_created_at: 100_000,
            created_at: 1_700_000_000_000,
        }
    }

    #[test]
    fn test_from_outgoing_round_trip() {
        let request = sample_request();
        let owner = [3u8; 32];
        let contact = [4u8; 32];
        let mut ffi = ContactRequestFFI::from_outgoing(owner, contact, &request);
        assert_eq!(ffi.owner_id, owner);
        assert_eq!(ffi.contact_id, contact);
        assert!(ffi.is_outgoing);
        assert_eq!(ffi.sender_key_index, 7);
        assert_eq!(ffi.recipient_key_index, 9);
        assert_eq!(ffi.account_reference, 11);
        assert_eq!(ffi.encrypted_public_key_len, 96);
        let pk = unsafe {
            std::slice::from_raw_parts(ffi.encrypted_public_key, ffi.encrypted_public_key_len)
        };
        assert_eq!(pk, &[0x01; 96]);
        assert_eq!(ffi.encrypted_account_label_len, 3);
        let label = unsafe {
            std::slice::from_raw_parts(ffi.encrypted_account_label, ffi.encrypted_account_label_len)
        };
        assert_eq!(label, &[0xAA, 0xBB, 0xCC]);
        assert_eq!(ffi.auto_accept_proof_len, 4);
        assert_eq!(ffi.core_height_created_at, 100_000);
        assert_eq!(ffi.created_at, 1_700_000_000_000);

        unsafe { free_contact_requests_ffi(&mut ffi as *mut ContactRequestFFI, 1) };
        assert!(ffi.encrypted_public_key.is_null());
        assert_eq!(ffi.encrypted_public_key_len, 0);
        assert!(ffi.encrypted_account_label.is_null());
        assert!(ffi.auto_accept_proof.is_null());
        // Idempotent — second call must not double-free.
        unsafe { free_contact_requests_ffi(&mut ffi as *mut ContactRequestFFI, 1) };
    }

    #[test]
    fn test_from_incoming_no_optional_payloads() {
        let mut request = sample_request();
        request.encrypted_account_label = None;
        request.auto_accept_proof = None;
        let mut ffi = ContactRequestFFI::from_incoming([5u8; 32], [6u8; 32], &request);
        assert!(!ffi.is_outgoing);
        assert!(ffi.encrypted_account_label.is_null());
        assert_eq!(ffi.encrypted_account_label_len, 0);
        assert!(ffi.auto_accept_proof.is_null());
        assert_eq!(ffi.auto_accept_proof_len, 0);
        unsafe { free_contact_requests_ffi(&mut ffi as *mut ContactRequestFFI, 1) };
    }
}

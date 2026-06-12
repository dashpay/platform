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
    /// Whether the [`EstablishedContact`] this row was projected from
    /// has a **permanently broken** payment channel (G1c).
    ///
    /// Only meaningful for rows projected from the `established` map —
    /// both the outgoing and incoming row of an established pair carry
    /// the same flag (it's a property of the relationship, not of one
    /// direction). Always `false` for rows projected from pending
    /// `sent_requests` / `incoming_requests` (a pending request has no
    /// channel yet). The Swift handler persists it on both rows; the UI
    /// reads it to disable "Send Dash" and surface "payment channel
    /// broken — ask the contact to send a new request".
    ///
    /// [`EstablishedContact`]: platform_wallet::EstablishedContact
    pub payment_channel_broken: bool,
    /// Owner-private alias for the contact (`contactInfo`-backed, M3
    /// task 13). Heap-allocated NUL-terminated UTF-8, or null when
    /// unset. Only stamped on rows projected from the `established`
    /// map (pending rows have no metadata); released by
    /// [`free_contact_requests_ffi`].
    pub alias: *const std::os::raw::c_char,
    /// Owner-private note — same conventions as [`Self::alias`].
    pub note: *const std::os::raw::c_char,
    /// `contactInfo.displayHidden` — whether the owner hid this
    /// contact. Established rows only; always `false` for pending.
    pub is_hidden: bool,
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

/// Flat C mirror of a [`RejectedContactRequest`] tombstone (G5 stage 1)
/// for the `rejected` array on [`OnPersistContactsFn`].
///
/// The suppression key is `(owner_id, sender_id, account_reference)` —
/// deliberately **not** bare sender id, so a rotated (bumped
/// `accountReference`) request from the same sender is still let
/// through. The Swift handler persists one row per tombstone keyed on
/// that triple so a once-rejected request stays suppressed across a
/// recurring re-sync.
///
/// `document_id` is carried for audit / exact-match purposes only; it
/// is **not** part of the suppression key. `has_document_id` gates it
/// (`false` ⇒ the source `Option` was `None` and `document_id` is
/// zero-filled).
///
/// [`RejectedContactRequest`]: platform_wallet::changeset::RejectedContactRequest
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ContactRequestRejectionFFI {
    /// The wallet-owned identity that rejected the request (recipient).
    pub owner_id: [u8; 32],
    /// The identity whose request was rejected (the sender).
    pub sender_id: [u8; 32],
    /// The `accountReference` of the rejected request — part of the
    /// suppression key. A request from the same sender with a different
    /// `accountReference` is NOT suppressed.
    pub account_reference: u32,
    /// Whether [`Self::document_id`] carries a real id. `false` ⇒ the
    /// source `Option<Identifier>` was `None`.
    pub has_document_id: bool,
    /// The rejected document's id when known, else zero-filled (gated by
    /// [`Self::has_document_id`]). Not part of the suppression key.
    pub document_id: [u8; 32],
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
//   144       payment_channel_broken         bool
//   145..=151 (padding to 8)
//   152..=159 alias                          *const c_char
//   160..=167 note                           *const c_char
//   168       is_hidden                      bool
//   169..=175 (tail padding to alignment 8)
//
// Total size = 176, alignment = 8 (from u64 / pointer fields).
const _: [u8; 176] = [0u8; std::mem::size_of::<ContactRequestFFI>()];
const _: [u8; 8] = [0u8; std::mem::align_of::<ContactRequestFFI>()];

// Expected `ContactRequestRemovalFFI` layout: 64 bytes, alignment 1.
const _: [u8; 64] = [0u8; std::mem::size_of::<ContactRequestRemovalFFI>()];
const _: [u8; 1] = [0u8; std::mem::align_of::<ContactRequestRemovalFFI>()];

// Expected `ContactRequestRejectionFFI` layout on all targets:
//
//   0..=31    owner_id            [u8; 32]
//   32..=63   sender_id           [u8; 32]
//   64..=67   account_reference   u32
//   68        has_document_id     bool
//   69..=100  document_id         [u8; 32]
//   101..=103 (tail padding to alignment 4)
//
// Total size = 104, alignment = 4 (from the u32 field).
const _: [u8; 104] = [0u8; std::mem::size_of::<ContactRequestRejectionFFI>()];
const _: [u8; 4] = [0u8; std::mem::align_of::<ContactRequestRejectionFFI>()];

impl ContactRequestRejectionFFI {
    /// Project a [`RejectedContactRequest`] onto its flat C mirror.
    /// `document_id` is zero-filled with `has_document_id == false`
    /// when the source `Option` is `None`.
    ///
    /// [`RejectedContactRequest`]: platform_wallet::changeset::RejectedContactRequest
    pub fn from_rejected(rejected: &platform_wallet::changeset::RejectedContactRequest) -> Self {
        let (has_document_id, document_id) = match rejected.document_id {
            Some(id) => (true, id.to_buffer()),
            None => (false, [0u8; 32]),
        };
        Self {
            owner_id: rejected.owner_id.to_buffer(),
            sender_id: rejected.sender_id.to_buffer(),
            account_reference: rejected.account_reference,
            has_document_id,
            document_id,
        }
    }
}

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
        Self::from_parts(owner_id, contact_id, true, request, false, None, None, false)
    }

    /// Sibling of [`Self::from_outgoing`] for the incoming direction
    /// (contact sent the request to owner). `is_outgoing == false`.
    pub fn from_incoming(
        owner_id: [u8; 32],
        contact_id: [u8; 32],
        request: &platform_wallet::ContactRequest,
    ) -> Self {
        Self::from_parts(owner_id, contact_id, false, request, false, None, None, false)
    }

    /// Build the **outgoing** row of an established contact, stamping
    /// the relationship's `payment_channel_broken` flag and the
    /// owner-private metadata (alias / note / hidden — contactInfo,
    /// M3) onto the row.
    ///
    /// Used by the persister's `established` projection (one outgoing +
    /// one incoming row per entry), where these are properties of the
    /// relationship and are therefore replicated onto both rows.
    #[allow(clippy::too_many_arguments)]
    pub fn from_established_outgoing(
        owner_id: [u8; 32],
        contact_id: [u8; 32],
        request: &platform_wallet::ContactRequest,
        payment_channel_broken: bool,
        alias: Option<&str>,
        note: Option<&str>,
        is_hidden: bool,
    ) -> Self {
        Self::from_parts(
            owner_id,
            contact_id,
            true,
            request,
            payment_channel_broken,
            alias,
            note,
            is_hidden,
        )
    }

    /// Sibling of [`Self::from_established_outgoing`] for the **incoming**
    /// row of an established contact.
    #[allow(clippy::too_many_arguments)]
    pub fn from_established_incoming(
        owner_id: [u8; 32],
        contact_id: [u8; 32],
        request: &platform_wallet::ContactRequest,
        payment_channel_broken: bool,
        alias: Option<&str>,
        note: Option<&str>,
        is_hidden: bool,
    ) -> Self {
        Self::from_parts(
            owner_id,
            contact_id,
            false,
            request,
            payment_channel_broken,
            alias,
            note,
            is_hidden,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        owner_id: [u8; 32],
        contact_id: [u8; 32],
        is_outgoing: bool,
        request: &platform_wallet::ContactRequest,
        payment_channel_broken: bool,
        alias: Option<&str>,
        note: Option<&str>,
        is_hidden: bool,
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
            payment_channel_broken,
            alias: allocate_c_string(alias),
            note: allocate_c_string(note),
            is_hidden,
        }
    }
}

/// Heap-allocate a NUL-terminated copy of `value`, or null for `None`
/// / interior-NUL strings. Released by [`free_contact_requests_ffi`]
/// via [`free_c_string`].
fn allocate_c_string(value: Option<&str>) -> *const std::os::raw::c_char {
    match value {
        Some(v) => match std::ffi::CString::new(v) {
            Ok(c) => c.into_raw(),
            Err(_) => ptr::null(),
        },
        None => ptr::null(),
    }
}

/// Reclaim a string previously published via [`allocate_c_string`].
/// Idempotent on null slots.
fn free_c_string(slot: &mut *const std::os::raw::c_char) {
    if !slot.is_null() {
        let _ = unsafe { std::ffi::CString::from_raw(*slot as *mut std::os::raw::c_char) };
    }
    *slot = ptr::null();
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
        free_c_string(&mut entry.alias);
        free_c_string(&mut entry.note);
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
/// - `rejected` / `rejected_count`: rejected-incoming-request tombstones
///   (G5 stage 1), keyed `(owner, sender, account_reference)`. The host
///   persists these so a once-rejected request stays suppressed across
///   a recurring re-sync, while a rotated (bumped-`accountReference`)
///   request from the same sender is still let through. Pointer is
///   valid only for the duration of the callback; rows are POD (no heap
///   payloads), so the host must copy any it wants to retain.
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
    rejected: *const ContactRequestRejectionFFI,
    rejected_count: usize,
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
        // Pending (non-established) rows are never broken.
        assert!(!ffi.payment_channel_broken);

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

    /// The `established_*` constructors stamp the relationship's
    /// `payment_channel_broken` flag onto BOTH the outgoing and incoming
    /// row. This pins the M1 G1c flag survives the persister projection
    /// (the plain `from_outgoing`/`from_incoming` pending constructors
    /// always emit `false` — verified above), so a Swift `@Query`-driven
    /// contact row can render the broken-channel badge without consulting
    /// a live handle getter.
    #[test]
    fn established_rows_carry_payment_channel_broken_flag() {
        let request = sample_request();
        let owner = [3u8; 32];
        let contact = [4u8; 32];

        // Broken relationship: both projected rows must carry the flag —
        // plus the owner-private metadata (contactInfo, M3).
        let mut out = ContactRequestFFI::from_established_outgoing(
            owner,
            contact,
            &request,
            true,
            Some("ally"),
            Some("a note"),
            true,
        );
        let mut inc = ContactRequestFFI::from_established_incoming(
            owner,
            contact,
            &request,
            true,
            Some("ally"),
            Some("a note"),
            true,
        );
        assert!(out.is_outgoing);
        assert!(!inc.is_outgoing);
        assert!(out.payment_channel_broken);
        assert!(inc.payment_channel_broken);
        for row in [&out, &inc] {
            let alias = unsafe { std::ffi::CStr::from_ptr(row.alias) };
            assert_eq!(alias.to_str().unwrap(), "ally");
            let note = unsafe { std::ffi::CStr::from_ptr(row.note) };
            assert_eq!(note.to_str().unwrap(), "a note");
            assert!(row.is_hidden);
        }

        // Healthy relationship without metadata: flag clear, strings null.
        let mut healthy = ContactRequestFFI::from_established_outgoing(
            owner, contact, &request, false, None, None, false,
        );
        assert!(!healthy.payment_channel_broken);
        assert!(healthy.alias.is_null());
        assert!(healthy.note.is_null());
        assert!(!healthy.is_hidden);

        unsafe {
            free_contact_requests_ffi(&mut out as *mut ContactRequestFFI, 1);
            free_contact_requests_ffi(&mut inc as *mut ContactRequestFFI, 1);
            free_contact_requests_ffi(&mut healthy as *mut ContactRequestFFI, 1);
        }
        assert!(out.alias.is_null(), "free must reclaim + null the alias");
        assert!(out.note.is_null());
    }

    /// `ContactRequestRejectionFFI::from_rejected` must carry the full
    /// `(owner, sender, account_reference)` suppression key plus the
    /// optional document id. When the source `document_id` is `Some`,
    /// `has_document_id` is true and the bytes round-trip; when `None`,
    /// `has_document_id` is false and the buffer is zero-filled. Pins the
    /// G5 tombstone projection so the Swift handler can persist the exact
    /// suppression key.
    #[test]
    fn rejection_ffi_round_trips_key_and_optional_document_id() {
        use platform_wallet::changeset::RejectedContactRequest;
        use dpp::prelude::Identifier;

        let with_doc = RejectedContactRequest {
            owner_id: Identifier::from([7u8; 32]),
            sender_id: Identifier::from([8u8; 32]),
            account_reference: 42,
            document_id: Some(Identifier::from([9u8; 32])),
        };
        let ffi = ContactRequestRejectionFFI::from_rejected(&with_doc);
        assert_eq!(ffi.owner_id, [7u8; 32]);
        assert_eq!(ffi.sender_id, [8u8; 32]);
        assert_eq!(ffi.account_reference, 42);
        assert!(ffi.has_document_id);
        assert_eq!(ffi.document_id, [9u8; 32]);

        let without_doc = RejectedContactRequest {
            owner_id: Identifier::from([1u8; 32]),
            sender_id: Identifier::from([2u8; 32]),
            account_reference: 0,
            document_id: None,
        };
        let ffi = ContactRequestRejectionFFI::from_rejected(&without_doc);
        assert!(!ffi.has_document_id);
        // Gated off — the buffer is zero-filled, not garbage.
        assert_eq!(ffi.document_id, [0u8; 32]);
        assert_eq!(ffi.account_reference, 0);
    }
}

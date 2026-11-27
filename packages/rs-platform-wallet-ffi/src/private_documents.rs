//! FFI bindings for reading any private document type's entries (protocol
//! version 15).
//!
//! A private type's store holds fixed-size encrypted entries that name
//! nobody; a wallet finds its own by trying every entry with its identities'
//! keys. [`platform_wallet_read_private_document_entries`] runs
//! `IdentityWallet::read_private_document_entries` with keys derived inside
//! the resolver-backed signer and hands back what opened, each entry's
//! payload read against the type's schema. The read keeps nothing: the
//! caller keeps the cursor it got and hands it back to read on.
//!
//! Release the result with [`platform_wallet_private_document_read_free`].

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use platform_encryption::PrivateDocumentAuthMode;
use platform_wallet::{
    OpenedPrivateDocument, PrivateDocumentCursor, PrivateDocumentDirection, PrivateDocumentRead,
};
use rs_sdk_ffi::MnemonicResolverHandle;

use crate::dashpay::resolver_contact_crypto_provider;
use crate::error::*;
use crate::handle::*;
use crate::private_contact_request::{saturating_count, seed_mismatch_result};
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

/// One entry of a private document type's store that one of the wallet's
/// identities opened or recovered (`OpenedPrivateDocument`). An entry from
/// one of the wallet's identities to another is reported twice, once each
/// way.
///
/// Exactly one of `properties_json` and `properties_refusal` is non-null.
/// Both are read-only strings owned by the enclosing
/// [`PrivateDocumentReadFFI`] and released with it: do not modify or free
/// them, as their release needs their original length; copy one first when an
/// API needs a writable buffer.
#[repr(C)]
#[derive(Debug)]
pub struct PrivateDocumentEntryFFI {
    /// The entry's position in the store.
    pub position: u64,
    /// The height of the block that appended it.
    pub block_height: u64,
    /// The time of the block that appended it, in milliseconds.
    pub block_time_ms: u64,
    /// The properties the payload holds, decoded and checked against the
    /// type's schema, as a NUL-terminated UTF-8 JSON object (binary values
    /// base64, identifiers base58, as a document query returns them); null
    /// when the payload was refused.
    pub properties_json: *mut c_char,
    /// Why the payload's properties were refused, as a NUL-terminated UTF-8
    /// string; null when they read.
    pub properties_refusal: *mut c_char,
    /// The wallet's identity that opened the entry, when received, or whose
    /// outgoing viewing key recovered it, when sent.
    pub wallet_identity_id: [u8; 32],
    /// The identity the entry is sealed to.
    pub recipient_identity_id: [u8; 32],
    /// The identity the entry names as its sender, when
    /// `has_claimed_sender`; zeros otherwise. A claim: the read does not
    /// check it against that identity's keys.
    pub claimed_sender_identity_id: [u8; 32],
    /// The entry's commitment: two entries with the same one are copies.
    pub commitment: [u8; 32],
    /// The wallet identity's key: the one the entry is sealed to, when
    /// received, or the one the outgoing viewing key is derived from, when
    /// sent.
    pub wallet_key_id: u32,
    /// The key the entry names as its sender's, when `has_claimed_sender`;
    /// 0 otherwise. A claim, as `claimed_sender_identity_id`.
    pub claimed_sender_key_id: u32,
    /// Which way the entry went: 0 received, 1 sent.
    pub direction: u8,
    /// How the entry tells its recipient who wrote it: 0 anonymous, 1
    /// deniable MAC, 2 signature.
    pub auth_mode: u8,
    /// Whether the entry names a sender (it is not anonymous).
    pub has_claimed_sender: bool,
}

// Expected `PrivateDocumentEntryFFI` layout on 64-bit targets:
//
//   0..=7      position                    u64
//   8..=15     block_height                u64
//   16..=23    block_time_ms               u64
//   24..=31    properties_json             *mut c_char
//   32..=39    properties_refusal          *mut c_char
//   40..=71    wallet_identity_id          [u8; 32]
//   72..=103   recipient_identity_id       [u8; 32]
//   104..=135  claimed_sender_identity_id  [u8; 32]
//   136..=167  commitment                  [u8; 32]
//   168..=171  wallet_key_id               u32
//   172..=175  claimed_sender_key_id       u32
//   176        direction                   u8
//   177        auth_mode                   u8
//   178        has_claimed_sender          bool
//   179..=183  (tail padding to 8)
//
// Total size = 184, alignment = 8.
#[cfg(target_pointer_width = "64")]
const _: [u8; 184] = [0u8; std::mem::size_of::<PrivateDocumentEntryFFI>()];
#[cfg(target_pointer_width = "64")]
const _: [u8; 8] = [0u8; std::mem::align_of::<PrivateDocumentEntryFFI>()];
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, block_height) == 8);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, block_time_ms) == 16);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, properties_json) == 24);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, properties_refusal) == 32);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, wallet_identity_id) == 40);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, recipient_identity_id) == 72);
#[cfg(target_pointer_width = "64")]
const _: () =
    assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, claimed_sender_identity_id) == 104);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, commitment) == 136);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, wallet_key_id) == 168);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, claimed_sender_key_id) == 172);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, direction) == 176);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, auth_mode) == 177);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryFFI, has_claimed_sender) == 178);

/// Where reading a private document type's store goes on from
/// (`PrivateDocumentCursor`). The caller keeps it between reads and hands it
/// back unchanged: it carries the largest store size proved so far, so a page
/// from a node behind does not end the reading early, the fingerprint of the
/// keys read with, so a read after the wallet's keys changed starts over, and
/// the fingerprint of the store read, so a read of another store refuses it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrivateDocumentCursorFFI {
    /// The position the next read starts at.
    pub position: u64,
    /// The largest store size a page proved, when
    /// `has_proved_total_count`; 0 otherwise.
    pub proved_total_count: u64,
    /// The fingerprint of the keys the reads scanned with. Opaque: hand it
    /// back as it came.
    pub key_set: [u8; 32],
    /// The fingerprint of the store (network, contract id and type name) the
    /// reads read. Opaque: hand it back as it came.
    pub store: [u8; 32],
    /// Whether a page was ever proved, so `proved_total_count` holds the
    /// store size. `false` means the store's size is unknown, not 0.
    pub has_proved_total_count: bool,
}

// Expected `PrivateDocumentCursorFFI` layout on 64-bit targets:
//
//   0..=7    position                u64
//   8..=15   proved_total_count      u64
//   16..=47  key_set                 [u8; 32]
//   48..=79  store                   [u8; 32]
//   80       has_proved_total_count  bool
//   81..=87  (tail padding to 8)
//
// Total size = 88, alignment = 8.
#[cfg(target_pointer_width = "64")]
const _: [u8; 88] = [0u8; std::mem::size_of::<PrivateDocumentCursorFFI>()];
#[cfg(target_pointer_width = "64")]
const _: [u8; 8] = [0u8; std::mem::align_of::<PrivateDocumentCursorFFI>()];
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentCursorFFI, proved_total_count) == 8);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentCursorFFI, key_set) == 16);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentCursorFFI, store) == 48);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentCursorFFI, has_proved_total_count) == 80);

impl From<PrivateDocumentCursor> for PrivateDocumentCursorFFI {
    fn from(cursor: PrivateDocumentCursor) -> Self {
        Self {
            position: cursor.position,
            proved_total_count: cursor.proved_total_count.unwrap_or(0),
            key_set: cursor.key_set,
            store: cursor.store,
            has_proved_total_count: cursor.proved_total_count.is_some(),
        }
    }
}

impl From<PrivateDocumentCursorFFI> for PrivateDocumentCursor {
    fn from(cursor: PrivateDocumentCursorFFI) -> Self {
        Self {
            position: cursor.position,
            proved_total_count: cursor
                .has_proved_total_count
                .then_some(cursor.proved_total_count),
            key_set: cursor.key_set,
            store: cursor.store,
        }
    }
}

/// What a read of a private document type's store found for the wallet
/// (`PrivateDocumentRead`). Counts above `u32::MAX` saturate. Release with
/// [`platform_wallet_private_document_read_free`].
#[repr(C)]
#[derive(Debug)]
pub struct PrivateDocumentReadFFI {
    /// The entries opened or recovered, in store order (at one position, the
    /// received one first); null when there are none.
    pub entries: *mut PrivateDocumentEntryFFI,
    /// How many entries `entries` holds.
    pub entry_count: usize,
    /// Where to read on from. The read keeps nothing: pass it back, as it
    /// came, as `from` to read on.
    pub cursor: PrivateDocumentCursorFFI,
    /// The store pages read.
    pub pages_read: u32,
    /// Entries that decrypted under one of the wallet's keys but are
    /// refused.
    pub rejected_count: u32,
    /// The wallet's identities the store was read for: those with an HD
    /// index and a key an entry can be sealed to. With none, nothing was
    /// read.
    pub identity_count: u32,
    /// Keys of the wallet's identities an entry could be sealed to that the
    /// wallet does not derive at their identity key slot, such as a DashPay
    /// Connect application key: the read did not scan with them, so no entry
    /// sealed to them is in it, whether or not the read is complete.
    pub unscanned_key_count: u32,
    /// Whether the read reached the store's last entry, as large a store as
    /// any page proved, with every key but the unscanned ones
    /// (`unscanned_key_count`). A later read from `cursor` reads the entries
    /// appended since. `false` with `pages_read` 0 means nothing was read: the
    /// page budget was 0, or no identity has a key the wallet can scan with,
    /// and reading again changes nothing until that does.
    pub complete: bool,
    /// The wallet's keys changed since the `from` cursor, so the read started
    /// over at the store's start: its entries replace those of the earlier
    /// reads rather than follow them.
    pub restarted: bool,
}

// Expected `PrivateDocumentReadFFI` layout on 64-bit targets:
//
//   0..=7     entries              *mut PrivateDocumentEntryFFI
//   8..=15    entry_count          usize
//   16..=103  cursor               PrivateDocumentCursorFFI (88 bytes)
//   104..=107 pages_read           u32
//   108..=111 rejected_count       u32
//   112..=115 identity_count       u32
//   116..=119 unscanned_key_count  u32
//   120       complete             bool
//   121       restarted            bool
//   122..=127 (tail padding to 8)
//
// Total size = 128, alignment = 8.
#[cfg(target_pointer_width = "64")]
const _: [u8; 128] = [0u8; std::mem::size_of::<PrivateDocumentReadFFI>()];
#[cfg(target_pointer_width = "64")]
const _: [u8; 8] = [0u8; std::mem::align_of::<PrivateDocumentReadFFI>()];
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentReadFFI, entry_count) == 8);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentReadFFI, cursor) == 16);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentReadFFI, pages_read) == 104);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentReadFFI, rejected_count) == 108);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentReadFFI, identity_count) == 112);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentReadFFI, unscanned_key_count) == 116);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentReadFFI, complete) == 120);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::offset_of!(PrivateDocumentReadFFI, restarted) == 121);

impl PrivateDocumentReadFFI {
    /// No entries, every count zero, a zeroed cursor: the value an
    /// out-param holds before a read fills it and after it is freed.
    pub fn empty() -> Self {
        Self {
            entries: std::ptr::null_mut(),
            entry_count: 0,
            cursor: PrivateDocumentCursorFFI::default(),
            pages_read: 0,
            rejected_count: 0,
            identity_count: 0,
            unscanned_key_count: 0,
            complete: false,
            restarted: false,
        }
    }
}

impl Default for PrivateDocumentReadFFI {
    fn default() -> Self {
        Self::empty()
    }
}

/// The discriminant `auth_mode` carries over the FFI.
fn auth_mode_byte(auth_mode: PrivateDocumentAuthMode) -> u8 {
    match auth_mode {
        PrivateDocumentAuthMode::Anonymous => 0,
        PrivateDocumentAuthMode::DeniableMac => 1,
        PrivateDocumentAuthMode::Signature => 2,
    }
}

/// The discriminant `direction` carries over the FFI.
fn direction_byte(direction: PrivateDocumentDirection) -> u8 {
    match direction {
        PrivateDocumentDirection::Received => 0,
        PrivateDocumentDirection::Sent => 1,
    }
}

/// Release the strings `entry` owns.
///
/// # Safety
/// Each non-null string must come from `CString::into_raw` and not be freed
/// yet.
unsafe fn free_entry_strings(entry: &mut PrivateDocumentEntryFFI) {
    for string in [&mut entry.properties_json, &mut entry.properties_refusal] {
        if !string.is_null() {
            unsafe { drop(CString::from_raw(*string)) };
            *string = std::ptr::null_mut();
        }
    }
}

/// `entry` as the FFI hands it over; its properties as the JSON a document
/// query returns (`platform_value::Value` through `serde_json`, which escapes
/// a NUL in a string as `\u0000`).
///
/// A refusal is DPP's diagnostic, which can quote a rejected string verbatim,
/// NUL included; a C string cannot hold a NUL, so each is written as the two
/// characters `\0`. The entry keeps its refusal and the read its other
/// entries.
fn private_document_entry_ffi(
    entry: OpenedPrivateDocument,
) -> Result<PrivateDocumentEntryFFI, PlatformWalletFFIResult> {
    let (properties_json, properties_refusal) = match entry.properties {
        Ok(properties) => (
            CString::new(serde_json::to_string(&properties)?)?.into_raw(),
            std::ptr::null_mut(),
        ),
        Err(refusal) => (
            std::ptr::null_mut(),
            CString::new(refusal.replace('\0', "\\0"))?.into_raw(),
        ),
    };
    let (claimed_sender_identity_id, claimed_sender_key_id) = entry
        .claimed_sender
        .map(|(identity_id, key_id)| (identity_id.to_buffer(), key_id))
        .unwrap_or(([0; 32], 0));
    Ok(PrivateDocumentEntryFFI {
        position: entry.position,
        block_height: entry.block_height,
        block_time_ms: entry.block_time_ms,
        properties_json,
        properties_refusal,
        wallet_identity_id: entry.wallet_identity_id.to_buffer(),
        recipient_identity_id: entry.recipient_identity_id.to_buffer(),
        claimed_sender_identity_id,
        commitment: entry.commitment,
        wallet_key_id: entry.wallet_key_id,
        claimed_sender_key_id,
        direction: direction_byte(entry.direction),
        auth_mode: auth_mode_byte(entry.auth_mode),
        has_claimed_sender: entry.claimed_sender.is_some(),
    })
}

/// `read` as the FFI hands it over. The entries are moved into an
/// exact-length boxed slice, which
/// [`platform_wallet_private_document_read_free`] reclaims as one. On an
/// error nothing stays allocated.
fn private_document_read_ffi(
    read: PrivateDocumentRead,
) -> Result<PrivateDocumentReadFFI, PlatformWalletFFIResult> {
    let mut entries = Vec::with_capacity(read.entries.len());
    for entry in read.entries {
        match private_document_entry_ffi(entry) {
            Ok(entry) => entries.push(entry),
            Err(error) => {
                for entry in &mut entries {
                    // SAFETY: the strings were made by `private_document_entry_ffi`
                    // and are released once, here.
                    unsafe { free_entry_strings(entry) };
                }
                return Err(error);
            }
        }
    }
    let entry_count = entries.len();
    let entries = if entries.is_empty() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(entries.into_boxed_slice()) as *mut PrivateDocumentEntryFFI
    };
    Ok(PrivateDocumentReadFFI {
        entries,
        entry_count,
        cursor: read.cursor.into(),
        pages_read: saturating_count(read.pages_read),
        rejected_count: saturating_count(read.rejected.len()),
        identity_count: saturating_count(read.identities.len()),
        unscanned_key_count: saturating_count(read.unscanned_keys.len()),
        complete: read.complete,
        restarted: read.restarted,
    })
}

/// What a host should know after a read panicked: a read sends nothing and
/// keeps nothing, so this call changed nothing and the cursor passed in still
/// holds.
const READ_PANIC_GUIDANCE: &str = "The read sent nothing and kept nothing; no result was \
     written, and the cursor passed in still holds.";

/// Run the read export's body with a panic turned into an
/// `ErrorWalletOperation` result instead of unwinding into the C caller:
/// `block_on_worker` re-raises a worker's panic on the calling thread,
/// inside the `extern "C"` frame, where it would abort the host. The code
/// promises no write: a read leaves nothing behind to reconcile.
fn catch_read_panic(
    operation: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        operation,
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        None,
        READ_PANIC_GUIDANCE,
        body,
    )
}

/// The cursor `from` points to; `None` for null, which reads from the store's
/// start.
///
/// # Safety
/// `from`, when non-null, must point to a readable `PrivateDocumentCursorFFI`.
unsafe fn read_cursor(from: *const PrivateDocumentCursorFFI) -> Option<PrivateDocumentCursor> {
    unsafe { from.as_ref() }.map(|cursor| PrivateDocumentCursor::from(*cursor))
}

/// Read the store of the private document type `document_type_name` of the
/// contract `data_contract_id` for every identity of the wallet, at most
/// `max_pages` proved pages from the cursor `from`, or from the store's start
/// when `from` is null (`IdentityWallet::read_private_document_entries`): the
/// entries sealed to them, opened, and the entries they sealed, recovered
/// from the store alone, each with its payload read against the type's
/// schema.
///
/// `core_signer_handle` is the wallet-HD resolver signer: the keys are
/// derived inside it and stay there. Nothing is kept: read on by passing the
/// result's `cursor` back, unchanged, as `from`. A store only grows, so a
/// later read from there holds what was appended since. When the wallet's
/// keys changed since that cursor, the read starts over from the store's
/// start and sets `restarted`: its entries replace the earlier ones. An
/// entry's sender is what the entry claims; the read does not check it
/// against that identity's keys.
///
/// Two reads read no page. With `max_pages` 0, or no identity holding a key
/// with a path, the read succeeds before the resolver, the seed check, the
/// protocol version check and any request: its success shows none of them.
/// When keys have paths but none derives its registered key, the resolver is
/// asked first (the seed check and the key derivation), so the read fails as
/// they fail, then succeeds with no page and no request to the store.
///
/// Errors: a `from` cursor of another store (another contract or type) with
/// `ErrorInvalidParameter`; a resolver whose seed is not the wallet's is
/// refused before any page is read with `ErrorInvalidParameter`, as the
/// private contact request sync refuses it; a network below protocol version
/// 15 with `ErrorProtocolVersionNotActive`; a contract that declares no such
/// private type, or a failed page fetch, with the code the wallet error maps
/// to.
///
/// A panic in the read or its marshalling is returned as
/// `ErrorWalletOperation`, with `*out_read` left the empty value, on builds
/// where panics unwind (Android and host builds). The iOS profiles build with
/// `panic = "abort"`, so there a panic still aborts the process before this
/// guard can see it.
///
/// `*out_read` is reset to the empty value first, so it is well-defined on
/// every return; release it with
/// [`platform_wallet_private_document_read_free`].
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`; the caller pins it for the call.
/// - `data_contract_id` must point to 32 readable bytes.
/// - `document_type_name` must be a valid NUL-terminated UTF-8 C string.
/// - `from`, when non-null, must point to a readable `PrivateDocumentCursorFFI`
///   returned by a read of this store and must not overlap `out_read`.
/// - `out_read` must be a valid, writable pointer and must not own a previous
///   result: the call resets it first. To read on, copy the previous result's
///   cursor by value, free the previous result, and pass the copy as `from`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_read_private_document_entries(
    wallet_handle: Handle,
    core_signer_handle: *mut MnemonicResolverHandle,
    data_contract_id: *const u8,
    document_type_name: *const c_char,
    from: *const PrivateDocumentCursorFFI,
    max_pages: u32,
    out_read: *mut PrivateDocumentReadFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_read);
    // Publish the empty value before any fallible work, so every early
    // return leaves the out-param well-defined.
    unsafe { *out_read = PrivateDocumentReadFFI::empty() };
    catch_read_panic("private document read", || {
        check_ptr!(core_signer_handle);
        check_ptr!(data_contract_id);
        check_ptr!(document_type_name);
        let data_contract_id =
            unwrap_result_or_return!(unsafe { read_identifier(data_contract_id) });
        let document_type_name =
            unwrap_result_or_return!(unsafe { CStr::from_ptr(document_type_name) }.to_str())
                .to_string();
        // SAFETY: the caller guarantees a non-null `from` is readable.
        let from = unsafe { read_cursor(from) };

        // Look the wallet up under the registry guard, but run the read
        // outside it: a read fetches many pages, and a registry read guard
        // held across that wait would stall `platform_wallet_destroy` and
        // every other registry reader.
        let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
            (
                wallet.identity().clone(),
                wallet.wallet_id(),
                wallet.network(),
            )
        });
        let (identity, wallet_id, network) = unwrap_option_or_return!(option);
        // SAFETY: the caller pins the resolver handle for the duration of
        // this call.
        let provider =
            unsafe { resolver_contact_crypto_provider(core_signer_handle, wallet_id, network) };
        let result = block_on_worker(async move {
            identity
                .read_private_document_entries(
                    &provider,
                    data_contract_id,
                    &document_type_name,
                    from,
                    max_pages as usize,
                )
                .await
        });
        let read = match result {
            Ok(read) => read,
            Err(error) => return seed_mismatch_result(error).unwrap_or_else(|error| error.into()),
        };
        let read = match private_document_read_ffi(read) {
            Ok(read) => read,
            Err(result) => return result,
        };
        // The last step: nothing after it can panic, so a caught panic
        // leaves `*out_read` the empty value.
        unsafe { *out_read = read };
        PlatformWalletFFIResult::ok()
    })
}

/// Release what a [`platform_wallet_read_private_document_entries`] wrote
/// to `read`: the entries array and every string in it. Resets `*read` to
/// the empty value, so a second call, or a call on the empty value, does
/// nothing. Null is ignored.
///
/// # Safety
/// `read`, when non-null, must point to a `PrivateDocumentReadFFI` that is
/// the empty value or was filled by
/// `platform_wallet_read_private_document_entries` and not freed since.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_private_document_read_free(
    read: *mut PrivateDocumentReadFFI,
) {
    if read.is_null() {
        return;
    }
    let read = unsafe { &mut *read };
    if !read.entries.is_null() && read.entry_count > 0 {
        // Mirror image of `private_document_read_ffi`: the entries were
        // handed over as an exact-length boxed slice.
        let mut entries = unsafe {
            Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                read.entries,
                read.entry_count,
            ))
        };
        for entry in entries.iter_mut() {
            // SAFETY: each string was made by `private_document_entry_ffi`
            // and is released once, here.
            unsafe { free_entry_strings(entry) };
        }
    }
    *read = PrivateDocumentReadFFI::empty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use dash_sdk::platform::private_document_scan::RejectedPrivateDocumentEntry;
    use dpp::platform_value::Value;
    use dpp::prelude::Identifier;
    use platform_encryption::PrivateDocumentEnvelopeError;

    const WALLET_IDENTITY: [u8; 32] = [0xA1; 32];
    const OTHER_IDENTITY: [u8; 32] = [0xC2; 32];
    const KEY_SET: [u8; 32] = [0x5E; 32];
    const STORE: [u8; 32] = [0x57; 32];

    fn text_of(string: *mut c_char) -> String {
        assert!(!string.is_null());
        unsafe { CStr::from_ptr(string) }
            .to_str()
            .expect("UTF-8")
            .to_string()
    }

    /// A read holding an entry received with a claimed sender, an anonymous
    /// one sent, and a received one whose payload was refused.
    fn sample_read() -> PrivateDocumentRead {
        let entry = |position: u64,
                     direction: PrivateDocumentDirection,
                     auth_mode: PrivateDocumentAuthMode,
                     claimed_sender: Option<(Identifier, u32)>,
                     properties: Result<BTreeMap<String, Value>, String>| {
            OpenedPrivateDocument {
                position,
                block_height: 10 + position,
                block_time_ms: 1_700_000_000_000 + position,
                direction,
                wallet_identity_id: Identifier::from(WALLET_IDENTITY),
                wallet_key_id: 2,
                recipient_identity_id: Identifier::from(match direction {
                    PrivateDocumentDirection::Received => WALLET_IDENTITY,
                    PrivateDocumentDirection::Sent => OTHER_IDENTITY,
                }),
                auth_mode,
                claimed_sender,
                commitment: [position as u8; 32],
                properties,
            }
        };
        PrivateDocumentRead {
            identities: vec![Identifier::from(WALLET_IDENTITY)],
            entries: vec![
                entry(
                    0,
                    PrivateDocumentDirection::Received,
                    PrivateDocumentAuthMode::DeniableMac,
                    Some((Identifier::from(OTHER_IDENTITY), 4)),
                    Ok(BTreeMap::from([
                        ("text".to_string(), Value::Text("hello".to_string())),
                        ("blob".to_string(), Value::Bytes(vec![1, 2, 3])),
                        ("friend".to_string(), Value::Identifier([0; 32])),
                    ])),
                ),
                entry(
                    2,
                    PrivateDocumentDirection::Sent,
                    PrivateDocumentAuthMode::Anonymous,
                    None,
                    Ok(BTreeMap::from([(
                        "text".to_string(),
                        Value::Text("hi".to_string()),
                    )])),
                ),
                entry(
                    3,
                    PrivateDocumentDirection::Received,
                    PrivateDocumentAuthMode::Signature,
                    Some((Identifier::from(OTHER_IDENTITY), 1)),
                    Err("text is shorter than 3".to_string()),
                ),
            ],
            rejected: vec![RejectedPrivateDocumentEntry {
                position: 1,
                block_height: 11,
                error: PrivateDocumentEnvelopeError::WrongRecipient,
            }],
            unscanned_keys: vec![
                (Identifier::from(WALLET_IDENTITY), 7),
                (Identifier::from(WALLET_IDENTITY), 9),
            ],
            pages_read: 2,
            cursor: PrivateDocumentCursor {
                position: 4,
                proved_total_count: Some(4),
                key_set: KEY_SET,
                store: STORE,
            },
            restarted: true,
            complete: true,
        }
    }

    #[test]
    fn should_marshal_received_sent_and_refused_entries_and_free_them() {
        let mut read = private_document_read_ffi(sample_read()).expect("the read marshals");

        assert_eq!(read.entry_count, 3);
        assert_eq!(
            read.cursor,
            PrivateDocumentCursorFFI {
                position: 4,
                proved_total_count: 4,
                key_set: KEY_SET,
                store: STORE,
                has_proved_total_count: true,
            }
        );
        assert_eq!(
            (
                read.pages_read,
                read.rejected_count,
                read.identity_count,
                read.unscanned_key_count
            ),
            (2, 1, 1, 2)
        );
        assert!(read.complete);
        assert!(read.restarted);
        let entries = unsafe { std::slice::from_raw_parts(read.entries, read.entry_count) };

        let received = &entries[0];
        assert_eq!(
            (
                received.position,
                received.block_height,
                received.block_time_ms
            ),
            (0, 10, 1_700_000_000_000)
        );
        assert_eq!((received.direction, received.auth_mode), (0, 1));
        assert!(received.has_claimed_sender);
        assert_eq!(received.claimed_sender_identity_id, OTHER_IDENTITY);
        assert_eq!(received.claimed_sender_key_id, 4);
        assert_eq!(received.wallet_identity_id, WALLET_IDENTITY);
        assert_eq!(received.recipient_identity_id, WALLET_IDENTITY);
        assert_eq!(received.wallet_key_id, 2);
        assert_eq!(received.commitment, [0; 32]);
        assert!(received.properties_refusal.is_null());
        // The JSON a document query returns: bytes base64, identifiers base58.
        let json: serde_json::Value =
            serde_json::from_str(&text_of(received.properties_json)).expect("JSON");
        assert_eq!(
            json,
            serde_json::json!({
                "text": "hello",
                "blob": "AQID",
                "friend": "11111111111111111111111111111111",
            })
        );

        let sent = &entries[1];
        assert_eq!((sent.position, sent.direction, sent.auth_mode), (2, 1, 0));
        assert!(!sent.has_claimed_sender);
        assert_eq!(sent.claimed_sender_identity_id, [0; 32]);
        assert_eq!(sent.claimed_sender_key_id, 0);
        assert_eq!(sent.recipient_identity_id, OTHER_IDENTITY);
        assert_eq!(text_of(sent.properties_json), r#"{"text":"hi"}"#);
        assert!(sent.properties_refusal.is_null());

        let refused = &entries[2];
        assert_eq!((refused.position, refused.auth_mode), (3, 2));
        assert!(refused.properties_json.is_null());
        assert_eq!(
            text_of(refused.properties_refusal),
            "text is shorter than 3"
        );

        unsafe { platform_wallet_private_document_read_free(&mut read) };
        assert_empty(&read);
        // Idempotent once reset.
        unsafe { platform_wallet_private_document_read_free(&mut read) };
    }

    #[test]
    fn should_marshal_a_read_for_no_identity_as_empty_and_free_it_safely() {
        let mut read = private_document_read_ffi(PrivateDocumentRead {
            cursor: PrivateDocumentCursor {
                position: 5,
                proved_total_count: None,
                key_set: KEY_SET,
                store: STORE,
            },
            ..Default::default()
        })
        .expect("the read marshals");
        assert!(read.entries.is_null());
        assert_eq!(
            (
                read.entry_count,
                read.identity_count,
                read.unscanned_key_count
            ),
            (0, 0, 0)
        );
        assert_eq!(read.cursor.position, 5);
        // No page proved: the size is unknown, not 0.
        assert!(!read.cursor.has_proved_total_count);
        unsafe {
            platform_wallet_private_document_read_free(&mut read);
            platform_wallet_private_document_read_free(&mut read);
            platform_wallet_private_document_read_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn should_escape_a_nul_in_a_refusal_and_keep_the_rest_of_the_read() {
        let mut read = sample_read();
        // A DPP diagnostic can quote the rejected string, NUL included.
        read.entries[2].properties = Err("bad\0reason".to_string());
        let mut read = private_document_read_ffi(read).expect("the read marshals");

        assert_eq!(read.entry_count, 3);
        let entries = unsafe { std::slice::from_raw_parts(read.entries, read.entry_count) };
        assert!(entries[2].properties_json.is_null());
        assert_eq!(text_of(entries[2].properties_refusal), r"bad\0reason");
        assert!(text_of(entries[0].properties_json).contains("hello"));
        assert_eq!(text_of(entries[1].properties_json), r#"{"text":"hi"}"#);
        assert_eq!(read.cursor.position, 4);
        assert_eq!(read.cursor.key_set, KEY_SET);
        assert_eq!(read.cursor.store, STORE);
        assert_eq!(
            (read.pages_read, read.rejected_count, read.identity_count),
            (2, 1, 1)
        );

        unsafe { platform_wallet_private_document_read_free(&mut read) };
        assert_empty(&read);
    }

    #[test]
    fn should_fail_the_read_when_properties_cannot_become_json() {
        let mut read = sample_read();
        // JSON object keys are strings: a map keyed by an array has no JSON.
        // The first two entries' strings are made by then, and are freed.
        read.entries[2].properties = Ok(BTreeMap::from([(
            "nested".to_string(),
            Value::Map(vec![(Value::Array(vec![]), Value::Null)]),
        )]));
        let result = private_document_read_ffi(read).expect_err("no JSON for the properties");
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
    }

    #[test]
    fn should_round_trip_a_cursor_through_its_ffi_form() {
        // A null `from` reads from the store's start; it is no error.
        assert_eq!(unsafe { read_cursor(std::ptr::null()) }, None);

        for proved_total_count in [Some(7), Some(0), None] {
            let cursor = PrivateDocumentCursor {
                position: 3,
                proved_total_count,
                key_set: KEY_SET,
                store: STORE,
            };
            let ffi = PrivateDocumentCursorFFI::from(cursor);
            assert_eq!(ffi.has_proved_total_count, proved_total_count.is_some());
            assert_eq!(ffi.proved_total_count, proved_total_count.unwrap_or(0));
            assert_eq!((ffi.key_set, ffi.store), (KEY_SET, STORE));
            assert_eq!(unsafe { read_cursor(&ffi) }, Some(cursor));
        }
    }

    /// Calls the read from `from` with each pointer argument valid except as
    /// given.
    unsafe fn read_with(
        resolver: *mut MnemonicResolverHandle,
        contract_id: *const u8,
        type_name: *const c_char,
        from: *const PrivateDocumentCursorFFI,
        out_read: *mut PrivateDocumentReadFFI,
    ) -> PlatformWalletFFIResult {
        unsafe {
            platform_wallet_read_private_document_entries(
                0xDEAD_BEEF,
                resolver,
                contract_id,
                type_name,
                from,
                1,
                out_read,
            )
        }
    }

    fn stale_read() -> PrivateDocumentReadFFI {
        PrivateDocumentReadFFI {
            entries: std::ptr::null_mut(),
            entry_count: 0,
            cursor: PrivateDocumentCursorFFI {
                position: 9,
                proved_total_count: 9,
                key_set: KEY_SET,
                store: STORE,
                has_proved_total_count: true,
            },
            pages_read: 9,
            rejected_count: 9,
            identity_count: 9,
            unscanned_key_count: 9,
            complete: true,
            restarted: true,
        }
    }

    fn assert_empty(read: &PrivateDocumentReadFFI) {
        assert!(read.entries.is_null());
        assert_eq!(read.cursor, PrivateDocumentCursorFFI::default());
        assert_eq!(
            (
                read.entry_count,
                read.pages_read,
                read.rejected_count,
                read.identity_count,
                read.unscanned_key_count,
                read.complete,
                read.restarted,
            ),
            (0, 0, 0, 0, 0, false, false)
        );
    }

    #[test]
    fn should_reject_each_null_pointer_argument_of_the_read() {
        let contract_id = [7u8; 32];
        let type_name = c"secret";
        // A resolver is never called before the wallet lookup; any non-null
        // pointer passes the null check.
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();

        let result = unsafe {
            read_with(
                resolver,
                contract_id.as_ptr(),
                type_name.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        let cases: [(*mut MnemonicResolverHandle, *const u8, *const c_char); 3] = [
            (
                std::ptr::null_mut(),
                contract_id.as_ptr(),
                type_name.as_ptr(),
            ),
            (resolver, std::ptr::null(), type_name.as_ptr()),
            (resolver, contract_id.as_ptr(), std::ptr::null()),
        ];
        for (resolver, contract_id, type_name) in cases {
            let mut read = stale_read();
            let result = unsafe {
                read_with(
                    resolver,
                    contract_id,
                    type_name,
                    std::ptr::null(),
                    &mut read,
                )
            };
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            assert_empty(&read);
        }
    }

    #[test]
    fn should_clear_the_read_output_on_an_unknown_wallet() {
        let contract_id = [7u8; 32];
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();
        let cursor = stale_read().cursor;
        // A null `from` is no null-pointer error: both reach the wallet
        // lookup.
        for from in [std::ptr::null(), &cursor as *const PrivateDocumentCursorFFI] {
            let mut read = stale_read();
            let result = unsafe {
                read_with(
                    resolver,
                    contract_id.as_ptr(),
                    c"secret".as_ptr(),
                    from,
                    &mut read,
                )
            };
            assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
            assert_empty(&read);
        }
    }

    #[test]
    fn should_return_a_worker_panic_as_a_wallet_operation_error() {
        // The read's worker panicking: `block_on_worker` re-raises it on the
        // calling thread, which the guard turns into a result. The export
        // itself cannot be made to panic here without a real wallet.
        let mut result = catch_read_panic("private document read", || {
            block_on_worker(async {
                panic!("worker boom");
            });
            PlatformWalletFFIResult::ok()
        });
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = text_of(result.message);
        assert!(message.starts_with("private document read panicked"));
        assert!(message.ends_with(READ_PANIC_GUIDANCE));
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    }
}

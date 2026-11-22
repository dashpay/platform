//! FFI bindings for the DPNS username marketplace on the platform-wallet
//! [`IdentityWallet`](platform_wallet::IdentityWallet): search with sale
//! state, the local name-state rows, the four trade ops (list / delist /
//! transfer / purchase), the per-name trade history, the on-demand
//! marketplace sync pass, and giving up a name the wallet identity holds
//! ([`platform_wallet_dpns_name_give_up_choices`] then
//! [`platform_wallet_dpns_give_up_name`]): a delete, which frees the name
//! for anybody to register again, or a tombstone, which nobody can undo.
//! Which of the two the network accepts for a name is decided in
//! `platform-wallet`, and this module only reports it. Both give-up entry
//! points need Platform protocol version 15: on a network proven below it
//! they return `ErrorProtocolVersionNotActive` (70) and nothing is signed.
//!
//! Wallet-layer design record:
//! `platform-wallet`. The typed rejections
//! these entry points can return (`ErrorDocumentNotForSale`,
//! `ErrorDocumentPriceChanged`, `ErrorInsufficientIdentityCredits`,
//! `ErrorContestedNameNotTradable`) are documented on
//! [`PlatformWalletFFIResultCode`](crate::error::PlatformWalletFFIResultCode);
//! three of them carry a stable JSON detail object in the result message.
//!
//! Prices are **credits** everywhere on this boundary (1 duff = 1000
//! credits). The duffs↔credits conversion is a host concern.
//!
//! Memory contract, matching the rest of the crate: every returned
//! pointer is Rust-owned and released by the paired `*_free` function in
//! this module — single values with
//! [`dpns_marketplace_name_free`], arrays with
//! [`dpns_marketplace_names_free`] / [`dpns_name_state_rows_free`] /
//! [`dpns_name_history_events_free`]. A single value and an array are
//! DIFFERENT allocations (a `Box<T>` vs a `Box<[T]>`), so the free
//! functions are not interchangeable. Empty results are reported as
//! `null` + count `0` with a success code, never as an error. The two
//! give-up results ([`DpnsNameGiveUpChoicesFFI`], [`DpnsNameGivenUpFFI`])
//! are plain data written into caller-owned structs and have no `*_free`.
//!
//! Every export here clones the wallet's identity handle under the wallet
//! registry's lock and runs the operation after releasing it. An operation
//! can wait on the host's signer and persistence callbacks, or on the DPNS
//! operation gate behind one that does, and a callback tearing down a wallet
//! (any alias) takes the registry lock for writing: held across the wait,
//! the two would wait on each other.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use dpp::prelude::Identifier;
use platform_wallet::changeset::{DpnsNameSaleStatus, DpnsNameStateEntry};
use platform_wallet::wallet::identity::network::{
    DpnsDomainState, DpnsNameGiveUpChoices, DpnsNameGiveUpMethod, DpnsNameGiveUpOutcome,
    DpnsNameGivenUp, DpnsNameHistoryEvent, DpnsNameHistoryEventKind,
};
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

// ---------------------------------------------------------------------------
// Flat result structs
// ---------------------------------------------------------------------------

/// A DPNS `domain` document read off Platform, with the marketplace
/// fields the plain name lookup drops: the document id every trade
/// transition needs, and `$price` (the sale state).
///
/// `label` / `normalized_label` are heap-allocated NUL-terminated UTF-8
/// owned by this struct. Release a single value with
/// [`dpns_marketplace_name_free`], an array with
/// [`dpns_marketplace_names_free`].
///
/// Optional fields travel as a `has_*` flag plus the value, never as a
/// sentinel: `has_price == false` means "not listed for sale", which is
/// a different fact from "listed at 0 credits".
#[repr(C)]
pub struct DpnsMarketplaceNameFFI {
    /// The domain document id — stable across transfers and purchases.
    pub document_id: [u8; 32],
    /// The document's `$ownerId`: the identity that owns (and may sell)
    /// the name.
    pub owner_id: [u8; 32],
    /// Whether `records_identity_id` is populated.
    pub has_records_identity: bool,
    /// `records.identity` — the identity the name resolves to. The
    /// protocol rewrites it to the new owner on purchase/transfer.
    /// Ignore unless `has_records_identity`.
    pub records_identity_id: [u8; 32],
    /// Display label, e.g. "Alice".
    pub label: *mut c_char,
    /// Homograph-normalized label, e.g. "a11ce".
    pub normalized_label: *mut c_char,
    /// Whether `price` is populated. `false` = the name is NOT for sale.
    pub has_price: bool,
    /// Listed sale price in credits (`$price`). Ignore unless `has_price`.
    pub price: u64,
    /// Document `$createdAt` in ms. `0` = unknown (the existing
    /// convention on this boundary for absent document timestamps).
    pub created_at_ms: u64,
    /// Document `$updatedAt` in ms — bumps on price changes. `0` = unknown.
    pub updated_at_ms: u64,
    /// Document `$transferredAt` in ms — set on purchase/transfer.
    /// `0` = unknown.
    pub transferred_at_ms: u64,
}

/// One locally persisted marketplace row: a name tracked for a wallet
/// identity, with its sale state and — for names that already left —
/// the counterparty.
///
/// Distinct from [`DpnsMarketplaceNameFFI`]: this is the wallet's own
/// bookkeeping (no network read), so it carries `wallet_identity_id` /
/// `status` / `counterparty_id` instead of the live document's
/// `$ownerId` and `records.identity`. For a `Sold`/`Transferred` row
/// the current owner IS the counterparty; for an `Owned` row it is
/// `wallet_identity_id`. Release with [`dpns_name_state_rows_free`].
#[repr(C)]
pub struct DpnsNameStateRowFFI {
    /// The domain document id — this row's key.
    pub document_id: [u8; 32],
    /// The wallet identity this row is tracked for. For `Owned` rows the
    /// document's `$ownerId`; for `Sold`/`Transferred` rows the previous
    /// owner (ours).
    pub wallet_identity_id: [u8; 32],
    /// Display label, e.g. "Alice".
    pub label: *mut c_char,
    /// Homograph-normalized label, e.g. "a11ce".
    pub normalized_label: *mut c_char,
    /// Whether `price` is populated. `false` = not listed for sale.
    pub has_price: bool,
    /// Last-known listed sale price in credits. Ignore unless `has_price`.
    pub price: u64,
    /// Ownership status relative to `wallet_identity_id`:
    /// `0` = owned, `1` = sold, `2` = transferred.
    pub status: u8,
    /// Whether `counterparty_id` is populated — true exactly when
    /// `status != 0`.
    pub has_counterparty: bool,
    /// The buyer (`status == 1`) or recipient (`status == 2`). Ignore
    /// unless `has_counterparty`.
    pub counterparty_id: [u8; 32],
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

/// One event in a name's trade timeline. All-POD (no owned strings), but
/// the array is still Rust-allocated — release it with
/// [`dpns_name_history_events_free`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DpnsNameHistoryEventFFI {
    /// What happened: `0` = registered, `1` = price set, `2` = purchased,
    /// `3` = transferred. A transfer whose `from_id == to_id` is a
    /// transfer-to-self delist.
    pub kind: u8,
    /// Block time of the event in ms.
    pub at_ms: u64,
    /// Whether `block_height` is populated.
    pub has_block_height: bool,
    /// Block height of the event. Ignore unless `has_block_height`.
    pub block_height: u64,
    /// Whether `price` is populated — true for `kind` 1 and 2.
    pub has_price: bool,
    /// Price in credits. Ignore unless `has_price`.
    pub price: u64,
    /// Whether `from_id` is populated — true for `kind` 2 and 3.
    pub has_from: bool,
    /// The seller (`kind == 2`) or sender (`kind == 3`). Ignore unless
    /// `has_from`.
    pub from_id: [u8; 32],
    /// Whether `to_id` is populated — true for `kind` 2 and 3.
    pub has_to: bool,
    /// The buyer (`kind == 2`) or recipient (`kind == 3`). Ignore unless
    /// `has_to`.
    pub to_id: [u8; 32],
}

/// One label newly observed on a wallet identity during marketplace sync.
#[repr(C)]
pub struct DpnsNameAddedFFI {
    pub identity_id: [u8; 32],
    pub label: *mut c_char,
}

/// One name that left a wallet identity during marketplace sync.
#[repr(C)]
pub struct DpnsNameDepartedFFI {
    pub identity_id: [u8; 32],
    pub label: *mut c_char,
    pub has_document_id: bool,
    pub document_id: [u8; 32],
    /// Whether the departure could be classified. When true, `status` is
    /// `1` for sold or `2` for transferred and `counterparty_id` is present.
    pub has_status: bool,
    pub status: u8,
    pub counterparty_id: [u8; 32],
}

/// One listing-price change observed during marketplace sync.
#[repr(C)]
pub struct DpnsPriceChangeFFI {
    pub document_id: [u8; 32],
    pub label: *mut c_char,
    pub has_previous: bool,
    pub previous: u64,
    pub has_current: bool,
    pub current: u64,
}

/// Lossless result of one per-wallet marketplace sync pass.
///
/// Release all owned arrays and strings with
/// [`dpns_marketplace_sync_summary_free`]. The older counts-only API remains
/// available for source and ABI compatibility.
#[repr(C)]
pub struct DpnsMarketplaceSyncSummaryFFI {
    pub names_tracked: u32,
    pub names_added: *mut DpnsNameAddedFFI,
    pub names_added_count: usize,
    pub names_departed: *mut DpnsNameDepartedFFI,
    pub names_departed_count: usize,
    pub prices_changed: *mut DpnsPriceChangeFFI,
    pub prices_changed_count: usize,
    pub sync_unix_ms: u64,
}

impl Default for DpnsMarketplaceSyncSummaryFFI {
    fn default() -> Self {
        Self {
            names_tracked: 0,
            names_added: ptr::null_mut(),
            names_added_count: 0,
            names_departed: ptr::null_mut(),
            names_departed_count: 0,
            prices_changed: ptr::null_mut(),
            prices_changed_count: 0,
            sync_unix_ms: 0,
        }
    }
}

/// `method` value of [`platform_wallet_dpns_give_up_name`] and
/// [`DpnsNameGivenUpFFI::method`]: the name's `domain` document is deleted,
/// and anybody may register the name again.
pub const DPNS_NAME_GIVE_UP_METHOD_DELETE: u8 = 0;
/// `method` value of [`platform_wallet_dpns_give_up_name`] and
/// [`DpnsNameGivenUpFFI::method`]: a `tombstone` document takes the name's
/// place, and nobody can register it again. Nobody can undo it, the
/// identity that created it included.
pub const DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE: u8 = 1;

/// Which ways the network accepts for giving up one name a wallet identity
/// holds, as [`platform_wallet_dpns_name_give_up_choices`] reports them.
/// A host offers exactly the ways whose flag is `true`.
///
/// Plain data (no heap members): it is written into a caller-owned struct
/// and has no `*_free` function.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DpnsNameGiveUpChoicesFFI {
    /// The name's `domain` document id.
    pub document_id: [u8; 32],
    /// Whether the network accepts a delete
    /// ([`DPNS_NAME_GIVE_UP_METHOD_DELETE`]) for this name.
    pub can_delete: bool,
    /// Whether the network accepts a tombstone
    /// ([`DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE`]) for this name.
    pub can_tombstone: bool,
}

/// A name given up, as the network confirmed it
/// ([`platform_wallet_dpns_give_up_name`]).
///
/// Plain data (no heap members): it is written into a caller-owned struct
/// and has no `*_free` function.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DpnsNameGivenUpFFI {
    /// The `domain` document checked before signing. A delete removed exactly
    /// this one. A tombstone consumed the owner's domain of the name at
    /// execution, which is this one unless the name was deleted and
    /// registered again in the moments between.
    pub document_id: [u8; 32],
    /// How it was given up: [`DPNS_NAME_GIVE_UP_METHOD_DELETE`] or
    /// [`DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE`].
    pub method: u8,
    /// Whether `tombstone_id` is populated: true exactly when `method` is
    /// [`DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE`].
    pub has_tombstone: bool,
    /// The id of the tombstone document the network stored. Ignore unless
    /// `has_tombstone`.
    pub tombstone_id: [u8; 32],
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

/// Heap-allocate `s` as an owned C string, or `null` if it contains an
/// interior NUL. Same fallback the DPNS label arrays use in
/// [`crate::dpns`] — the host reads a null label as an empty one rather
/// than losing the whole row.
fn owned_c_string(s: &str) -> *mut c_char {
    CString::new(s)
        .map(|c| c.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// Release a C string produced by [`owned_c_string`] and null the slot,
/// so a second free is a no-op.
///
/// # Safety
/// `slot` must be null or point at a `CString::into_raw` allocation.
unsafe fn free_owned_c_string(slot: &mut *mut c_char) {
    if !slot.is_null() {
        let _ = unsafe { CString::from_raw(*slot) };
        *slot = ptr::null_mut();
    }
}

impl DpnsMarketplaceNameFFI {
    /// Flatten a live domain state. Allocates both label strings.
    fn from_state(state: &DpnsDomainState) -> Self {
        let (has_records_identity, records_identity_id) = match state.records_identity_id {
            Some(id) => (true, id.to_buffer()),
            None => (false, [0u8; 32]),
        };
        let (has_price, price) = match state.price {
            Some(p) => (true, p),
            None => (false, 0),
        };
        Self {
            document_id: state.document_id.to_buffer(),
            owner_id: state.owner_id.to_buffer(),
            has_records_identity,
            records_identity_id,
            label: owned_c_string(&state.label),
            normalized_label: owned_c_string(&state.normalized_label),
            has_price,
            price,
            created_at_ms: state.created_at_ms.unwrap_or(0),
            updated_at_ms: state.updated_at_ms.unwrap_or(0),
            transferred_at_ms: state.transferred_at_ms.unwrap_or(0),
        }
    }
}

impl DpnsNameStateRowFFI {
    /// Flatten a persisted marketplace row. Allocates both label strings.
    fn from_entry(entry: &DpnsNameStateEntry) -> Self {
        // Wildcard-free so a new status variant is a compile error rather
        // than a silent mis-map (same discipline as `status_to_u8` in
        // `invitation_persistence`).
        let (status, has_counterparty, counterparty_id) = match entry.status {
            DpnsNameSaleStatus::Owned => (0u8, false, [0u8; 32]),
            DpnsNameSaleStatus::Sold { to } => (1u8, true, to.to_buffer()),
            DpnsNameSaleStatus::Transferred { to } => (2u8, true, to.to_buffer()),
        };
        let (has_price, price) = match entry.price {
            Some(p) => (true, p),
            None => (false, 0),
        };
        Self {
            document_id: entry.document_id.to_buffer(),
            wallet_identity_id: entry.wallet_identity_id.to_buffer(),
            label: owned_c_string(&entry.label),
            normalized_label: owned_c_string(&entry.normalized_label),
            has_price,
            price,
            status,
            has_counterparty,
            counterparty_id,
            created_at_ms: entry.created_at_ms.unwrap_or(0),
            updated_at_ms: entry.updated_at_ms.unwrap_or(0),
            transferred_at_ms: entry.transferred_at_ms.unwrap_or(0),
            last_synced_at_ms: entry.last_synced_at_ms,
        }
    }
}

impl DpnsNameHistoryEventFFI {
    /// Flatten one timeline event. All-POD — nothing to allocate.
    fn from_event(event: &DpnsNameHistoryEvent) -> Self {
        let mut out = Self {
            kind: 0,
            at_ms: event.at_ms,
            has_block_height: event.block_height.is_some(),
            block_height: event.block_height.unwrap_or(0),
            has_price: false,
            price: 0,
            has_from: false,
            from_id: [0u8; 32],
            has_to: false,
            to_id: [0u8; 32],
        };
        // Wildcard-free: a new event kind must be mapped explicitly, not
        // silently reported as a registration.
        match event.kind {
            DpnsNameHistoryEventKind::Registered => {
                out.kind = 0;
            }
            DpnsNameHistoryEventKind::PriceSet { price } => {
                out.kind = 1;
                out.has_price = true;
                out.price = price;
            }
            DpnsNameHistoryEventKind::Purchased {
                price,
                seller,
                buyer,
            } => {
                out.kind = 2;
                out.has_price = true;
                out.price = price;
                out.has_from = true;
                out.from_id = seller.to_buffer();
                out.has_to = true;
                out.to_id = buyer.to_buffer();
            }
            DpnsNameHistoryEventKind::Transferred { from, to } => {
                out.kind = 3;
                out.has_from = true;
                out.from_id = from.to_buffer();
                out.has_to = true;
                out.to_id = to.to_buffer();
            }
        }
        out
    }
}

/// Read a host's `method` byte. `None` for a value that names no method;
/// the caller refuses it before the wallet is touched.
fn give_up_method_from_u8(method: u8) -> Option<DpnsNameGiveUpMethod> {
    match method {
        DPNS_NAME_GIVE_UP_METHOD_DELETE => Some(DpnsNameGiveUpMethod::Delete),
        DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE => Some(DpnsNameGiveUpMethod::Tombstone),
        _ => None,
    }
}

/// The `method` byte of `method`. Wildcard-free, so a new method is a
/// compile error here rather than a silent mis-map.
fn give_up_method_to_u8(method: DpnsNameGiveUpMethod) -> u8 {
    match method {
        DpnsNameGiveUpMethod::Delete => DPNS_NAME_GIVE_UP_METHOD_DELETE,
        DpnsNameGiveUpMethod::Tombstone => DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE,
    }
}

impl DpnsNameGiveUpChoicesFFI {
    /// Flatten the wallet's answer. The label stays behind: the host asked
    /// with it.
    fn from_choices(choices: &DpnsNameGiveUpChoices) -> Self {
        Self {
            document_id: choices.document_id.to_buffer(),
            can_delete: choices.can_delete,
            can_tombstone: choices.can_tombstone,
        }
    }
}

impl DpnsNameGivenUpFFI {
    /// Flatten a confirmed give-up. Every field is derived from its outcome, so a
    /// tombstone id crosses exactly with a tombstone.
    fn from_given_up(given_up: &DpnsNameGivenUp) -> Self {
        let (has_tombstone, tombstone_id) = match given_up.outcome {
            DpnsNameGiveUpOutcome::Tombstoned { tombstone_id } => (true, tombstone_id.to_buffer()),
            DpnsNameGiveUpOutcome::Deleted => (false, [0u8; 32]),
        };
        Self {
            document_id: given_up.document_id.to_buffer(),
            method: give_up_method_to_u8(given_up.outcome.method()),
            has_tombstone,
            tombstone_id,
        }
    }
}

/// Move `values` into a heap array and publish it through the out-params.
/// An empty input writes `null` + `0` — an expected outcome, not an
/// error, and one the paired `*_free` tolerates.
///
/// # Safety
/// `out_ptr` / `out_count` must be valid, writable, non-null.
unsafe fn publish_array<T>(values: Vec<T>, out_ptr: *mut *mut T, out_count: *mut usize) {
    if values.is_empty() {
        unsafe {
            *out_ptr = ptr::null_mut();
            *out_count = 0;
        }
        return;
    }
    let count = values.len();
    let boxed = values.into_boxed_slice();
    unsafe {
        *out_ptr = Box::into_raw(boxed) as *mut T;
        *out_count = count;
    }
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Search DPNS names by prefix, returning full domain state (document
/// id, owner, `$price`, timestamps) ordered by normalized label.
///
/// An empty `prefix` is a valid alphabetical browse. `limit == 0` uses
/// the wallet's default page size. `start_after` is an optional 32-byte
/// cursor — pass the previous page's last `document_id` to continue, or
/// `null` for the first page.
///
/// There is no server-side price filter or ordering: `$price` is not an
/// indexable system property, so the marketplace is
/// search-driven. Release the array with [`dpns_marketplace_names_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_marketplace_search(
    wallet_handle: Handle,
    prefix: *const c_char,
    limit: u32,
    start_after: *const u8,
    out_results: *mut *mut DpnsMarketplaceNameFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(prefix);
    check_ptr!(out_results);
    check_ptr!(out_count);
    // Define the out-slots before any fallible work so an error return
    // never leaves the caller holding stack garbage to free.
    unsafe {
        *out_results = ptr::null_mut();
        *out_count = 0;
    }

    let prefix_str =
        unwrap_result_or_return!(unsafe { CStr::from_ptr(prefix) }.to_str()).to_string();
    let limit_opt = if limit == 0 { None } else { Some(limit) };
    let start_after_id = if start_after.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(unsafe {
            read_identifier(start_after)
        }))
    };

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move {
        identity
            .search_dpns_names_with_state(&prefix_str, limit_opt, start_after_id)
            .await
    });
    let states = unwrap_result_or_return!(result);

    let rows: Vec<DpnsMarketplaceNameFFI> = states
        .iter()
        .map(DpnsMarketplaceNameFFI::from_state)
        .collect();
    unsafe { publish_array(rows, out_results, out_count) };
    PlatformWalletFFIResult::ok()
}

/// Fetch the authoritative marketplace state of a single DPNS name
/// (`"alice"` or `"alice.dash"`).
///
/// A name that is not registered is an expected outcome, NOT an error:
/// the call succeeds with `*out_result == null`. Release a non-null
/// result with [`dpns_marketplace_name_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_marketplace_name_state(
    wallet_handle: Handle,
    name: *const c_char,
    out_result: *mut *mut DpnsMarketplaceNameFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(name);
    check_ptr!(out_result);
    unsafe { *out_result = ptr::null_mut() };

    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move { identity.dpns_name_state(&name_str).await });
    let state_opt = unwrap_result_or_return!(result);
    if let Some(state) = state_opt {
        let boxed = Box::new(DpnsMarketplaceNameFFI::from_state(&state));
        unsafe { *out_result = Box::into_raw(boxed) };
    }
    PlatformWalletFFIResult::ok()
}

/// Read this wallet's locally persisted marketplace rows — owned names
/// with their sale state, plus retained `Sold`/`Transferred` rows.
///
/// Pass a 32-byte `identity_id` to filter to one wallet identity, or
/// `null` for every identity in the wallet. Reads the in-memory working
/// set: no network round-trip, so this is the cheap read behind a
/// "my names" screen. Release with [`dpns_name_state_rows_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_marketplace_my_names(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_rows: *mut *mut DpnsNameStateRowFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_rows);
    check_ptr!(out_count);
    unsafe {
        *out_rows = ptr::null_mut();
        *out_count = 0;
    }

    let filter: Option<Identifier> = if identity_id.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(unsafe {
            read_identifier(identity_id)
        }))
    };

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result =
        block_on_worker(async move { identity.local_dpns_name_states(filter.as_ref()).await });
    let entries = unwrap_result_or_return!(result);

    let rows: Vec<DpnsNameStateRowFFI> = entries
        .iter()
        .map(DpnsNameStateRowFFI::from_entry)
        .collect();
    unsafe { publish_array(rows, out_rows, out_count) };
    PlatformWalletFFIResult::ok()
}

/// The trade timeline of `name`: registration, price changes, purchases
/// (with price and counterparties), and transfers — merged and ordered
/// by block time ascending.
///
/// Works for names that already left the wallet (the document id is then
/// taken from the local marketplace rows). An empty timeline writes
/// `null` + `0` with a success code. Release with
/// [`dpns_name_history_events_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_name_history(
    wallet_handle: Handle,
    name: *const c_char,
    out_events: *mut *mut DpnsNameHistoryEventFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(name);
    check_ptr!(out_events);
    check_ptr!(out_count);
    unsafe {
        *out_events = ptr::null_mut();
        *out_count = 0;
    }

    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move { identity.dpns_name_history(&name_str).await });
    let events = unwrap_result_or_return!(result);

    let rows: Vec<DpnsNameHistoryEventFFI> = events
        .iter()
        .map(DpnsNameHistoryEventFFI::from_event)
        .collect();
    unsafe { publish_array(rows, out_events, out_count) };
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Trade operations
// ---------------------------------------------------------------------------

/// List (or re-price) `name` for sale at `price_credits`.
///
/// Goes through `IdentityWallet::set_dpns_name_price`: authoritative
/// name resolution (typed contested / not-found errors), ownership
/// check, automatic AUTHENTICATION + ECDSA signing-key selection on the
/// owner, broadcast, and a local sale-state write from the CONFIRMED
/// document. `out_state` receives that confirmed state — release it with
/// [`dpns_marketplace_name_free`].
///
/// `signer_handle` must be a valid, non-destroyed handle produced by
/// `dash_sdk_signer_create_with_ctx` (typically `KeychainSigner.handle`);
/// the caller retains ownership.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_set_name_price(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    name: *const c_char,
    price_credits: u64,
    signer_handle: *mut SignerHandle,
    out_state: *mut *mut DpnsMarketplaceNameFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(name);
    check_ptr!(out_state);
    unsafe { *out_state = ptr::null_mut() };
    check_ptr!(signer_handle);

    let owner_id = unwrap_result_or_return!(unsafe { read_identifier(owner_identity_id) });
    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    // Launder the signer handle across the `Send + 'static` future bound:
    // the raw pointer is not `Send`, but the address is, and the signer is
    // guaranteed alive for the whole synchronous call by the caller's
    // ownership contract. Same idiom as `document.rs`.
    let signer_addr = signer_handle as usize;

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move {
        let signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
        identity
            .set_dpns_name_price(&owner_id, &name_str, price_credits, signer)
            .await
    });
    let state = unwrap_result_or_return!(result);
    unsafe { *out_state = Box::into_raw(Box::new(DpnsMarketplaceNameFFI::from_state(&state))) };
    PlatformWalletFFIResult::ok()
}

/// Delist `name` — remove its `$price` while keeping ownership.
///
/// Goes through `IdentityWallet::delist_dpns_name`, which broadcasts a
/// transfer to the owner's OWN identity: consensus strips `$price` on
/// transfer, and DPNS has no dedicated remove-price transition. The Rust
/// side verifies the confirmed document actually carries no `$price`
/// before recording the delist locally, so a consensus-semantics change
/// fails loudly rather than persisting a delist that didn't happen.
///
/// `out_state` receives the confirmed state — release it with
/// [`dpns_marketplace_name_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_delist_name(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    name: *const c_char,
    signer_handle: *mut SignerHandle,
    out_state: *mut *mut DpnsMarketplaceNameFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(name);
    check_ptr!(out_state);
    unsafe { *out_state = ptr::null_mut() };
    check_ptr!(signer_handle);

    let owner_id = unwrap_result_or_return!(unsafe { read_identifier(owner_identity_id) });
    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    let signer_addr = signer_handle as usize;

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move {
        let signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
        identity
            .delist_dpns_name(&owner_id, &name_str, signer)
            .await
    });
    let state = unwrap_result_or_return!(result);
    unsafe { *out_state = Box::into_raw(Box::new(DpnsMarketplaceNameFFI::from_state(&state))) };
    PlatformWalletFFIResult::ok()
}

/// Transfer `name` to `recipient_id` without payment (a gift or
/// off-market handover). Consensus strips any `$price` on transfer, so
/// this also delists.
///
/// Goes through `IdentityWallet::transfer_dpns_name`, which reconciles
/// both sides locally when they belong to this wallet. Use
/// [`platform_wallet_dpns_delist_name`] for a transfer to self — this
/// entry point rejects `recipient_id == owner_identity_id` with an
/// invalid-parameter error.
///
/// `out_state` receives the confirmed state — release it with
/// [`dpns_marketplace_name_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_transfer_name(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    name: *const c_char,
    recipient_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_state: *mut *mut DpnsMarketplaceNameFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(name);
    check_ptr!(out_state);
    unsafe { *out_state = ptr::null_mut() };
    check_ptr!(signer_handle);

    let owner_id = unwrap_result_or_return!(unsafe { read_identifier(owner_identity_id) });
    let recipient = unwrap_result_or_return!(unsafe { read_identifier(recipient_id) });
    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    let signer_addr = signer_handle as usize;

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move {
        let signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
        identity
            .transfer_dpns_name(&owner_id, &name_str, &recipient, signer)
            .await
    });
    let state = unwrap_result_or_return!(result);
    unsafe { *out_state = Box::into_raw(Box::new(DpnsMarketplaceNameFFI::from_state(&state))) };
    PlatformWalletFFIResult::ok()
}

/// Purchase `name` for `purchaser_identity_id` at exactly
/// `expected_price_credits` — the price the user confirmed.
///
/// Goes through `IdentityWallet::purchase_dpns_name`, whose pre-flight
/// is fully typed: name resolution (contested-aware), a self-purchase
/// guard, `ErrorDocumentNotForSale` (37), `ErrorDocumentPriceChanged`
/// (38) when the listing no longer matches, and
/// `ErrorInsufficientIdentityCredits` (39) when the buyer's balance
/// can't cover the price plus the fee reserve.
///
/// The broadcast transition carries `expected_price_credits`, NEVER a
/// re-read price, so a listing change between pre-flight and broadcast
/// is rejected by consensus and surfaces as the same typed code 38 — the
/// purchase does not execute at an unconfirmed price.
///
/// `out_state` receives the confirmed state (now owned by the
/// purchaser) — release it with [`dpns_marketplace_name_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_purchase_name(
    wallet_handle: Handle,
    purchaser_identity_id: *const u8,
    name: *const c_char,
    expected_price_credits: u64,
    signer_handle: *mut SignerHandle,
    out_state: *mut *mut DpnsMarketplaceNameFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(name);
    check_ptr!(out_state);
    unsafe { *out_state = ptr::null_mut() };
    check_ptr!(signer_handle);

    let purchaser_id = unwrap_result_or_return!(unsafe { read_identifier(purchaser_identity_id) });
    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    let signer_addr = signer_handle as usize;

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move {
        let signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
        identity
            .purchase_dpns_name(&purchaser_id, &name_str, expected_price_credits, signer)
            .await
    });
    let state = unwrap_result_or_return!(result);
    unsafe { *out_state = Box::into_raw(Box::new(DpnsMarketplaceNameFFI::from_state(&state))) };
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Giving up a name
// ---------------------------------------------------------------------------

/// Which ways the network accepts for `owner_identity_id` to give up
/// `name` (`"alice"` or `"alice.dash"`): a delete, a tombstone, or either.
/// Nothing is signed or broadcast.
///
/// Goes through `IdentityWallet::dpns_name_give_up_choices`, which owns
/// the rule (a contested name can only be tombstoned); a host offers
/// exactly the ways whose flag is `true` in `out_choices`.
///
/// Typed results:
/// - `ErrorProtocolVersionNotActive` (70): a verified answer shows the
///   network below Platform protocol version 15, where no name can be
///   given up.
/// - `ErrorSdkPinnedBelowProtocolVersion` (75): the SDK is pinned to a
///   version below 15, whatever the network runs.
/// - `ErrorUnknown`: the network's version could not be learned (a
///   failed version probe); the refusal may pass on a retry.
/// - `NotFound`: the name is not registered.
/// - `ErrorInvalidParameter`: another identity holds it.
///
/// `out_choices` is caller-owned and is zeroed before any fallible work, so
/// an error leaves it all-`false`. It holds no heap memory: there is
/// nothing to free.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_name_give_up_choices(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    name: *const c_char,
    out_choices: *mut DpnsNameGiveUpChoicesFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_choices);
    unsafe { *out_choices = DpnsNameGiveUpChoicesFFI::default() };
    check_ptr!(name);

    let owner_id = unwrap_result_or_return!(unsafe { read_identifier(owner_identity_id) });
    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move {
        identity
            .dpns_name_give_up_choices(&owner_id, &name_str)
            .await
    });
    let choices = unwrap_result_or_return!(result);
    unsafe { *out_choices = DpnsNameGiveUpChoicesFFI::from_choices(&choices) };
    PlatformWalletFFIResult::ok()
}

/// Give up `name`, held by the wallet identity `owner_identity_id`, by
/// `method`: [`DPNS_NAME_GIVE_UP_METHOD_DELETE`] frees the name for anybody
/// to register again; [`DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE`] removes it and
/// nobody can register it again or undo the tombstone. Any other `method`
/// value is refused with `ErrorInvalidParameter` before the wallet is
/// touched.
///
/// `expected_document_id` is the 32-byte `domain` document id the host showed
/// its user (`DpnsNameGiveUpChoicesFFI::document_id`): a replacement
/// registration the pre-signing read detects is refused with
/// `ErrorInvalidParameter`. A delete targets that document; a tombstone is
/// bound to the owner and name, so a replacement made between that read and
/// execution is the one it consumes.
///
/// Goes through `IdentityWallet::give_up_dpns_name`, which reads the name
/// again before signing (protocol version 15, registered, held by
/// `owner_identity_id`, still `expected_document_id`, `method` accepted for
/// it), selects the signing key on the owner, and broadcasts. The wallet's
/// stored state changes only once the network confirmed the name gone: the
/// persister commits the name's marketplace-row removal, then an identity
/// snapshot without the label. On an error before that nothing stored
/// changes. If the backend refuses that cleanup, the result is still a
/// success (the network confirmed it) and the next marketplace sync removes
/// the name.
///
/// Typed results:
/// - `ErrorProtocolVersionNotActive` (70): a verified answer shows the
///   network below protocol version 15. Nothing is signed.
/// - `ErrorSdkPinnedBelowProtocolVersion` (75): the SDK is pinned below 15.
/// - `ErrorUnknown` for a refusal whose version probe failed; it may pass
///   on a retry.
/// - `NotFound`: the name is not registered.
/// - `ErrorInvalidParameter`: another identity holds it, it is no longer
///   `expected_document_id`, or the network does not accept `method` for it.
/// - `ErrorTransactionBroadcastUnconfirmed` (20): the
///   transition may have been delivered; do not give the name up again
///   until a read shows whether it was.
/// - `ErrorSigningKeyUnavailable` (31): the signer has no key for the
///   selected public key.
///
/// `signer_handle` must be a valid, non-destroyed handle produced by
/// `dash_sdk_signer_create_with_ctx` (typically `KeychainSigner.handle`);
/// the caller retains ownership. `out_given_up` is caller-owned and is
/// zeroed before any fallible work. It holds no heap memory: there is
/// nothing to free.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_give_up_name(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    name: *const c_char,
    expected_document_id: *const u8,
    method: u8,
    signer_handle: *mut SignerHandle,
    out_given_up: *mut DpnsNameGivenUpFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_given_up);
    unsafe { *out_given_up = DpnsNameGivenUpFFI::default() };
    check_ptr!(name);
    check_ptr!(signer_handle);

    let Some(give_up_method) = give_up_method_from_u8(method) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "unknown DPNS name give-up method {method}: expected \
                 {DPNS_NAME_GIVE_UP_METHOD_DELETE} (delete) or \
                 {DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE} (tombstone)"
            ),
        );
    };
    let owner_id = unwrap_result_or_return!(unsafe { read_identifier(owner_identity_id) });
    let expected_id = unwrap_result_or_return!(unsafe { read_identifier(expected_document_id) });
    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    let signer_addr = signer_handle as usize;

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move {
        let signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
        identity
            .give_up_dpns_name(&owner_id, &name_str, &expected_id, give_up_method, signer)
            .await
    });
    let given_up = unwrap_result_or_return!(result);
    unsafe { *out_given_up = DpnsNameGivenUpFFI::from_given_up(&given_up) };
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// On-demand sync
// ---------------------------------------------------------------------------

/// Run one marketplace sync pass on THIS wallet and report its delta.
///
/// Refreshes owned-name rows (price / sale state), adds newly observed
/// names to the identity label lists, detects names that LEFT an
/// identity (sold or transferred away), and refreshes the balances of
/// identities that sold a name. All four out-params are optional — pass
/// `null` to ignore any of them:
///
///   * `out_names_tracked`: owned-name rows written this pass.
///   * `out_names_added`: labels newly observed on a wallet identity.
///   * `out_names_departed`: names that left a wallet identity.
///   * `out_prices_changed`: listed-price changes since the last pass.
///
/// This is the per-wallet, on-demand entry point (pull-to-refresh). The
/// recurring cross-wallet sweep is the manager-level coordinator in
/// [`crate::dpns_sync`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_marketplace_sync(
    wallet_handle: Handle,
    out_names_tracked: *mut u32,
    out_names_added: *mut u32,
    out_names_departed: *mut u32,
    out_prices_changed: *mut u32,
) -> PlatformWalletFFIResult {
    // Optional out-params: define every non-null slot before the fallible
    // work so an error return leaves well-defined zeros, not garbage.
    unsafe {
        for slot in [
            out_names_tracked,
            out_names_added,
            out_names_departed,
            out_prices_changed,
        ] {
            if !slot.is_null() {
                *slot = 0;
            }
        }
    }

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move { identity.sync_dpns_marketplace().await });
    let summary = unwrap_result_or_return!(result);

    unsafe {
        if !out_names_tracked.is_null() {
            *out_names_tracked = summary.names_tracked;
        }
        if !out_names_added.is_null() {
            *out_names_added = summary.names_added.len() as u32;
        }
        if !out_names_departed.is_null() {
            *out_names_departed = summary.names_departed.len() as u32;
        }
        if !out_prices_changed.is_null() {
            *out_prices_changed = summary.prices_changed.len() as u32;
        }
    }
    PlatformWalletFFIResult::ok()
}

/// Run one marketplace sync pass and retain the complete delta and completion
/// timestamp. This is the lossless companion to the original counts-only
/// [`platform_wallet_dpns_marketplace_sync`] entry point.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dpns_marketplace_sync_detailed(
    wallet_handle: Handle,
    out_summary: *mut DpnsMarketplaceSyncSummaryFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_summary);
    unsafe { *out_summary = DpnsMarketplaceSyncSummaryFFI::default() };

    let identity = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move { identity.sync_dpns_marketplace().await });
    let summary = unwrap_result_or_return!(result);

    let added: Vec<DpnsNameAddedFFI> = summary
        .names_added
        .into_iter()
        .map(|(identity_id, label)| DpnsNameAddedFFI {
            identity_id: identity_id.to_buffer(),
            label: owned_c_string(&label),
        })
        .collect();
    let departed: Vec<DpnsNameDepartedFFI> = summary
        .names_departed
        .into_iter()
        .map(|row| {
            let (has_status, status, counterparty_id) = match row.status {
                Some(DpnsNameSaleStatus::Sold { to }) => (true, 1, to.to_buffer()),
                Some(DpnsNameSaleStatus::Transferred { to }) => (true, 2, to.to_buffer()),
                Some(DpnsNameSaleStatus::Owned) | None => (false, 0, [0; 32]),
            };
            DpnsNameDepartedFFI {
                identity_id: row.identity_id.to_buffer(),
                label: owned_c_string(&row.label),
                has_document_id: row.document_id.is_some(),
                document_id: row.document_id.map_or([0; 32], |id| id.to_buffer()),
                has_status,
                status,
                counterparty_id,
            }
        })
        .collect();
    let prices: Vec<DpnsPriceChangeFFI> = summary
        .prices_changed
        .into_iter()
        .map(|row| DpnsPriceChangeFFI {
            document_id: row.document_id.to_buffer(),
            label: owned_c_string(&row.label),
            has_previous: row.previous.is_some(),
            previous: row.previous.unwrap_or(0),
            has_current: row.current.is_some(),
            current: row.current.unwrap_or(0),
        })
        .collect();

    let mut ffi = DpnsMarketplaceSyncSummaryFFI {
        names_tracked: summary.names_tracked,
        sync_unix_ms: summary.sync_unix_ms,
        ..Default::default()
    };
    unsafe {
        publish_array(added, &mut ffi.names_added, &mut ffi.names_added_count);
        publish_array(
            departed,
            &mut ffi.names_departed,
            &mut ffi.names_departed_count,
        );
        publish_array(
            prices,
            &mut ffi.prices_changed,
            &mut ffi.prices_changed_count,
        );
        *out_summary = ffi;
    }
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Destructors
// ---------------------------------------------------------------------------

/// Release a SINGLE [`DpnsMarketplaceNameFFI`] returned through an
/// `out_state` / `out_result` pointer — its two label strings, then the
/// value itself. No-op on `null`.
///
/// Not interchangeable with [`dpns_marketplace_names_free`]: a single
/// value is a `Box<T>`, an array is a `Box<[T]>`.
///
/// # Safety
/// `name` must be null or a pointer this module returned through a
/// single-value out-param, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn dpns_marketplace_name_free(name: *mut DpnsMarketplaceNameFFI) {
    if name.is_null() {
        return;
    }
    let mut boxed = unsafe { Box::from_raw(name) };
    unsafe {
        free_owned_c_string(&mut boxed.label);
        free_owned_c_string(&mut boxed.normalized_label);
    }
}

/// Release the owned arrays and strings inside a detailed sync summary and
/// reset it to the empty value. Safe to call on an already-empty summary.
#[no_mangle]
pub unsafe extern "C" fn dpns_marketplace_sync_summary_free(
    summary: *mut DpnsMarketplaceSyncSummaryFFI,
) {
    if summary.is_null() {
        return;
    }
    let summary = unsafe { &mut *summary };
    if !summary.names_added.is_null() && summary.names_added_count > 0 {
        let rows = unsafe {
            std::slice::from_raw_parts_mut(summary.names_added, summary.names_added_count)
        };
        for row in rows.iter_mut() {
            unsafe { free_owned_c_string(&mut row.label) };
        }
        let _ = unsafe { Box::from_raw(rows as *mut [DpnsNameAddedFFI]) };
    }
    if !summary.names_departed.is_null() && summary.names_departed_count > 0 {
        let rows = unsafe {
            std::slice::from_raw_parts_mut(summary.names_departed, summary.names_departed_count)
        };
        for row in rows.iter_mut() {
            unsafe { free_owned_c_string(&mut row.label) };
        }
        let _ = unsafe { Box::from_raw(rows as *mut [DpnsNameDepartedFFI]) };
    }
    if !summary.prices_changed.is_null() && summary.prices_changed_count > 0 {
        let rows = unsafe {
            std::slice::from_raw_parts_mut(summary.prices_changed, summary.prices_changed_count)
        };
        for row in rows.iter_mut() {
            unsafe { free_owned_c_string(&mut row.label) };
        }
        let _ = unsafe { Box::from_raw(rows as *mut [DpnsPriceChangeFFI]) };
    }
    *summary = DpnsMarketplaceSyncSummaryFFI::default();
}

/// Release an array of [`DpnsMarketplaceNameFFI`] — every row's two
/// label strings, then the array. No-op on `null` / `count == 0` (the
/// empty-result shape).
///
/// # Safety
/// `names` must be null or an array this module returned with exactly
/// `count` elements, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn dpns_marketplace_names_free(
    names: *mut DpnsMarketplaceNameFFI,
    count: usize,
) {
    if names.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(names, count) };
    for row in slice.iter_mut() {
        unsafe {
            free_owned_c_string(&mut row.label);
            free_owned_c_string(&mut row.normalized_label);
        }
    }
    let _ = unsafe { Box::from_raw(slice as *mut [DpnsMarketplaceNameFFI]) };
}

/// Release an array of [`DpnsNameStateRowFFI`] — every row's two label
/// strings, then the array. No-op on `null` / `count == 0`.
///
/// # Safety
/// `rows` must be null or an array this module returned with exactly
/// `count` elements, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn dpns_name_state_rows_free(rows: *mut DpnsNameStateRowFFI, count: usize) {
    if rows.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(rows, count) };
    for row in slice.iter_mut() {
        unsafe {
            free_owned_c_string(&mut row.label);
            free_owned_c_string(&mut row.normalized_label);
        }
    }
    let _ = unsafe { Box::from_raw(slice as *mut [DpnsNameStateRowFFI]) };
}

/// Release an array of [`DpnsNameHistoryEventFFI`]. The rows are all-POD
/// (no owned strings), so this only reclaims the array allocation.
/// No-op on `null` / `count == 0`.
///
/// # Safety
/// `events` must be null or an array this module returned with exactly
/// `count` elements, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn dpns_name_history_events_free(
    events: *mut DpnsNameHistoryEventFFI,
    count: usize,
) {
    if events.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(events, count) };
    let _ = unsafe { Box::from_raw(slice as *mut [DpnsNameHistoryEventFFI]) };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain_state(price: Option<u64>, records_identity: Option<Identifier>) -> DpnsDomainState {
        DpnsDomainState {
            document_id: Identifier::from([1u8; 32]),
            label: "Alice".to_string(),
            normalized_label: "a11ce".to_string(),
            normalized_parent_domain_name: "dash".to_string(),
            owner_id: Identifier::from([2u8; 32]),
            records_identity_id: records_identity,
            price,
            created_at_ms: Some(10),
            updated_at_ms: None,
            transferred_at_ms: Some(30),
        }
    }

    fn state_entry(status: DpnsNameSaleStatus, price: Option<u64>) -> DpnsNameStateEntry {
        DpnsNameStateEntry {
            document_id: Identifier::from([1u8; 32]),
            wallet_identity_id: Identifier::from([2u8; 32]),
            label: "Alice".to_string(),
            normalized_label: "a11ce".to_string(),
            normalized_parent_domain_name: "dash".to_string(),
            price,
            status,
            created_at_ms: Some(10),
            updated_at_ms: Some(20),
            transferred_at_ms: None,
            last_synced_at_ms: 99,
        }
    }

    /// Absent optionals must cross as `has_* == false`, never as a
    /// fabricated value: "not for sale" and "listed at 0 credits" are
    /// different facts, and so are "no `$updatedAt`" and "updated at
    /// epoch".
    #[test]
    fn marketplace_name_absent_optionals_are_flagged_not_fabricated() {
        let mut ffi = DpnsMarketplaceNameFFI::from_state(&domain_state(None, None));
        assert!(!ffi.has_price);
        assert_eq!(ffi.price, 0);
        assert!(!ffi.has_records_identity);
        assert_eq!(ffi.records_identity_id, [0u8; 32]);
        assert_eq!(ffi.updated_at_ms, 0);
        assert_eq!(ffi.created_at_ms, 10);
        assert_eq!(ffi.transferred_at_ms, 30);
        unsafe {
            free_owned_c_string(&mut ffi.label);
            free_owned_c_string(&mut ffi.normalized_label);
        }
    }

    #[test]
    fn marketplace_name_round_trips_present_fields() {
        let records = Identifier::from([3u8; 32]);
        let ffi = DpnsMarketplaceNameFFI::from_state(&domain_state(Some(5_000), Some(records)));
        assert_eq!(ffi.document_id, [1u8; 32]);
        assert_eq!(ffi.owner_id, [2u8; 32]);
        assert!(ffi.has_records_identity);
        assert_eq!(ffi.records_identity_id, [3u8; 32]);
        assert!(ffi.has_price);
        assert_eq!(ffi.price, 5_000);
        let label = unsafe { CStr::from_ptr(ffi.label) }
            .to_string_lossy()
            .into_owned();
        let normalized = unsafe { CStr::from_ptr(ffi.normalized_label) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(label, "Alice");
        assert_eq!(normalized, "a11ce");
        // Free through the public single-value destructor, which is what
        // the host calls.
        unsafe { dpns_marketplace_name_free(Box::into_raw(Box::new(ffi))) };
    }

    /// The status discriminants are the ABI contract with the host's
    /// `DpnsNameSaleStatus` mirror; pin all three plus their
    /// counterparty flags.
    #[test]
    fn name_state_row_pins_status_discriminants() {
        let buyer = Identifier::from([7u8; 32]);
        let cases = [
            (DpnsNameSaleStatus::Owned, 0u8, false, [0u8; 32]),
            (DpnsNameSaleStatus::Sold { to: buyer }, 1u8, true, [7u8; 32]),
            (
                DpnsNameSaleStatus::Transferred { to: buyer },
                2u8,
                true,
                [7u8; 32],
            ),
        ];
        for (status, expected_status, expected_has_cp, expected_cp) in cases {
            let row = DpnsNameStateRowFFI::from_entry(&state_entry(status, Some(1)));
            assert_eq!(row.status, expected_status);
            assert_eq!(row.has_counterparty, expected_has_cp);
            assert_eq!(row.counterparty_id, expected_cp);
            assert_eq!(row.last_synced_at_ms, 99);
            free_rows(vec![row]);
        }
    }

    #[test]
    fn name_state_row_unlisted_price_is_flagged() {
        let row = DpnsNameStateRowFFI::from_entry(&state_entry(DpnsNameSaleStatus::Owned, None));
        assert!(!row.has_price);
        assert_eq!(row.price, 0);
        assert_eq!(row.transferred_at_ms, 0);
        free_rows(vec![row]);
    }

    /// Publish `rows` exactly as an entry point would, then release them
    /// through the public destructor — so the tests exercise the real
    /// allocation shape (`Box<[T]>`) rather than a hand-rolled one.
    fn free_rows(rows: Vec<DpnsNameStateRowFFI>) {
        let mut out: *mut DpnsNameStateRowFFI = ptr::null_mut();
        let mut count: usize = 0;
        unsafe {
            publish_array(rows, &mut out, &mut count);
            dpns_name_state_rows_free(out, count);
        }
    }

    /// The event-kind discriminants and which optional payloads each kind
    /// carries are both ABI contracts — a host reading `price` on a
    /// registration event, or mistaking a purchase for a transfer, shows
    /// the user a wrong trade history.
    #[test]
    fn history_event_kinds_and_payloads_are_pinned() {
        let seller = Identifier::from([4u8; 32]);
        let buyer = Identifier::from([5u8; 32]);

        let registered = DpnsNameHistoryEventFFI::from_event(&DpnsNameHistoryEvent {
            kind: DpnsNameHistoryEventKind::Registered,
            at_ms: 1,
            block_height: None,
        });
        assert_eq!(registered.kind, 0);
        assert!(!registered.has_price);
        assert!(!registered.has_from);
        assert!(!registered.has_to);
        assert!(!registered.has_block_height);
        assert_eq!(registered.block_height, 0);

        let priced = DpnsNameHistoryEventFFI::from_event(&DpnsNameHistoryEvent {
            kind: DpnsNameHistoryEventKind::PriceSet { price: 42 },
            at_ms: 2,
            block_height: Some(1_000),
        });
        assert_eq!(priced.kind, 1);
        assert!(priced.has_price);
        assert_eq!(priced.price, 42);
        assert!(!priced.has_from);
        assert!(priced.has_block_height);
        assert_eq!(priced.block_height, 1_000);

        let purchased = DpnsNameHistoryEventFFI::from_event(&DpnsNameHistoryEvent {
            kind: DpnsNameHistoryEventKind::Purchased {
                price: 7,
                seller,
                buyer,
            },
            at_ms: 3,
            block_height: None,
        });
        assert_eq!(purchased.kind, 2);
        assert!(purchased.has_price);
        assert_eq!(purchased.price, 7);
        // Purchase: from = seller, to = buyer.
        assert_eq!(purchased.from_id, [4u8; 32]);
        assert_eq!(purchased.to_id, [5u8; 32]);

        let transferred = DpnsNameHistoryEventFFI::from_event(&DpnsNameHistoryEvent {
            kind: DpnsNameHistoryEventKind::Transferred {
                from: seller,
                to: buyer,
            },
            at_ms: 4,
            block_height: None,
        });
        assert_eq!(transferred.kind, 3);
        assert!(!transferred.has_price);
        assert_eq!(transferred.from_id, [4u8; 32]);
        assert_eq!(transferred.to_id, [5u8; 32]);
    }

    /// Every destructor must tolerate the empty-result shape (`null` +
    /// `0`) and a bare `null`, since that is exactly what a
    /// no-results-but-successful call publishes.
    #[test]
    fn destructors_are_null_and_empty_tolerant() {
        unsafe {
            dpns_marketplace_name_free(ptr::null_mut());
            dpns_marketplace_names_free(ptr::null_mut(), 0);
            dpns_marketplace_names_free(ptr::null_mut(), 3);
            dpns_name_state_rows_free(ptr::null_mut(), 0);
            dpns_name_history_events_free(ptr::null_mut(), 0);
            dpns_marketplace_sync_summary_free(ptr::null_mut());
        }
    }

    #[test]
    fn detailed_sync_summary_frees_every_nested_array() {
        let mut summary = DpnsMarketplaceSyncSummaryFFI {
            names_tracked: 3,
            sync_unix_ms: 99,
            ..Default::default()
        };
        unsafe {
            publish_array(
                vec![DpnsNameAddedFFI {
                    identity_id: [1; 32],
                    label: owned_c_string("Alice"),
                }],
                &mut summary.names_added,
                &mut summary.names_added_count,
            );
            publish_array(
                vec![DpnsNameDepartedFFI {
                    identity_id: [2; 32],
                    label: owned_c_string("Bob"),
                    has_document_id: true,
                    document_id: [3; 32],
                    has_status: true,
                    status: 1,
                    counterparty_id: [4; 32],
                }],
                &mut summary.names_departed,
                &mut summary.names_departed_count,
            );
            publish_array(
                vec![DpnsPriceChangeFFI {
                    document_id: [5; 32],
                    label: owned_c_string("Carol"),
                    has_previous: false,
                    previous: 0,
                    has_current: true,
                    current: 7,
                }],
                &mut summary.prices_changed,
                &mut summary.prices_changed_count,
            );
            dpns_marketplace_sync_summary_free(&mut summary);
        }
        assert_eq!(summary.names_tracked, 0);
        assert_eq!(summary.sync_unix_ms, 0);
        assert!(summary.names_added.is_null());
        assert!(summary.names_departed.is_null());
        assert!(summary.prices_changed.is_null());
    }

    /// `publish_array` on an empty Vec must write the documented
    /// `null` + `0` pair rather than a dangling one-past-the-end pointer,
    /// and the paired free must accept it.
    #[test]
    fn publish_array_writes_the_empty_shape() {
        // Seed the out-slots with garbage a caller could mistake for a
        // real result, so the assertions below prove they were OVERWRITTEN
        // rather than merely left alone.
        let mut out: *mut DpnsMarketplaceNameFFI = std::ptr::dangling_mut();
        let mut count: usize = 7;
        unsafe { publish_array(Vec::new(), &mut out, &mut count) };
        assert!(out.is_null());
        assert_eq!(count, 0);
        unsafe { dpns_marketplace_names_free(out, count) };
    }

    /// A populated array round-trips through `publish_array` and its
    /// destructor without leaking the per-row label strings (verified
    /// under the test harness's allocator; a double free would abort).
    #[test]
    fn publish_array_round_trips_and_frees_rows() {
        let states = [
            domain_state(Some(1), None),
            domain_state(None, Some(Identifier::from([6u8; 32]))),
        ];
        let rows: Vec<DpnsMarketplaceNameFFI> = states
            .iter()
            .map(DpnsMarketplaceNameFFI::from_state)
            .collect();
        let mut out: *mut DpnsMarketplaceNameFFI = ptr::null_mut();
        let mut count: usize = 0;
        unsafe { publish_array(rows, &mut out, &mut count) };
        assert!(!out.is_null());
        assert_eq!(count, 2);
        let first_label = unsafe { CStr::from_ptr((*out).label) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(first_label, "Alice");
        unsafe { dpns_marketplace_names_free(out, count) };
    }

    /// Unknown handles must surface as `NotFound` through
    /// `unwrap_option_or_return!` rather than dereferencing a stale slot,
    /// and required out-pointers must be rejected first with
    /// `ErrorNullPointer`. Covers the pointer-discipline half of every
    /// entry point without needing a live wallet.
    #[test]
    fn unknown_handle_and_null_out_pointers_are_rejected() {
        let bogus: Handle = 0xDEAD_BEEF;

        let mut names: *mut DpnsMarketplaceNameFFI = ptr::null_mut();
        let mut count: usize = 0;
        let prefix = CString::new("a").unwrap();
        let r = unsafe {
            platform_wallet_dpns_marketplace_search(
                bogus,
                prefix.as_ptr(),
                0,
                ptr::null(),
                &mut names,
                &mut count,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let mut detailed = DpnsMarketplaceSyncSummaryFFI::default();
        let r = unsafe { platform_wallet_dpns_marketplace_sync_detailed(bogus, &mut detailed) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
        assert!(detailed.names_added.is_null());
        assert_eq!(detailed.sync_unix_ms, 0);
        assert!(names.is_null());
        assert_eq!(count, 0);

        let mut rows: *mut DpnsNameStateRowFFI = ptr::null_mut();
        let mut rows_count: usize = 0;
        let r = unsafe {
            platform_wallet_dpns_marketplace_my_names(
                bogus,
                ptr::null(),
                &mut rows,
                &mut rows_count,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        // All four sync out-params are optional — a null-only call must
        // still reach the handle lookup.
        let r = unsafe {
            platform_wallet_dpns_marketplace_sync(
                bogus,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        // Required out-pointer missing: rejected before the handle lookup.
        let c = CString::new("alice").unwrap();
        let r = unsafe {
            platform_wallet_dpns_marketplace_name_state(bogus, c.as_ptr(), ptr::null_mut())
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        // Missing signer handle: rejected before the handle lookup too.
        let mut state: *mut DpnsMarketplaceNameFFI = ptr::null_mut();
        let r = unsafe {
            platform_wallet_dpns_set_name_price(
                bogus,
                [0u8; 32].as_ptr(),
                c.as_ptr(),
                1,
                ptr::null_mut(),
                &mut state,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(state.is_null());
    }

    /// The method bytes are the ABI contract with the host's give-up
    /// mirror: each one reads back as the method it names and writes back
    /// as the same byte, and no other byte names a method.
    #[test]
    fn should_map_give_up_methods_to_their_bytes_both_ways() {
        for (method, byte) in [
            (DpnsNameGiveUpMethod::Delete, 0u8),
            (DpnsNameGiveUpMethod::Tombstone, 1u8),
        ] {
            assert_eq!(give_up_method_to_u8(method), byte);
            assert_eq!(give_up_method_from_u8(byte), Some(method));
        }
        assert_eq!(DPNS_NAME_GIVE_UP_METHOD_DELETE, 0);
        assert_eq!(DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE, 1);
        for byte in 2..=u8::MAX {
            assert_eq!(give_up_method_from_u8(byte), None, "byte {byte}");
        }
    }

    #[test]
    fn should_flatten_give_up_choices_with_both_flags_as_reported() {
        for (can_delete, can_tombstone) in [(true, true), (false, true)] {
            let ffi = DpnsNameGiveUpChoicesFFI::from_choices(&DpnsNameGiveUpChoices {
                document_id: Identifier::from([1u8; 32]),
                label: "Alice".to_string(),
                can_delete,
                can_tombstone,
            });
            assert_eq!(ffi.document_id, [1u8; 32]);
            assert_eq!(ffi.can_delete, can_delete);
            assert_eq!(ffi.can_tombstone, can_tombstone);
        }
    }

    /// A tombstone id crosses only with its flag, and a delete carries no
    /// fabricated one.
    #[test]
    fn should_flatten_a_given_up_name_with_the_tombstone_only_when_present() {
        let deleted = DpnsNameGivenUpFFI::from_given_up(&DpnsNameGivenUp {
            document_id: Identifier::from([1u8; 32]),
            label: "Alice".to_string(),
            outcome: DpnsNameGiveUpOutcome::Deleted,
        });
        assert_eq!(deleted.document_id, [1u8; 32]);
        assert_eq!(deleted.method, DPNS_NAME_GIVE_UP_METHOD_DELETE);
        assert!(!deleted.has_tombstone);
        assert_eq!(deleted.tombstone_id, [0u8; 32]);

        let tombstoned = DpnsNameGivenUpFFI::from_given_up(&DpnsNameGivenUp {
            document_id: Identifier::from([1u8; 32]),
            label: "Alice".to_string(),
            outcome: DpnsNameGiveUpOutcome::Tombstoned {
                tombstone_id: Identifier::from([9u8; 32]),
            },
        });
        assert_eq!(tombstoned.method, DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE);
        assert!(tombstoned.has_tombstone);
        assert_eq!(tombstoned.tombstone_id, [9u8; 32]);
    }

    /// An unknown method is refused as a bad argument before the handle is
    /// looked up: an unknown handle would otherwise answer `NotFound`.
    #[test]
    fn should_refuse_an_unknown_give_up_method_before_touching_the_wallet() {
        let bogus: Handle = 0xDEAD_BEEF;
        let name = CString::new("alice").unwrap();
        // Any non-null address passes the signer null check; the call
        // returns before it could be dereferenced.
        let signer = ptr::NonNull::<SignerHandle>::dangling().as_ptr();
        let mut out = DpnsNameGivenUpFFI {
            method: 7,
            has_tombstone: true,
            ..Default::default()
        };
        let r = unsafe {
            platform_wallet_dpns_give_up_name(
                bogus,
                [0u8; 32].as_ptr(),
                name.as_ptr(),
                [1u8; 32].as_ptr(),
                2,
                signer,
                &mut out,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
        assert_eq!(out, DpnsNameGivenUpFFI::default());

        let r = unsafe {
            platform_wallet_dpns_give_up_name(
                bogus,
                [0u8; 32].as_ptr(),
                name.as_ptr(),
                [1u8; 32].as_ptr(),
                DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE,
                signer,
                &mut out,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
    }

    /// Required pointers are rejected with `ErrorNullPointer`, an unknown
    /// handle with `NotFound`, and every out-struct is left zeroed.
    #[test]
    fn should_reject_null_pointers_and_unknown_handles_on_give_up_entry_points() {
        let bogus: Handle = 0xDEAD_BEEF;
        let name = CString::new("alice").unwrap();
        let owner = [0u8; 32];

        let mut choices = DpnsNameGiveUpChoicesFFI {
            can_delete: true,
            can_tombstone: true,
            ..Default::default()
        };
        let r = unsafe {
            platform_wallet_dpns_name_give_up_choices(
                bogus,
                owner.as_ptr(),
                name.as_ptr(),
                &mut choices,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
        assert_eq!(choices, DpnsNameGiveUpChoicesFFI::default());

        let r = unsafe {
            platform_wallet_dpns_name_give_up_choices(
                bogus,
                owner.as_ptr(),
                name.as_ptr(),
                ptr::null_mut(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        choices.can_delete = true;
        choices.document_id = [9u8; 32];
        let r = unsafe {
            platform_wallet_dpns_name_give_up_choices(
                bogus,
                owner.as_ptr(),
                ptr::null(),
                &mut choices,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(choices, DpnsNameGiveUpChoicesFFI::default());

        let signer = ptr::NonNull::<SignerHandle>::dangling().as_ptr();
        let mut stale = DpnsNameGivenUpFFI {
            method: 1,
            has_tombstone: true,
            tombstone_id: [9u8; 32],
            ..Default::default()
        };
        let r = unsafe {
            platform_wallet_dpns_give_up_name(
                bogus,
                owner.as_ptr(),
                ptr::null(),
                [1u8; 32].as_ptr(),
                DPNS_NAME_GIVE_UP_METHOD_DELETE,
                signer,
                &mut stale,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(stale, DpnsNameGivenUpFFI::default());

        let mut given_up = DpnsNameGivenUpFFI::default();
        let r = unsafe {
            platform_wallet_dpns_give_up_name(
                bogus,
                owner.as_ptr(),
                name.as_ptr(),
                [1u8; 32].as_ptr(),
                DPNS_NAME_GIVE_UP_METHOD_DELETE,
                ptr::null_mut(),
                &mut given_up,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(given_up, DpnsNameGivenUpFFI::default());

        let signer = ptr::NonNull::<SignerHandle>::dangling().as_ptr();
        let r = unsafe {
            platform_wallet_dpns_give_up_name(
                bogus,
                owner.as_ptr(),
                name.as_ptr(),
                [1u8; 32].as_ptr(),
                DPNS_NAME_GIVE_UP_METHOD_DELETE,
                signer,
                ptr::null_mut(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }
}

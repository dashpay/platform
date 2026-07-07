//! FFI bindings for DPNS name operations on the platform-wallet
//! [`IdentityWallet`](platform_wallet::IdentityWallet).

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Flat FFI result from [`platform_wallet_search_dpns_names`].
///
/// `label` is heap-allocated NUL-terminated UTF-8 owned by the
/// caller — release with [`dpns_search_results_free`] on the whole
/// array. `identity_id` is a 32-byte inline buffer.
#[repr(C)]
pub struct DpnsSearchResultFFI {
    pub identity_id: [u8; 32],
    pub label: *mut c_char,
}

/// Register a DPNS name for an identity on Platform using an
/// externally-supplied signer.
///
/// The wallet handle is still used to look up the identity from the
/// in-process `IdentityManager` so we can pick the HIGH/CRITICAL
/// authentication key the document state transition requires — but
/// every signature crosses the FFI through the supplied
/// `signer_handle`. Works on watch-only wallets (no seed Rust-side).
///
/// Returns the full domain name (e.g. "alice.dash") via
/// `out_full_domain_name` — a heap-allocated C-string the caller must
/// release with [`crate::platform_wallet_string_free`].
///
/// On success the just-registered name is appended to
/// `ManagedIdentity.dpns_names` on the Rust side and an identity
/// changeset is queued so the Swift persister observes the update via
/// `on_persist_identities_fn`. `signer_handle` must be a valid,
/// non-destroyed handle produced by `dash_sdk_signer_create_with_ctx`
/// (typically `KeychainSigner.handle`); the caller retains ownership.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_register_dpns_name_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    name: *const c_char,
    signer_handle: *mut SignerHandle,
    out_full_domain_name: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(name);
    check_ptr!(out_full_domain_name);
    // Null the out-pointer before the fallible work below (identifier/name
    // parsing, the async registration) so an error return never leaves it
    // holding stack garbage for a cleanup-on-error caller to
    // `platform_wallet_string_free`.
    unsafe { *out_full_domain_name = std::ptr::null_mut() };
    check_ptr!(signer_handle);

    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });
    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
            identity_wallet
                .register_name_with_external_signer(&id, &name_str, signer)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let full_name = unwrap_result_or_return!(result);
    let cstr = unwrap_result_or_return!(CString::new(full_name));
    unsafe { *out_full_domain_name = cstr.into_raw() };
    PlatformWalletFFIResult::ok()
}

/// Resolve a DPNS name (`"alice"` or `"alice.dash"`) to an identity id.
///
/// `out_found` reports whether the lookup returned a hit. When `true`,
/// `out_identity_id` is populated; when `false`, `out_identity_id` is
/// zeroed.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_resolve_dpns_name(
    wallet_handle: Handle,
    name: *const c_char,
    out_identity_id: *mut u8,
    out_found: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(name);
    check_ptr!(out_identity_id);
    check_ptr!(out_found);

    let name_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(name) }.to_str()).to_string();

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.resolve_name(&name_str).await })
    });
    let result = unwrap_option_or_return!(option);
    let resolved = unwrap_result_or_return!(result);
    match resolved {
        Some(id) => unsafe {
            write_identifier(out_identity_id, &id);
            *out_found = true;
        },
        None => unsafe {
            std::ptr::write_bytes(out_identity_id, 0u8, 32);
            *out_found = false;
        },
    }
    PlatformWalletFFIResult::ok()
}

/// Prefix search over DPNS documents on Platform.
///
/// Returns a heap-allocated array of [`DpnsSearchResultFFI`] values
/// via `out_results` / `out_count`. Release the whole array (plus
/// each entry's `label` C-string) by calling
/// [`dpns_search_results_free`].
///
/// `limit` is an advisory cap; pass `0` to defer to the SDK's
/// default. The SDK currently caps the response at 100 documents.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_search_dpns_names(
    wallet_handle: Handle,
    prefix: *const c_char,
    limit: u32,
    out_results: *mut *mut DpnsSearchResultFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(prefix);
    check_ptr!(out_results);
    check_ptr!(out_count);

    let prefix_str =
        unwrap_result_or_return!(unsafe { CStr::from_ptr(prefix) }.to_str()).to_string();
    let sdk_limit = if limit == 0 { None } else { Some(limit) };

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.search_names(&prefix_str, sdk_limit).await })
    });
    let result = unwrap_option_or_return!(option);
    let list = unwrap_result_or_return!(result);

    use dash_sdk::platform::dpns_usernames::DpnsUsername;
    if list.is_empty() {
        unsafe {
            *out_results = ptr::null_mut();
            *out_count = 0;
        }
        return PlatformWalletFFIResult::ok();
    }
    let mut buf: Vec<DpnsSearchResultFFI> = Vec::with_capacity(list.len());
    for u in list {
        let DpnsUsername {
            full_name,
            owner_id,
            ..
        } = u;
        let c = CString::new(full_name)
            .map(|c| c.into_raw())
            .unwrap_or(ptr::null_mut());
        buf.push(DpnsSearchResultFFI {
            identity_id: owner_id.to_buffer(),
            label: c,
        });
    }
    let count = buf.len();
    let boxed = buf.into_boxed_slice();
    let array_ptr = Box::into_raw(boxed) as *mut DpnsSearchResultFFI;
    unsafe {
        *out_results = array_ptr;
        *out_count = count;
    }
    PlatformWalletFFIResult::ok()
}

/// Release an array previously returned by
/// [`platform_wallet_search_dpns_names`]. Walks the array to release
/// every `label` C-string before releasing the array itself. Safe to
/// call with `results = null` / `count = 0` — both are no-ops.
#[no_mangle]
pub unsafe extern "C" fn dpns_search_results_free(results: *mut DpnsSearchResultFFI, count: usize) {
    if results.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(results, count) };
    for entry in slice.iter_mut() {
        if !entry.label.is_null() {
            let _ = unsafe { CString::from_raw(entry.label) };
            entry.label = ptr::null_mut();
        }
    }
    let _ = unsafe { Box::from_raw(slice as *mut [DpnsSearchResultFFI]) };
}

/// Fetch DPNS usernames for `identity_id` from Platform and merge
/// them into `ManagedIdentity.dpns_names`. Returns the number of
/// newly-added labels via `out_added` (unchanged when the cache
/// already contains every name).
///
/// Use this from iOS load paths instead of
/// `dash_sdk_dpns_get_usernames_by_identity` directly — the wallet
/// path updates the persister changeset on success so
/// `PersistentIdentity` refreshes via the
/// `on_persist_identities_fn` callback, and the local cache is the
/// source of truth for subsequent reads without another round-trip.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_sync_dpns_names(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_added: *mut u32,
) -> PlatformWalletFFIResult {
    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.sync_dpns_names(&id).await })
    });
    let result = unwrap_option_or_return!(option);
    let added = unwrap_result_or_return!(result);
    if !out_added.is_null() {
        unsafe { *out_added = added };
    }
    PlatformWalletFFIResult::ok()
}

/// Heap-allocated array of DPNS labels returned by
/// [`managed_identity_get_dpns_names`]. Each `labels[i]` is a
/// NUL-terminated UTF-8 C-string owned by the array and released
/// wholesale by [`dpns_name_array_free`].
#[repr(C)]
pub struct DpnsNameArray {
    pub labels: *mut *mut c_char,
    pub count: usize,
}

impl DpnsNameArray {
    pub fn empty() -> Self {
        Self {
            labels: std::ptr::null_mut(),
            count: 0,
        }
    }
}

/// Read the cached DPNS labels for a [`ManagedIdentity`] handle.
///
/// Returns the labels from
/// [`ManagedIdentity.dpns_names`](platform_wallet::ManagedIdentity).
/// Empty when nothing has been synced yet — follow with
/// [`platform_wallet_sync_dpns_names`] to populate. Release the
/// returned array via [`dpns_name_array_free`].
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_dpns_names(
    identity_handle: Handle,
    out_array: *mut DpnsNameArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity
            .dpns_names
            .iter()
            .map(|i| i.label.clone())
            .collect::<Vec<_>>()
    });
    let labels = unwrap_option_or_return!(option);
    if labels.is_empty() {
        unsafe { *out_array = DpnsNameArray::empty() };
        return PlatformWalletFFIResult::ok();
    }
    let mut raw_labels: Vec<*mut c_char> = Vec::with_capacity(labels.len());
    for label in labels {
        let c = match CString::new(label) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        };
        raw_labels.push(c);
    }
    let count = raw_labels.len();
    let boxed = raw_labels.into_boxed_slice();
    let ptr = Box::into_raw(boxed) as *mut *mut c_char;
    unsafe { *out_array = DpnsNameArray { labels: ptr, count } };
    PlatformWalletFFIResult::ok()
}

/// Release an array previously returned by
/// [`managed_identity_get_dpns_names`]. Walks the array to free every
/// label C-string before releasing the array itself. Safe to call
/// with `labels = null` / `count = 0`, and with a null outer pointer
/// (no-op).
///
/// Pointer-only signature: `DpnsNameArray` is a 16-byte aggregate at
/// the AAPCS64 / Swift-ABI cliff, so by-value isn't safe across
/// `@_silgen_name`. Caller passes `&mut array`; on return the
/// pointer + count are reset so a double-free no-ops.
#[no_mangle]
pub unsafe extern "C" fn dpns_name_array_free(array: *mut DpnsNameArray) {
    if array.is_null() {
        return;
    }
    let array = unsafe { &mut *array };
    if array.labels.is_null() || array.count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(array.labels, array.count) };
    for entry in slice.iter_mut() {
        if !entry.is_null() {
            let _ = unsafe { CString::from_raw(*entry) };
            *entry = std::ptr::null_mut();
        }
    }
    let _ = unsafe { Box::from_raw(slice as *mut [*mut c_char]) };
    array.labels = std::ptr::null_mut();
    array.count = 0;
}

/// One contender row in [`ContestVoteStateFFI`]. Plain scalar struct
/// (no owned allocations) — reclaimed wholesale when the parent's
/// `contenders_ptr` array is freed.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ContestContenderFFI {
    pub identity_id: [u8; 32],
    pub vote_tally: u32,
}

/// Winner-kind discriminant for [`ContestVoteStateFFI`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContestWinnerKindFFI {
    None = 0,
    WonByIdentity = 1,
    Locked = 2,
}

/// Flat FFI snapshot of a DPNS contest's vote state, returned by
/// [`platform_wallet_fetch_contest_vote_state`].
///
/// Caller owns `label` + `contenders_ptr`; both are released by
/// [`contest_vote_state_ffi_free`]. Safe to call free on an
/// all-null snapshot (the default / "not found" state).
/// `winner_identity_id` is only populated when `winner_kind` is
/// `WonByIdentity` (1); ignore the field for `None` / `Locked`.
#[repr(C)]
pub struct ContestVoteStateFFI {
    pub label: *mut c_char,
    pub end_time_ms: u64,
    pub contenders_ptr: *mut ContestContenderFFI,
    pub contenders_count: usize,
    pub abstain_votes: u32,
    pub lock_votes: u32,
    pub winner_kind: u8,
    pub winner_identity_id: [u8; 32],
}

impl ContestVoteStateFFI {
    pub fn empty() -> Self {
        Self {
            label: std::ptr::null_mut(),
            end_time_ms: 0,
            contenders_ptr: std::ptr::null_mut(),
            contenders_count: 0,
            abstain_votes: 0,
            lock_votes: 0,
            winner_kind: ContestWinnerKindFFI::None as u8,
            winner_identity_id: [0u8; 32],
        }
    }
}

/// Fetch the current vote state for a DPNS contest `identity_id` is
/// contending for. `out_found` signals whether the lookup returned a
/// hit; `out_state` is populated only when `out_found` is `true`.
/// Release `out_state` with [`contest_vote_state_ffi_free`] whether
/// or not `out_found` was set — free is a no-op on the empty /
/// zeroed struct that the "not found" path leaves behind.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_fetch_contest_vote_state(
    wallet_handle: Handle,
    identity_id: *const u8,
    label: *const c_char,
    out_state: *mut ContestVoteStateFFI,
    out_found: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(label);
    check_ptr!(out_state);
    check_ptr!(out_found);

    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });
    let label_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(label) }.to_str()).to_string();

    // Pre-clear the out struct so a downstream early-return leaves
    // the caller looking at well-defined empty state, never garbage.
    unsafe {
        *out_state = ContestVoteStateFFI::empty();
        *out_found = false;
    }

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.contest_vote_state(&id, &label_str).await })
    });
    let result = unwrap_option_or_return!(option);
    let state_opt = unwrap_result_or_return!(result);
    match state_opt {
        Some(state) => {
            use platform_wallet::wallet::identity::network::ContestWinner;

            let label_c = CString::new(state.label)
                .map(|c| c.into_raw())
                .unwrap_or(std::ptr::null_mut());

            let (contenders_ptr, contenders_count) = if state.contenders.is_empty() {
                (std::ptr::null_mut(), 0usize)
            } else {
                let buf: Vec<ContestContenderFFI> = state
                    .contenders
                    .into_iter()
                    .map(|c| ContestContenderFFI {
                        identity_id: c.identity_id.to_buffer(),
                        vote_tally: c.vote_tally,
                    })
                    .collect();
                let count = buf.len();
                let boxed = buf.into_boxed_slice();
                let ptr = Box::into_raw(boxed) as *mut ContestContenderFFI;
                (ptr, count)
            };

            let (winner_kind, winner_identity_id) = match state.winner {
                ContestWinner::None => (ContestWinnerKindFFI::None as u8, [0u8; 32]),
                ContestWinner::WonByIdentity(id) => {
                    (ContestWinnerKindFFI::WonByIdentity as u8, id.to_buffer())
                }
                ContestWinner::Locked => (ContestWinnerKindFFI::Locked as u8, [0u8; 32]),
            };

            unsafe {
                *out_state = ContestVoteStateFFI {
                    label: label_c,
                    end_time_ms: state.end_time_ms,
                    contenders_ptr,
                    contenders_count,
                    abstain_votes: state.abstain_votes,
                    lock_votes: state.lock_votes,
                    winner_kind,
                    winner_identity_id,
                };
                *out_found = true;
            }
            PlatformWalletFFIResult::ok()
        }
        None => {
            unsafe {
                *out_state = ContestVoteStateFFI::empty();
                *out_found = false;
            }
            PlatformWalletFFIResult::ok()
        }
    }
}

/// Release heap allocations owned by a [`ContestVoteStateFFI`] —
/// the `label` C-string and the `contenders_ptr` array. Safe on an
/// `empty()` snapshot (every owned pointer is null-checked) and on
/// a null outer pointer (no-op).
///
/// Pointer-only signature: `ContestVoteStateFFI` is a heavyweight
/// aggregate well over the 16-byte AAPCS64 / Swift cliff. After the
/// call the owned pointers are reset so a double-free no-ops.
#[no_mangle]
pub unsafe extern "C" fn contest_vote_state_ffi_free(state: *mut ContestVoteStateFFI) {
    if state.is_null() {
        return;
    }
    let state = unsafe { &mut *state };
    if !state.label.is_null() {
        let _ = unsafe { CString::from_raw(state.label) };
        state.label = std::ptr::null_mut();
    }
    if !state.contenders_ptr.is_null() && state.contenders_count > 0 {
        let slice =
            unsafe { std::slice::from_raw_parts_mut(state.contenders_ptr, state.contenders_count) };
        let _ = unsafe { Box::from_raw(slice as *mut [ContestContenderFFI]) };
    }
    state.contenders_ptr = std::ptr::null_mut();
    state.contenders_count = 0;
}

/// Fetch the non-resolved contested DPNS names `identity_id` is a
/// contender for and replace
/// [`ManagedIdentity.contested_dpns_names`](platform_wallet::ManagedIdentity)
/// wholesale with the canonical set. Writes a full snapshot via the
/// persister (not dedup-append) so resolved contests disappear from
/// the local cache on sync. Returns the new count via `out_count`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_sync_contested_dpns_names(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_count: *mut u32,
) -> PlatformWalletFFIResult {
    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.sync_contested_dpns_names(&id).await })
    });
    let result = unwrap_option_or_return!(option);
    let labels = unwrap_result_or_return!(result);
    if !out_count.is_null() {
        unsafe { *out_count = labels.len() as u32 };
    }
    PlatformWalletFFIResult::ok()
}

/// Read the cached contested DPNS labels for a [`ManagedIdentity`]
/// handle. Returns an empty [`DpnsNameArray`] when the cache hasn't
/// been populated; follow with
/// [`platform_wallet_sync_contested_dpns_names`] to refresh. Release
/// via [`dpns_name_array_free`].
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_contested_dpns_names(
    identity_handle: Handle,
    out_array: *mut DpnsNameArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity.contested_dpns_names.clone()
    });
    let labels = unwrap_option_or_return!(option);
    if labels.is_empty() {
        unsafe { *out_array = DpnsNameArray::empty() };
        return PlatformWalletFFIResult::ok();
    }
    let mut raw_labels: Vec<*mut c_char> = Vec::with_capacity(labels.len());
    for label in labels {
        let c = match CString::new(label) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        };
        raw_labels.push(c);
    }
    let count = raw_labels.len();
    let boxed = raw_labels.into_boxed_slice();
    let ptr = Box::into_raw(boxed) as *mut *mut c_char;
    unsafe { *out_array = DpnsNameArray { labels: ptr, count } };
    PlatformWalletFFIResult::ok()
}

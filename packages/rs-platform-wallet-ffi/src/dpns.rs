//! FFI bindings for DPNS name operations on the platform-wallet
//! [`IdentityWallet`](platform_wallet::IdentityWallet).
//!
//! Three entry points:
//!
//! 1. [`platform_wallet_register_dpns_name`] — register a DPNS name
//!    for an identity. Runs on the 8 MB tokio worker (proof
//!    verification recurses), updates `ManagedIdentity.dpns_names`
//!    on success, and persists via the identity changeset so the
//!    Swift persister callback from `identity_persistence` will
//!    refresh `PersistentIdentity.dpnsName` automatically.
//!
//! 2. [`platform_wallet_resolve_dpns_name`] — resolve a DPNS name
//!    to an identity id. Async; no persistence side-effects.
//!
//! 3. [`platform_wallet_search_dpns_names`] — prefix search over
//!    Platform's DPNS documents. Async; returns a heap-allocated
//!    array of `DpnsSearchResultFFI` releasable via
//!    [`dpns_search_results_free`].
//!
//! Replaces the direct `dash_sdk_dpns_*` paths the iOS app was
//! using for DPNS writes — those paths are still functional but
//! bypass the identity manager + changeset layer, leaving
//! `ManagedIdentity.dpns_names` and `PersistentIdentity.dpnsName`
//! out of sync with on-chain state until the next sync. Routing
//! through this module fixes the drift.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;

/// Flat FFI result from [`platform_wallet_search_dpns_names`].
///
/// `label` is heap-allocated NUL-terminated UTF-8 owned by the
/// caller — release with [`dpns_search_results_free`] on the whole
/// array. `identity_id` is a 32-byte inline buffer.
#[repr(C)]
pub struct DpnsSearchResultFFI {
    /// Identity that owns the DPNS name.
    pub identity_id: [u8; 32],
    /// Fully-qualified label (e.g. "alice.dash").
    pub label: *mut c_char,
}

/// Register a DPNS name for an identity on Platform.
///
/// Returns the full domain name (e.g. "alice.dash") via
/// `out_full_domain_name` — a heap-allocated C-string the caller
/// must release with [`crate::platform_wallet_string_free`].
///
/// On success the just-registered name is appended to
/// `ManagedIdentity.dpns_names` on the Rust side and an identity
/// changeset is queued so the Swift persister observes the update
/// via `on_persist_identities_fn`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_register_dpns_name(
    wallet_handle: Handle,
    identity_id: *const u8,
    name: *const c_char,
    out_full_domain_name: *mut *mut c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if name.is_null() || out_full_domain_name.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "name or out_full_domain_name is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let id = match unsafe { read_identifier(identity_id) } {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identity identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "name is not valid UTF-8",
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result =
                block_on_worker(async move { identity.register_name(&id, &name_str).await });
            match result {
                Ok(full_name) => match CString::new(full_name) {
                    Ok(cstr) => {
                        unsafe { *out_full_domain_name = cstr.into_raw() };
                        PlatformWalletFFIResult::Success
                    }
                    Err(_) => {
                        // The returned domain name should never carry
                        // an interior NUL, but guard against it in
                        // case a future label encoding changes.
                        if !out_error.is_null() {
                            unsafe {
                                *out_error = PlatformWalletFFIError::new(
                                    PlatformWalletFFIResult::ErrorSerialization,
                                    "full domain name contained NUL",
                                );
                            }
                        }
                        PlatformWalletFFIResult::ErrorSerialization
                    }
                },
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("register_dpns_name failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Resolve a DPNS name (`"alice"` or `"alice.dash"`) to an identity id.
///
/// `out_found` reports whether the lookup returned a hit. When `true`,
/// `out_identity_id` is populated.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_resolve_dpns_name(
    wallet_handle: Handle,
    name: *const c_char,
    out_identity_id: *mut u8,
    out_found: *mut bool,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if name.is_null() || out_identity_id.is_null() || out_found.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided to resolve_dpns_name",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "name is not valid UTF-8",
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result = block_on_worker(async move { identity.resolve_name(&name_str).await });
            match result {
                Ok(Some(id)) => {
                    unsafe {
                        write_identifier(out_identity_id, &id);
                        *out_found = true;
                    }
                    PlatformWalletFFIResult::Success
                }
                Ok(None) => {
                    unsafe {
                        // Zero out the 32-byte buffer for a clean
                        // "not found" return value.
                        std::ptr::write_bytes(out_identity_id, 0u8, 32);
                        *out_found = false;
                    }
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("resolve_dpns_name failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
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
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if prefix.is_null() || out_results.is_null() || out_count.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided to search_dpns_names",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let prefix_str = match unsafe { CStr::from_ptr(prefix) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "prefix is not valid UTF-8",
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };
    // Rust-side takes `Option<u32>`; `0` means "default cap".
    let sdk_limit = if limit == 0 { None } else { Some(limit) };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result =
                block_on_worker(async move { identity.search_names(&prefix_str, sdk_limit).await });
            match result {
                Ok(list) => {
                    // Build the FFI array — each entry owns its label
                    // C-string via `CString::into_raw`. On the free
                    // side, `dpns_search_results_free` walks the array
                    // to reclaim every label before releasing the
                    // array itself.
                    use dash_sdk::platform::dpns_usernames::DpnsUsername;
                    if list.is_empty() {
                        unsafe {
                            *out_results = ptr::null_mut();
                            *out_count = 0;
                        }
                        return PlatformWalletFFIResult::Success;
                    }
                    let mut buf: Vec<DpnsSearchResultFFI> = Vec::with_capacity(list.len());
                    for u in list {
                        // DpnsUsername carries label + normalized_label
                        // + full_name + owner_id; we surface the full
                        // user-visible "alice.dash" plus the owning
                        // identity id.
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
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("search_dpns_names failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
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

// ---------------------------------------------------------------------------
// DPNS cache sync + read (per-identity)
// ---------------------------------------------------------------------------

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
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let id = match unsafe { read_identifier(identity_id) } {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identity identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result = block_on_worker(async move { identity.sync_dpns_names(&id).await });
            match result {
                Ok(added) => {
                    if !out_added.is_null() {
                        unsafe { *out_added = added };
                    }
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("sync_dpns_names failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
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
/// [`platform_wallet_sync_dpns_names`] to populate.
///
/// Release the returned array via [`dpns_name_array_free`].
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_dpns_names(
    identity_handle: Handle,
    out_array: *mut DpnsNameArray,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_array.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_array is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            if identity.dpns_names.is_empty() {
                unsafe { *out_array = DpnsNameArray::empty() };
                return PlatformWalletFFIResult::Success;
            }
            // Build a vector of owned C-string pointers; the array
            // itself is heap-allocated and released with every
            // label by `dpns_name_array_free`.
            let mut labels: Vec<*mut c_char> = Vec::with_capacity(identity.dpns_names.len());
            for info in &identity.dpns_names {
                let c = match CString::new(info.label.clone()) {
                    Ok(c) => c.into_raw(),
                    // NUL in label is unreachable in practice;
                    // keep the entry but surface as a null pointer
                    // so the caller's iteration doesn't crash.
                    Err(_) => std::ptr::null_mut(),
                };
                labels.push(c);
            }
            let count = labels.len();
            let boxed = labels.into_boxed_slice();
            let ptr = Box::into_raw(boxed) as *mut *mut c_char;
            unsafe {
                *out_array = DpnsNameArray { labels: ptr, count };
            }
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid managed identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
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

// ---------------------------------------------------------------------------
// Contest vote state (ephemeral — not cached)
// ---------------------------------------------------------------------------

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
///
/// `winner_identity_id` is only populated when `winner_kind` is
/// `WonByIdentity` (1); ignore the field for `None` / `Locked`.
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
#[repr(C)]
pub struct ContestVoteStateFFI {
    /// Heap-owned NUL-terminated UTF-8 label. `null` only on an
    /// empty/default-initialized struct.
    pub label: *mut c_char,
    /// Voting end time in milliseconds since epoch.
    pub end_time_ms: u64,
    /// Heap-owned contender array. `null` with `contenders_count = 0`
    /// when the contest has no listed contenders yet.
    pub contenders_ptr: *mut ContestContenderFFI,
    pub contenders_count: usize,
    pub abstain_votes: u32,
    pub lock_votes: u32,
    /// Winner discriminant. Maps to [`ContestWinnerKindFFI`].
    pub winner_kind: u8,
    /// Populated only when `winner_kind == WonByIdentity`.
    pub winner_identity_id: [u8; 32],
}

impl ContestVoteStateFFI {
    /// All-null/zeroed snapshot used as the out-param initial
    /// value. Writing an empty before the FFI call means the "not
    /// found" path leaves a well-defined struct that
    /// [`contest_vote_state_ffi_free`] can safely no-op on.
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

/// Fetch the current vote state for a DPNS contest `identity_id`
/// is contending for. `out_found` signals whether the lookup
/// returned a hit; `out_state` is populated only when `out_found`
/// is `true`. Release `out_state` with
/// [`contest_vote_state_ffi_free`] whether or not `out_found` was
/// set — free is a no-op on the empty / zeroed struct that the
/// "not found" path leaves behind.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_fetch_contest_vote_state(
    wallet_handle: Handle,
    identity_id: *const u8,
    label: *const c_char,
    out_state: *mut ContestVoteStateFFI,
    out_found: *mut bool,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if label.is_null() || out_state.is_null() || out_found.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided to fetch_contest_vote_state",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let id = match unsafe { read_identifier(identity_id) } {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identity identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "label is not valid UTF-8",
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result =
                block_on_worker(async move { identity.contest_vote_state(&id, &label_str).await });
            match result {
                Ok(Some(state)) => {
                    use platform_wallet::wallet::identity::network::ContestWinner;

                    // Heap-alloc the label + contenders; ownership
                    // moves to the caller, released by
                    // `contest_vote_state_ffi_free`.
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
                    PlatformWalletFFIResult::Success
                }
                Ok(None) => {
                    unsafe {
                        *out_state = ContestVoteStateFFI::empty();
                        *out_found = false;
                    }
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    unsafe {
                        *out_state = ContestVoteStateFFI::empty();
                        *out_found = false;
                    }
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("fetch_contest_vote_state failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
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

// ---------------------------------------------------------------------------
// Contested DPNS names
// ---------------------------------------------------------------------------

/// Fetch the non-resolved contested DPNS names `identity_id` is a
/// contender for and replace
/// [`ManagedIdentity.contested_dpns_names`](platform_wallet::ManagedIdentity)
/// wholesale with the canonical set. Writes a full snapshot via
/// the persister (not dedup-append) so resolved contests disappear
/// from the local cache on sync. Returns the new count via
/// `out_count`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_sync_contested_dpns_names(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_count: *mut u32,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let id = match unsafe { read_identifier(identity_id) } {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identity identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result =
                block_on_worker(async move { identity.sync_contested_dpns_names(&id).await });
            match result {
                Ok(labels) => {
                    if !out_count.is_null() {
                        unsafe { *out_count = labels.len() as u32 };
                    }
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("sync_contested_dpns_names failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Read the cached contested DPNS labels for a [`ManagedIdentity`]
/// handle. Returns an empty [`DpnsNameArray`] when the cache hasn't
/// been populated; follow with
/// [`platform_wallet_sync_contested_dpns_names`] to refresh.
/// Release via [`dpns_name_array_free`].
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_contested_dpns_names(
    identity_handle: Handle,
    out_array: *mut DpnsNameArray,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_array.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_array is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            if identity.contested_dpns_names.is_empty() {
                unsafe { *out_array = DpnsNameArray::empty() };
                return PlatformWalletFFIResult::Success;
            }
            let mut labels: Vec<*mut c_char> =
                Vec::with_capacity(identity.contested_dpns_names.len());
            for label in &identity.contested_dpns_names {
                let c = match CString::new(label.clone()) {
                    Ok(c) => c.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                };
                labels.push(c);
            }
            let count = labels.len();
            let boxed = labels.into_boxed_slice();
            let ptr = Box::into_raw(boxed) as *mut *mut c_char;
            unsafe {
                *out_array = DpnsNameArray { labels: ptr, count };
            }
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid managed identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

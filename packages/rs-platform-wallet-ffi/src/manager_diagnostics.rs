//! Read-only diagnostic accessors mirroring `PlatformWalletManager`'s
//! `*_blocking` snapshot methods on the FFI surface.
//!
//! Every fn here follows the same pattern as
//! [`crate::wallet::platform_wallet_manager_get_account_balances`]:
//! validate pointers, read the snapshot via the paired
//! `_blocking` accessor on `PlatformWalletManager`, marshal into a
//! heap-owned `[T]`, and hand back `(*const T, count)` via out-params.
//! Each non-trivial allocation has a paired `*_free` fn so the Swift
//! caller can reclaim it.
//!
//! These are bridges, not policy. No iteration, no decision logic —
//! the snapshot logic lives upstream on
//! [`platform_wallet::manager::accessors`].

use std::ffi::CString;
use std::os::raw::c_char;

use platform_wallet::manager::accessors::{
    AccountAddressInfoSnapshot, AccountAddressPoolSnapshot, AccountMetadataSnapshot,
    AccountTransactionSnapshot, AccountUtxoSnapshot, AddressBanInfoSnapshot,
    CoreWalletStateSnapshot, IdentitySyncConfigSnapshot, IdentityWalletStateSnapshot,
    PlatformAddressProviderStateSnapshot, PlatformAddressSyncConfigSnapshot,
    TrackedAssetLockSnapshot, WalletIdentityRowSnapshot,
};

use crate::check_ptr;
use crate::core_wallet_types::{
    AccountAddressPoolEntryFFI, AccountMetadataFFI, AccountTransactionEntryFFI,
    AccountUtxoEntryFFI, AddressBanInfoFFI, AddressInfoFFI, CoreWalletStateFFI,
    IdentitySyncConfigFFI, IdentityWalletStateFFI, OutPointFFI,
    PlatformAddressProviderStateFFI, PlatformAddressSyncConfigFFI,
    TrackedAssetLockEntryFFI, WalletIdentityRowFFI,
};
use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::handle::{Handle, PLATFORM_WALLET_MANAGER_STORAGE};
use crate::wallet_restore_types::AccountSpecFFI;

// ---------------------------------------------------------------------------
// Phase 2 — Manager-level diagnostic snapshots
// ---------------------------------------------------------------------------

/// Atomic snapshot of every wallet id currently registered on the
/// manager. Wallet ids are written as a flat `count * 32`-byte buffer.
/// Caller frees via [`platform_wallet_manager_free_wallet_ids`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_list_wallet_ids(
    manager_handle: Handle,
    out_bytes: *mut *const u8,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_bytes);
    check_ptr!(out_count);
    *out_bytes = std::ptr::null();
    *out_count = 0;

    let Some(ids) =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| m.list_wallet_ids_blocking())
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if ids.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let count = ids.len();
    let mut flat: Vec<u8> = Vec::with_capacity(count * 32);
    for id in &ids {
        flat.extend_from_slice(id);
    }
    let boxed = flat.into_boxed_slice();
    *out_bytes = Box::into_raw(boxed) as *const u8;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_free_wallet_ids(bytes: *mut u8, count: usize) {
    if !bytes.is_null() && count > 0 {
        let total = count * 32;
        // `Box::from_raw(slice_from_raw_parts_mut(...))` mirrors the
        // producer side (`Box::into_raw(vec.into_boxed_slice())`)
        // exactly. Using `Vec::from_raw_parts` would technically work
        // because `into_boxed_slice` shrinks capacity to length, but
        // its documented contract is "originally allocated by Vec";
        // the boxed-slice form is unambiguous and matches the rest
        // of this file.
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, total));
    }
}

/// Snapshot of the platform-address sync coordinator's config /
/// counters. Single-struct, no allocation — the result is written
/// in-place into `out_state`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_config(
    manager_handle: Handle,
    out_state: *mut PlatformAddressSyncConfigFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_state);
    let Some(snap): Option<PlatformAddressSyncConfigSnapshot> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| {
            m.platform_address_sync_config_blocking()
        })
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    *out_state = PlatformAddressSyncConfigFFI {
        interval_seconds: snap.interval_seconds,
        watch_list_size: snap.watch_list_size,
        last_event_unix_seconds: snap.last_event_unix_seconds,
    };
    PlatformWalletFFIResult::ok()
}

/// Snapshot of the identity sync coordinator's config / queue depth.
/// Single-struct, no allocation.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_config(
    manager_handle: Handle,
    out_state: *mut IdentitySyncConfigFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_state);
    let Some(snap): Option<IdentitySyncConfigSnapshot> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| m.identity_sync_config_blocking())
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    *out_state = IdentitySyncConfigFFI {
        interval_seconds: snap.interval_seconds,
        queue_depth: snap.queue_depth,
    };
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Phase 3 — Per-wallet state
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_core_wallet_state(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_state: *mut CoreWalletStateFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_state);
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let Some(snap_opt): Option<Option<CoreWalletStateSnapshot>> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| m.core_wallet_state_blocking(&wid))
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    let Some(snap) = snap_opt else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "Wallet not found".to_string(),
        );
    };
    *out_state = CoreWalletStateFFI {
        synced_height: snap.synced_height,
        last_processed_height: snap.last_processed_height,
        monitor_revision: snap.monitor_revision,
    };
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_wallet_state(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_state: *mut IdentityWalletStateFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_state);
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let Some(snap_opt): Option<Option<IdentityWalletStateSnapshot>> =
        PLATFORM_WALLET_MANAGER_STORAGE
            .with_item(manager_handle, |m| m.identity_wallet_state_blocking(&wid))
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    let Some(snap) = snap_opt else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "Wallet not found".to_string(),
        );
    };
    *out_state = IdentityWalletStateFFI {
        last_scanned_index: snap.last_scanned_index,
        scan_pending: snap.scan_pending,
    };
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_platform_address_provider_state(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_state: *mut PlatformAddressProviderStateFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_state);
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let Some(snap_opt): Option<Option<PlatformAddressProviderStateSnapshot>> =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| {
            m.platform_address_provider_state_blocking(&wid)
        })
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    let Some(snap) = snap_opt else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "Wallet not found".to_string(),
        );
    };
    *out_state = PlatformAddressProviderStateFFI {
        initialized: snap.initialized,
        accounts_watched: snap.accounts_watched,
        found_count: snap.found_count,
        known_balances_count: snap.known_balances_count,
        watermark_height: snap.watermark_height,
    };
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Phase 4 — Wallet metadata + floating state
// ---------------------------------------------------------------------------

// `platform_wallet_info_metadata` + `_free` were removed: every field
// they exposed (name, description, birth_height, synced_height,
// last_processed_height, total_transactions, first_loaded_at) was
// either duplicated by `platform_wallet_core_wallet_state` or had no
// active populator on this path. Re-add the surface only if a future
// caller needs name/description specifically.

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_tracked_asset_locks_list(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_entries: *mut *const TrackedAssetLockEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_entries);
    check_ptr!(out_count);
    *out_entries = std::ptr::null();
    *out_count = 0;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let Some(rows): Option<Vec<TrackedAssetLockSnapshot>> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| m.tracked_asset_locks_blocking(&wid))
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if rows.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let entries: Vec<TrackedAssetLockEntryFFI> = rows
        .into_iter()
        .map(|s| TrackedAssetLockEntryFFI {
            outpoint_txid: txid_to_array(&s.outpoint.txid),
            outpoint_vout: s.outpoint.vout,
            lock_type: s.lock_type,
            status: s.status,
            registration_index: s.registration_index,
            instant_lock_present: s.instant_lock_present,
            chain_lock_height: s.chain_lock_height,
        })
        .collect();
    let count = entries.len();
    let boxed = entries.into_boxed_slice();
    *out_entries = Box::into_raw(boxed) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_tracked_asset_locks_free(
    entries: *mut TrackedAssetLockEntryFFI,
    count: usize,
) {
    if !entries.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
    }
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_instant_send_locks(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_bytes: *mut *const u8,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_bytes);
    check_ptr!(out_count);
    *out_bytes = std::ptr::null();
    *out_count = 0;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let Some(txids) = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| m.instant_send_locks_blocking(&wid))
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if txids.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let count = txids.len();
    let mut flat: Vec<u8> = Vec::with_capacity(count * 32);
    for tx in &txids {
        flat.extend_from_slice(tx.as_ref());
    }
    let boxed = flat.into_boxed_slice();
    *out_bytes = Box::into_raw(boxed) as *const u8;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_instant_send_locks_free(bytes: *mut u8, count: usize) {
    // See `platform_wallet_manager_free_wallet_ids` for the rationale
    // on the `Box::from_raw` / `slice_from_raw_parts_mut` form (matches
    // the producer's `Box::into_raw(vec.into_boxed_slice())`).
    if !bytes.is_null() && count > 0 {
        let total = count * 32;
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, total));
    }
}

// ---------------------------------------------------------------------------
// Phase 5 — Per-account drill-down
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_metadata(
    manager_handle: Handle,
    wallet_id: *const u8,
    spec: *const AccountSpecFFI,
    out_meta: *mut AccountMetadataFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(spec);
    check_ptr!(out_meta);
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let target = match account_type_from_spec_ref(&*spec) {
        Ok(at) => at,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e,
            );
        }
    };
    let Some(snap_opt): Option<Option<AccountMetadataSnapshot>> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| {
            m.account_metadata_blocking(&wid, &target)
        })
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    let Some(snap) = snap_opt else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "Account not found".to_string(),
        );
    };
    *out_meta = AccountMetadataFFI {
        total_transactions: snap.total_transactions,
        total_utxos: snap.total_utxos,
        monitor_revision: snap.monitor_revision,
    };
    PlatformWalletFFIResult::ok()
}

/// Stable no-op — `AccountMetadataFFI` no longer carries heap-owned
/// fields after `is_watch_only` / `custom_name` were dropped. Kept on
/// the FFI surface so the Swift caller doesn't need to special-case
/// the freed-by-caller idiom on a single struct.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_metadata_free(_meta: *mut AccountMetadataFFI) {}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_address_pools(
    manager_handle: Handle,
    wallet_id: *const u8,
    spec: *const AccountSpecFFI,
    out_pools: *mut *const AccountAddressPoolEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(spec);
    check_ptr!(out_pools);
    check_ptr!(out_count);
    *out_pools = std::ptr::null();
    *out_count = 0;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let target = match account_type_from_spec_ref(&*spec) {
        Ok(at) => at,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e,
            );
        }
    };
    let Some(pools): Option<Vec<AccountAddressPoolSnapshot>> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| {
            m.account_address_pools_blocking(&wid, &target)
        })
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if pools.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let entries: Vec<AccountAddressPoolEntryFFI> = pools
        .into_iter()
        .map(|pool| {
            let addr_count = pool.addresses.len();
            let addrs: Vec<AddressInfoFFI> = pool
                .addresses
                .into_iter()
                .map(|a: AccountAddressInfoSnapshot| {
                    // Heap-allocate the address string + pubkey bytes;
                    // freed by `platform_wallet_account_address_pools_free`.
                    let address = optional_into_raw(Some(a.address));
                    let (pk_ptr, pk_len) = if a.public_key_bytes.is_empty() {
                        (std::ptr::null_mut(), 0usize)
                    } else {
                        let len = a.public_key_bytes.len();
                        let boxed = a.public_key_bytes.into_boxed_slice();
                        (Box::into_raw(boxed) as *mut u8, len)
                    };
                    AddressInfoFFI {
                        pubkey_hash: a.pubkey_hash,
                        address_index: a.address_index,
                        is_used: a.is_used,
                        last_used_height: 0,
                        address,
                        public_key_bytes: pk_ptr,
                        public_key_bytes_len: pk_len,
                    }
                })
                .collect();
            let addresses_ptr = if addr_count == 0 {
                std::ptr::null_mut()
            } else {
                Box::into_raw(addrs.into_boxed_slice()) as *mut AddressInfoFFI
            };
            AccountAddressPoolEntryFFI {
                pool_type: pool.pool_type,
                gap_limit: pool.gap_limit,
                last_used_index: pool.last_used_index,
                addresses: addresses_ptr,
                addresses_count: addr_count,
            }
        })
        .collect();
    let count = entries.len();
    let boxed = entries.into_boxed_slice();
    *out_pools = Box::into_raw(boxed) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_address_pools_free(
    pools: *mut AccountAddressPoolEntryFFI,
    count: usize,
) {
    if pools.is_null() || count == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts(pools, count);
    for entry in slice {
        if !entry.addresses.is_null() && entry.addresses_count > 0 {
            // Walk every per-address row first to release its
            // heap-owned `address` C string and `public_key_bytes`
            // buffer before the parent slice is reclaimed.
            let addrs = std::slice::from_raw_parts(entry.addresses, entry.addresses_count);
            for a in addrs {
                if !a.address.is_null() {
                    let _ = CString::from_raw(a.address);
                }
                if !a.public_key_bytes.is_null() && a.public_key_bytes_len > 0 {
                    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        a.public_key_bytes,
                        a.public_key_bytes_len,
                    ));
                }
            }
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                entry.addresses,
                entry.addresses_count,
            ));
        }
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(pools, count));
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_utxos(
    manager_handle: Handle,
    wallet_id: *const u8,
    spec: *const AccountSpecFFI,
    out_utxos: *mut *const AccountUtxoEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(spec);
    check_ptr!(out_utxos);
    check_ptr!(out_count);
    *out_utxos = std::ptr::null();
    *out_count = 0;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let target = match account_type_from_spec_ref(&*spec) {
        Ok(at) => at,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e,
            );
        }
    };
    let Some(rows): Option<Vec<AccountUtxoSnapshot>> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| m.account_utxos_blocking(&wid, &target))
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if rows.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let entries: Vec<AccountUtxoEntryFFI> = rows.into_iter().map(utxo_entry_ffi).collect();
    let count = entries.len();
    let boxed = entries.into_boxed_slice();
    *out_utxos = Box::into_raw(boxed) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

/// One snapshot row to its FFI entry, heap-owning the script bytes. Shared
/// by the paged and unpaged exports so their row shape — and the
/// `platform_wallet_account_utxos_free` contract both rely on — stays one
/// thing.
fn utxo_entry_ffi(s: AccountUtxoSnapshot) -> AccountUtxoEntryFFI {
    let script_len = s.script_pubkey.len();
    let script_ptr = if script_len == 0 {
        std::ptr::null_mut()
    } else {
        Box::into_raw(s.script_pubkey.into_boxed_slice()) as *mut u8
    };
    AccountUtxoEntryFFI {
        outpoint_txid: txid_to_array(&s.outpoint.txid),
        outpoint_vout: s.outpoint.vout,
        value_duffs: s.value_duffs,
        script_pubkey: script_ptr,
        script_pubkey_len: script_len,
        height: s.height,
        is_locked: s.is_locked,
    }
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_utxos_free(
    utxos: *mut AccountUtxoEntryFFI,
    count: usize,
) {
    if utxos.is_null() || count == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts(utxos, count);
    for entry in slice {
        if !entry.script_pubkey.is_null() && entry.script_pubkey_len > 0 {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                entry.script_pubkey,
                entry.script_pubkey_len,
            ));
        }
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(utxos, count));
}

/// One outpoint-ordered page of an account's UTXO inventory — the bounded
/// form of `platform_wallet_account_utxos`.
///
/// A wallet's UTXO count is chain-controlled (anyone who knows a watched
/// address can keep sending dust to it), so a periodic host-side audit
/// must never materialize the whole inventory at once. `after_txid` +
/// `after_vout` name the last outpoint of the previous page; pass a NULL
/// `after_txid` to start at the beginning. `limit` caps the rows returned
/// (0 means "no limit" — a paging caller should always pass a real cap),
/// and `out_has_more` reports whether further pages remain.
///
/// Rows are freed with `platform_wallet_account_utxos_free`, the same
/// entry type and the same deallocator as the unpaged call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_utxos_page(
    manager_handle: Handle,
    wallet_id: *const u8,
    spec: *const AccountSpecFFI,
    after_txid: *const u8,
    after_vout: u32,
    limit: usize,
    out_utxos: *mut *const AccountUtxoEntryFFI,
    out_count: *mut usize,
    out_has_more: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(spec);
    check_ptr!(out_utxos);
    check_ptr!(out_count);
    check_ptr!(out_has_more);
    *out_utxos = std::ptr::null();
    *out_count = 0;
    *out_has_more = false;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let target = match account_type_from_spec_ref(&*spec) {
        Ok(at) => at,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e,
            );
        }
    };
    // A NULL cursor is "from the beginning" — the only way to say it, since
    // the all-zero txid is a legal (if unreachable) outpoint.
    let after = if after_txid.is_null() {
        None
    } else {
        let raw: [u8; 32] = std::ptr::read(after_txid as *const [u8; 32]);
        Some(dashcore::OutPoint::from(&OutPointFFI {
            txid: raw,
            vout: after_vout,
        }))
    };
    let Some((rows, has_more)) = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| {
        m.account_utxos_page_blocking(&wid, &target, after, limit)
    }) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    *out_has_more = has_more;
    if rows.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let entries: Vec<AccountUtxoEntryFFI> = rows.into_iter().map(utxo_entry_ffi).collect();
    let count = entries.len();
    *out_utxos = Box::into_raw(entries.into_boxed_slice()) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

/// Classify `count` outpoints against the wallet's live engine state, in
/// one pass under one read lock. `out_classes` receives a `count`-byte
/// buffer, positionally aligned with the input: 0 unknown, 1 unspent, 2
/// spent (see `platform_wallet::manager::accessors::OUTPOINT_CLASS_*`).
///
/// The inverse direction of `platform_wallet_account_utxos_page`: a host
/// that mirrors the wallet's TXOs pages its OWN rows and asks about them
/// in batches, so neither side ever holds a full engine inventory. Cost is
/// the batch size times the account count, never the inventory size.
///
/// `2` means some recorded transaction spends the outpoint — including one
/// still in the mempool. It is not proof of a settled spend.
///
/// Free the verdicts with `platform_wallet_classify_outpoints_free`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_classify_outpoints(
    manager_handle: Handle,
    wallet_id: *const u8,
    outpoints: *const OutPointFFI,
    count: usize,
    out_classes: *mut *const u8,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_classes);
    check_ptr!(out_count);
    *out_classes = std::ptr::null();
    *out_count = 0;
    if count == 0 {
        return PlatformWalletFFIResult::ok();
    }
    check_ptr!(outpoints);
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let requested: Vec<dashcore::OutPoint> = std::slice::from_raw_parts(outpoints, count)
        .iter()
        .map(dashcore::OutPoint::from)
        .collect();
    let Some(classes) = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| {
        m.classify_outpoints_blocking(&wid, &requested)
    }) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    let len = classes.len();
    *out_classes = Box::into_raw(classes.into_boxed_slice()) as *const u8;
    *out_count = len;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_classify_outpoints_free(classes: *mut u8, count: usize) {
    if classes.is_null() || count == 0 {
        return;
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(classes, count));
}

/// The account's spent-outpoint inventory — the second half of the
/// store-reconcile surface (`platform_wallet_account_utxos` is the unspent
/// half). A persistence-mirror row still marked unspent whose outpoint
/// appears here lost its spend update (dashpay/platform#4425); a row in
/// NEITHER inventory is swept/abandoned residue (pre-rust-dashcore#971
/// stores). Free with `platform_wallet_account_spent_outpoints_free`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_spent_outpoints(
    manager_handle: Handle,
    wallet_id: *const u8,
    spec: *const AccountSpecFFI,
    out_outpoints: *mut *const OutPointFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(spec);
    check_ptr!(out_outpoints);
    check_ptr!(out_count);
    *out_outpoints = std::ptr::null();
    *out_count = 0;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let target = match account_type_from_spec_ref(&*spec) {
        Ok(at) => at,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e,
            );
        }
    };
    let Some(rows) = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| m.account_spent_outpoints_blocking(&wid, &target))
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if rows.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let entries: Vec<OutPointFFI> = rows.iter().map(OutPointFFI::from).collect();
    let count = entries.len();
    *out_outpoints = Box::into_raw(entries.into_boxed_slice()) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_spent_outpoints_free(
    outpoints: *mut OutPointFFI,
    count: usize,
) {
    if outpoints.is_null() || count == 0 {
        return;
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(outpoints, count));
}

// ---------------------------------------------------------------------------
// Phase 6 — Per-account transactions
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_transactions(
    manager_handle: Handle,
    wallet_id: *const u8,
    spec: *const AccountSpecFFI,
    page_offset: usize,
    page_limit: usize,
    out_txs: *mut *const AccountTransactionEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(spec);
    check_ptr!(out_txs);
    check_ptr!(out_count);
    *out_txs = std::ptr::null();
    *out_count = 0;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let target = match account_type_from_spec_ref(&*spec) {
        Ok(at) => at,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e,
            );
        }
    };
    let Some(rows): Option<Vec<AccountTransactionSnapshot>> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| {
            m.account_transactions_blocking(&wid, &target, page_offset, page_limit)
        })
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if rows.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let entries: Vec<AccountTransactionEntryFFI> = rows
        .into_iter()
        .map(|s| AccountTransactionEntryFFI {
            txid: txid_to_array(&s.txid),
            height: s.height,
            timestamp: s.timestamp,
            value_delta_duffs: s.value_delta_duffs,
            fee_duffs: s.fee_duffs,
            is_coinbase: s.is_coinbase,
        })
        .collect();
    let count = entries.len();
    let boxed = entries.into_boxed_slice();
    *out_txs = Box::into_raw(boxed) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_transactions_free(
    txs: *mut AccountTransactionEntryFFI,
    count: usize,
) {
    if !txs.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(txs, count));
    }
}

// ---------------------------------------------------------------------------
// Phase 7 — Identity manager structure
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_manager_out_of_wallet_ids(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_bytes: *mut *const u8,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_bytes);
    check_ptr!(out_count);
    *out_bytes = std::ptr::null();
    *out_count = 0;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let Some(ids) = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| {
        m.identity_manager_out_of_wallet_ids_blocking(&wid)
    }) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if ids.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let count = ids.len();
    let mut flat: Vec<u8> = Vec::with_capacity(count * 32);
    for id in &ids {
        flat.extend_from_slice(&id.to_buffer());
    }
    let boxed = flat.into_boxed_slice();
    *out_bytes = Box::into_raw(boxed) as *const u8;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_manager_out_of_wallet_ids_free(
    bytes: *mut u8,
    count: usize,
) {
    if !bytes.is_null() && count > 0 {
        let total = count * 32;
        // Match the producer's `Box::into_raw(vec.into_boxed_slice())`
        // shape — see `platform_wallet_manager_free_wallet_ids`.
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, total));
    }
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_manager_wallet_identities(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_rows: *mut *const WalletIdentityRowFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_rows);
    check_ptr!(out_count);
    *out_rows = std::ptr::null();
    *out_count = 0;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let Some(rows): Option<Vec<WalletIdentityRowSnapshot>> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| {
            m.identity_manager_wallet_identities_blocking(&wid)
        })
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if rows.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let entries: Vec<WalletIdentityRowFFI> = rows
        .into_iter()
        .map(|r| WalletIdentityRowFFI {
            registration_index: r.registration_index,
            identity_id: r.identity_id,
        })
        .collect();
    let count = entries.len();
    let boxed = entries.into_boxed_slice();
    *out_rows = Box::into_raw(boxed) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_manager_wallet_identities_free(
    rows: *mut WalletIdentityRowFFI,
    count: usize,
) {
    if !rows.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(rows, count));
    }
}

// ---------------------------------------------------------------------------
// Phase 8 — DAPI address ban list
// ---------------------------------------------------------------------------

/// Snapshot of every DAPI address' ban state, including the reason
/// each address was banned (when recorded).
///
/// On success `*out_entries` points at a heap-owned `[AddressBanInfoFFI]`
/// of length `*out_count`; the caller must release it via
/// [`platform_wallet_manager_address_ban_info_free`]. An empty list
/// returns `ok()` with `*out_entries = null`, `*out_count = 0`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_address_ban_info(
    manager_handle: Handle,
    out_entries: *mut *const AddressBanInfoFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_entries);
    check_ptr!(out_count);
    *out_entries = std::ptr::null();
    *out_count = 0;
    let Some(rows): Option<Vec<AddressBanInfoSnapshot>> = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| m.address_ban_info_blocking())
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    if rows.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let entries: Vec<AddressBanInfoFFI> = rows
        .into_iter()
        .map(|s| AddressBanInfoFFI {
            address: optional_into_raw(Some(s.uri)),
            banned: s.banned,
            ban_count: u32::try_from(s.ban_count).unwrap_or(u32::MAX),
            banned_until_ms: s.banned_until_ms.unwrap_or(0),
            reason: optional_into_raw(s.reason),
        })
        .collect();
    let count = entries.len();
    let boxed = entries.into_boxed_slice();
    *out_entries = Box::into_raw(boxed) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_address_ban_info_free(
    entries: *mut AddressBanInfoFFI,
    count: usize,
) {
    if entries.is_null() || count == 0 {
        return;
    }
    // Walk every row first to release its heap-owned `address` and
    // optional `reason` C strings before reclaiming the parent slice.
    let slice = std::slice::from_raw_parts(entries, count);
    for entry in slice {
        if !entry.address.is_null() {
            let _ = CString::from_raw(entry.address);
        }
        if !entry.reason.is_null() {
            let _ = CString::from_raw(entry.reason);
        }
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn txid_to_array(txid: &dashcore::Txid) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(txid.as_ref());
    out
}

fn optional_into_raw(s: Option<String>) -> *mut c_char {
    match s {
        Some(string) => match CString::new(string) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Project an [`AccountSpecFFI`] (without xpub) into the canonical
/// [`key_wallet::account::AccountType`] used by the snapshot
/// accessors. Mirrors `account_type_from_spec` in `persistence.rs`
/// without requiring the xpub field.
fn account_type_from_spec_ref(
    spec: &AccountSpecFFI,
) -> Result<key_wallet::account::AccountType, String> {
    use crate::wallet_restore_types::{AccountTypeTagFFI, StandardAccountTypeTagFFI};
    use key_wallet::account::{AccountType, StandardAccountType};
    // `spec.type_tag` is now a plain `u8` on the FFI surface — validate
    // before matching so an out-of-range byte from a corrupt SwiftData
    // row / forward-versioned tag doesn't trigger UB on a `repr(u8)`
    // enum match. Same shape as `persistence::account_type_from_spec`.
    let type_tag = AccountTypeTagFFI::try_from_u8(spec.type_tag).ok_or_else(|| {
        format!(
            "AccountSpecFFI carries unknown type_tag byte {} (out of declared range)",
            spec.type_tag
        )
    })?;
    Ok(match type_tag {
        AccountTypeTagFFI::Standard => {
            let standard_tag = StandardAccountTypeTagFFI::try_from_u8(spec.standard_tag)
                .ok_or_else(|| {
                    format!(
                        "AccountSpecFFI(Standard) carries unknown standard_tag byte {}",
                        spec.standard_tag
                    )
                })?;
            let standard_account_type = match standard_tag {
                StandardAccountTypeTagFFI::Bip44 => StandardAccountType::BIP44Account,
                StandardAccountTypeTagFFI::Bip32 => StandardAccountType::BIP32Account,
            };
            AccountType::Standard {
                index: spec.index,
                standard_account_type,
            }
        }
        AccountTypeTagFFI::CoinJoin => AccountType::CoinJoin { index: spec.index },
        AccountTypeTagFFI::IdentityRegistration => AccountType::IdentityRegistration,
        AccountTypeTagFFI::IdentityTopUp => AccountType::IdentityTopUp {
            registration_index: spec.registration_index,
        },
        AccountTypeTagFFI::IdentityTopUpNotBoundToIdentity => {
            AccountType::IdentityTopUpNotBoundToIdentity
        }
        AccountTypeTagFFI::IdentityInvitation => AccountType::IdentityInvitation,
        AccountTypeTagFFI::AssetLockAddressTopUp => AccountType::AssetLockAddressTopUp,
        AccountTypeTagFFI::AssetLockShieldedAddressTopUp => {
            AccountType::AssetLockShieldedAddressTopUp
        }
        AccountTypeTagFFI::ProviderVotingKeys => AccountType::ProviderVotingKeys,
        AccountTypeTagFFI::ProviderOwnerKeys => AccountType::ProviderOwnerKeys,
        AccountTypeTagFFI::ProviderOperatorKeys => AccountType::ProviderOperatorKeys,
        AccountTypeTagFFI::ProviderPlatformKeys => AccountType::ProviderPlatformKeys,
        AccountTypeTagFFI::DashpayReceivingFunds => AccountType::DashpayReceivingFunds {
            index: spec.index,
            user_identity_id: spec.user_identity_id,
            friend_identity_id: spec.friend_identity_id,
        },
        AccountTypeTagFFI::DashpayExternalAccount => AccountType::DashpayExternalAccount {
            index: spec.index,
            user_identity_id: spec.user_identity_id,
            friend_identity_id: spec.friend_identity_id,
        },
        AccountTypeTagFFI::PlatformPayment => AccountType::PlatformPayment {
            account: spec.index,
            key_class: spec.key_class,
        },
        AccountTypeTagFFI::IdentityAuthenticationEcdsa
        | AccountTypeTagFFI::IdentityAuthenticationBls => {
            return Err(format!(
                "AccountTypeTagFFI {:?} is no longer mappable to a key-wallet AccountType",
                type_tag
            ));
        }
    })
}

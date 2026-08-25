//! FFI for tracked (wallet-independent) masternodes: track / untrack /
//! rename, list, refresh, capabilities, and withdraw with a host-supplied
//! key. Thin marshalling over `platform_wallet::masternode::tracked`; the
//! records reuse [`MasternodeEntryFFI`] (`source == 1`) so hosts render
//! wallet and tracked masternodes with the same code.

use std::ffi::{c_char, CStr};

use platform_wallet::masternode::locator::parse_secret_for_role;
use platform_wallet::masternode::{
    capabilities_for_roles, LocatorSecret, MasternodeKeyRole, MasternodeRecord,
};

use crate::core_wallet_types::{masternode_entry_ffi, MasternodeEntryFFI};
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::{check_ptr, unwrap_result_or_return};

fn invalid_handle() -> PlatformWalletFFIResult {
    PlatformWalletFFIResult::err(
        PlatformWalletFFIResultCode::ErrorInvalidHandle,
        "invalid platform wallet manager handle",
    )
}

unsafe fn optional_string(ptr: *const c_char) -> Result<Option<String>, PlatformWalletFFIResult> {
    if ptr.is_null() {
        return Ok(None);
    }
    match CStr::from_ptr(ptr).to_str() {
        Ok(text) => Ok(Some(text.to_string())),
        Err(e) => Err(e.into()),
    }
}

unsafe fn write_records(
    records: Vec<MasternodeRecord>,
    network: dashcore::Network,
    out_entries: *mut *const MasternodeEntryFFI,
    out_count: *mut usize,
) {
    let entries: Vec<MasternodeEntryFFI> = records
        .iter()
        .map(|record| masternode_entry_ffi(record, network))
        .collect();
    let count = entries.len();
    if count == 0 {
        *out_entries = std::ptr::null();
        *out_count = 0;
    } else {
        *out_entries = Box::into_raw(entries.into_boxed_slice()) as *const _;
        *out_count = count;
    }
}

/// Track the masternode `pro_tx_hash` (32 wire bytes) independently of any
/// wallet. `label` is optional (null / blank = none). Seeds the record from
/// the current masternode list when available — local, no network; call
/// [`platform_wallet_manager_refresh_tracked_masternode`] afterwards for the
/// Platform / registration details. Returns the new record as a one-entry
/// array (free with `platform_wallet_manager_free_masternodes`).
///
/// Whether the row survives a restart depends on the configured persister —
/// see `PLATFORM_WALLET_PERSISTENCE_CAPABILITY_TRACKED_MASTERNODES`.
///
/// Errors: `ErrorInvalidParameter` when already tracked.
///
/// # Safety
/// `pro_tx_hash` must point at 32 readable bytes; `label` may be null;
/// `out_entry` / `out_count` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_track_masternode(
    manager_handle: Handle,
    pro_tx_hash: *const u8,
    label: *const c_char,
    out_entry: *mut *const MasternodeEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(pro_tx_hash);
    check_ptr!(out_entry);
    check_ptr!(out_count);
    *out_entry = std::ptr::null();
    *out_count = 0;

    let target: [u8; 32] = std::ptr::read(pro_tx_hash as *const [u8; 32]);
    let label = match optional_string(label) {
        Ok(label) => label,
        Err(e) => return e,
    };

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        (manager.tracked_masternodes_service(), manager.sdk().network)
    });
    let Some((service, network)) = option else {
        return invalid_handle();
    };
    let record = unwrap_result_or_return!(service.track_blocking(target, label));
    write_records(vec![record], network, out_entry, out_count);
    PlatformWalletFFIResult::ok()
}

/// Stop tracking `pro_tx_hash`. `out_removed` reports whether a row
/// existed. The host owns any keys it stored for this node (secure
/// storage) and deletes them itself.
///
/// # Safety
/// `pro_tx_hash` must point at 32 readable bytes; `out_removed` must be
/// writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_untrack_masternode(
    manager_handle: Handle,
    pro_tx_hash: *const u8,
    out_removed: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(pro_tx_hash);
    check_ptr!(out_removed);
    *out_removed = false;
    let target: [u8; 32] = std::ptr::read(pro_tx_hash as *const [u8; 32]);
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        manager.tracked_masternodes_service()
    });
    let Some(service) = option else {
        return invalid_handle();
    };
    *out_removed = unwrap_result_or_return!(service.untrack_blocking(&target));
    PlatformWalletFFIResult::ok()
}

/// Rename a tracked masternode (`label` null / blank clears it).
///
/// # Safety
/// `pro_tx_hash` must point at 32 readable bytes; `label` may be null.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_set_tracked_masternode_label(
    manager_handle: Handle,
    pro_tx_hash: *const u8,
    label: *const c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(pro_tx_hash);
    let target: [u8; 32] = std::ptr::read(pro_tx_hash as *const [u8; 32]);
    let label = match optional_string(label) {
        Ok(label) => label,
        Err(e) => return e,
    };
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        manager.tracked_masternodes_service()
    });
    let Some(service) = option else {
        return invalid_handle();
    };
    unwrap_result_or_return!(service.set_label_blocking(&target, label));
    PlatformWalletFFIResult::ok()
}

/// Every tracked masternode as a [`MasternodeEntryFFI`] (`source == 1`,
/// `label` set when named), with its status resolved against the CURRENT
/// masternode list (Active / Inactive / Retired, `Unknown` while the list
/// is unavailable). Sorted by when they were tracked. Free with
/// [`crate::wallet::platform_wallet_manager_free_masternodes`].
///
/// # Safety
/// `out_entries` / `out_count` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_list_tracked_masternodes(
    manager_handle: Handle,
    out_entries: *mut *const MasternodeEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_entries);
    check_ptr!(out_count);
    *out_entries = std::ptr::null();
    *out_count = 0;
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        (manager.tracked_masternodes_service(), manager.sdk().network)
    });
    let Some((service, network)) = option else {
        return invalid_handle();
    };
    write_records(service.list_blocking(), network, out_entries, out_count);
    PlatformWalletFFIResult::ok()
}

/// Refresh everything the wallet layer can learn about a tracked
/// masternode: its list entry (local), its Platform owner / operator
/// identities (owner + payout key hashes, claimable balance), and — once —
/// its ProRegTx via DAPI Core (registration height, collateral, original
/// keys). Blocks on the network round-trips. Partial results are kept and
/// persisted even when a step fails (the error is still returned). On
/// success returns the refreshed record as a one-entry array (free with
/// `platform_wallet_manager_free_masternodes`).
///
/// # Safety
/// `pro_tx_hash` must point at 32 readable bytes; `out_entry` / `out_count`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_refresh_tracked_masternode(
    manager_handle: Handle,
    pro_tx_hash: *const u8,
    out_entry: *mut *const MasternodeEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(pro_tx_hash);
    check_ptr!(out_entry);
    check_ptr!(out_count);
    *out_entry = std::ptr::null();
    *out_count = 0;
    let target: [u8; 32] = std::ptr::read(pro_tx_hash as *const [u8; 32]);

    // The refresh awaits Platform / DAPI, so snapshot the service handle
    // under the guard and run the future on a worker without holding it.
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        (manager.tracked_masternodes_service(), manager.sdk().network)
    });
    let Some((service, network)) = option else {
        return invalid_handle();
    };
    let record =
        unwrap_result_or_return!(block_on_worker(
            async move { service.refresh(&target).await }
        ));
    write_records(vec![record], network, out_entry, out_count);
    PlatformWalletFFIResult::ok()
}

/// What a host can do with a masternode given the key roles it holds for
/// it, as a mask over `MasternodeKeyRole` (bit = role discriminant).
/// `out_capabilities` bits: 0 withdraw, 1 vote, 2 update service,
/// 3 identifies the platform node. Pure policy — shared with Android so
/// action gating never diverges.
///
/// # Safety
/// `out_capabilities` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_masternode_capabilities(
    roles_mask: u8,
    out_capabilities: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_capabilities);
    let roles = MasternodeKeyRole::ALL
        .into_iter()
        .filter(|role| roles_mask & (1 << role.as_u8()) != 0);
    let caps = capabilities_for_roles(roles);
    let mut bits = 0u8;
    if caps.can_withdraw {
        bits |= 1;
    }
    if caps.can_vote {
        bits |= 1 << 1;
    }
    if caps.can_update_service {
        bits |= 1 << 2;
    }
    if caps.identifies_platform_node {
        bits |= 1 << 3;
    }
    *out_capabilities = bits;
    PlatformWalletFFIResult::ok()
}

/// Withdraw from a TRACKED masternode's owner identity with a
/// host-supplied key. `role` is 0 (owner key; pays the registered payout
/// address, `destination` must be null) or 4 (payout-address key;
/// `destination` optional, defaults to the payout address itself).
/// `key_text` is the private key as the user holds it — WIF
/// (network-checked) or 64-char hex. The key is used for this call only.
/// Returns the identity's new balance in credits.
///
/// Ambiguous outcomes surface as `ErrorMasternodeWithdrawalUnconfirmed`
/// with the same do-not-retry contract as the wallet-scoped withdraw.
///
/// # Safety
/// `pro_tx_hash` must point at 32 readable bytes; `key_text` must be a
/// valid NUL-terminated UTF-8 string; `destination` may be null;
/// `out_new_balance` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_tracked_masternode_withdraw(
    manager_handle: Handle,
    pro_tx_hash: *const u8,
    amount_credits: u64,
    role: u8,
    key_text: *const c_char,
    destination: *const c_char,
    out_new_balance: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(pro_tx_hash);
    check_ptr!(key_text);
    check_ptr!(out_new_balance);
    *out_new_balance = 0;

    let target: [u8; 32] = std::ptr::read(pro_tx_hash as *const [u8; 32]);
    let Some(role) = MasternodeKeyRole::from_u8(role) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("unknown masternode key role {role}"),
        );
    };
    let key_text = unwrap_result_or_return!(CStr::from_ptr(key_text).to_str()).to_string();
    let destination = match optional_string(destination) {
        Ok(destination) => destination,
        Err(e) => return e,
    };

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        (manager.tracked_masternodes_service(), manager.sdk().network)
    });
    let Some((service, network)) = option else {
        return invalid_handle();
    };

    // Decode the key host-side of the await so parse errors return typed
    // messages without touching the network.
    let secret = match parse_secret_for_role(&key_text, role, network) {
        Ok(LocatorSecret::Ecdsa { secret, .. }) => secret,
        Ok(_) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "a withdrawal key is a secp256k1 key (WIF or 64-char hex)",
            )
        }
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e.to_string(),
            )
        }
    };

    let new_balance = unwrap_result_or_return!(block_on_worker(async move {
        service
            .withdraw(&target, amount_credits, role, &secret, destination)
            .await
    }));
    *out_new_balance = new_balance;
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_handles_are_invalid_handles() {
        let hash = [0u8; 32];
        let mut entries: *const MasternodeEntryFFI = std::ptr::null();
        let mut count = 5usize;
        let mut r = unsafe {
            platform_wallet_manager_track_masternode(
                0,
                hash.as_ptr(),
                std::ptr::null(),
                &mut entries,
                &mut count,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert!(entries.is_null());
        assert_eq!(count, 0);
        unsafe { platform_wallet_ffi_result_free(&mut r) };

        let mut removed = true;
        let mut r =
            unsafe { platform_wallet_manager_untrack_masternode(0, hash.as_ptr(), &mut removed) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert!(!removed);
        unsafe { platform_wallet_ffi_result_free(&mut r) };

        let mut r = unsafe {
            platform_wallet_manager_list_tracked_masternodes(0, &mut entries, &mut count)
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
    }

    #[test]
    fn capabilities_mask_round_trips() {
        let mut out = 0xFFu8;
        let mut r = unsafe { platform_wallet_masternode_capabilities(0, &mut out) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out, 0, "no roles ⇒ no capabilities");
        unsafe { platform_wallet_ffi_result_free(&mut r) };

        // Owner (bit 0) + voting (bit 1) ⇒ withdraw + vote.
        let mut r = unsafe { platform_wallet_masternode_capabilities(0b11, &mut out) };
        assert_eq!(out, 0b11);
        unsafe { platform_wallet_ffi_result_free(&mut r) };

        // Owner payout (bit 4) alone still withdraws.
        let mut r = unsafe { platform_wallet_masternode_capabilities(1 << 4, &mut out) };
        assert_eq!(out, 1);
        unsafe { platform_wallet_ffi_result_free(&mut r) };

        // Operator (bit 2) + platform node (bit 3).
        let mut r = unsafe { platform_wallet_masternode_capabilities(0b1100, &mut out) };
        assert_eq!(out, 0b1100);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
    }

    #[test]
    fn withdraw_rejects_bad_roles_and_keys_before_handles() {
        let hash = [0u8; 32];
        let key = std::ffi::CString::new("xyz").unwrap();
        let mut balance = 7u64;
        // Unknown role.
        let mut r = unsafe {
            platform_wallet_manager_tracked_masternode_withdraw(
                0,
                hash.as_ptr(),
                1,
                42,
                key.as_ptr(),
                std::ptr::null(),
                &mut balance,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
        assert_eq!(balance, 0);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
    }
}

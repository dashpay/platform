//! FFI for the masternode locator: find a masternode from an IP, a
//! proTxHash or any of its private keys, and verify a key against a role.
//! Thin marshalling over `platform_wallet::masternode::locator`.

use std::ffi::{c_char, CStr, CString};

use platform_wallet::masternode::{
    KeyVerification, LocateOptions, MasternodeKeyRole, MasternodeLocateError,
    MasternodeLocateMatch, MasternodeLocateResult, PlatformLookup,
};

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::{check_ptr, unwrap_result_or_return};

/// One masternode the locator text names — the list's view of it plus how it
/// was matched. Returned by [`platform_wallet_manager_locate_masternode`];
/// free the array with [`platform_wallet_manager_free_masternode_matches`].
#[repr(C)]
pub struct MasternodeLocateMatchFFI {
    /// proTxHash (32 wire bytes) — same orientation as
    /// `MasternodeEntryFFI.pro_tx_hash`.
    pub pro_tx_hash: [u8; 32],
    /// `"ip:port"` of the Core P2P endpoint, or null (Tor / I2P-only entry).
    pub service_address: *mut c_char,
    /// Platform HTTP (DAPI) port, gated by `has_platform_http_port`
    /// (evonodes only).
    pub platform_http_port: u16,
    pub has_platform_http_port: bool,
    /// Operator BLS public key as serialized in the list (48 bytes).
    pub operator_public_key: [u8; 48],
    /// Voting key id (hash160).
    pub voting_key_id: [u8; 20],
    /// Tenderdash node id, gated by `has_platform_node_id` (evonodes only).
    pub platform_node_id: [u8; 20],
    pub has_platform_node_id: bool,
    /// `false` ⇒ PoSe-banned.
    pub is_valid: bool,
    pub is_evonode: bool,
    /// How it was found: 0 proTxHash, 1 service address, 2 private key.
    pub matched_by: u8,
    /// Roles the pasted key fills on this masternode, as a bit mask over
    /// `MasternodeKeyRole` (bit 0 owner, 1 voting, 2 operator, 3 platform
    /// node, 4 owner payout, 5 operator payout). 0 unless `matched_by == 2`.
    pub matched_key_roles: u8,
    /// Already one of a loaded wallet's own masternodes (`wallet_id` set) —
    /// hosts say "already in wallet" rather than offering to track it.
    pub in_wallet: bool,
    pub wallet_id: [u8; 32],
}

fn roles_mask(roles: &[MasternodeKeyRole]) -> u8 {
    roles
        .iter()
        .fold(0u8, |mask, role| mask | (1u8 << role.as_u8()))
}

fn cstring_or_null(s: String) -> *mut c_char {
    CString::new(s)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

fn match_ffi(m: &MasternodeLocateMatch) -> MasternodeLocateMatchFFI {
    let s = &m.summary;
    MasternodeLocateMatchFFI {
        pro_tx_hash: s.pro_tx_hash,
        service_address: s
            .service_address
            .map(|a| cstring_or_null(a.to_string()))
            .unwrap_or(std::ptr::null_mut()),
        platform_http_port: s.platform_http_port.unwrap_or(0),
        has_platform_http_port: s.platform_http_port.is_some(),
        operator_public_key: s.operator_public_key,
        voting_key_id: s.voting_key_id,
        platform_node_id: s.platform_node_id.unwrap_or([0u8; 20]),
        has_platform_node_id: s.platform_node_id.is_some(),
        is_valid: s.is_valid,
        is_evonode: s.is_evonode,
        matched_by: m.matched_by.as_u8(),
        matched_key_roles: roles_mask(&m.matched_keys),
        in_wallet: m.in_wallet.is_some(),
        wallet_id: m.in_wallet.unwrap_or([0u8; 32]),
    }
}

fn write_matches(
    result: MasternodeLocateResult,
    out_matches: *mut *const MasternodeLocateMatchFFI,
    out_count: *mut usize,
    out_platform_lookup: *mut u8,
    out_platform_error: *mut *mut c_char,
) {
    let entries: Vec<MasternodeLocateMatchFFI> = result.matches.iter().map(match_ffi).collect();
    let count = entries.len();
    // SAFETY: pointers were null-checked by the caller.
    unsafe {
        *out_platform_lookup = result.platform_lookup.as_u8();
        if !out_platform_error.is_null() {
            *out_platform_error = result
                .platform_error
                .map(cstring_or_null)
                .unwrap_or(std::ptr::null_mut());
        }
        if count == 0 {
            *out_matches = std::ptr::null();
            *out_count = 0;
        } else {
            *out_matches = Box::into_raw(entries.into_boxed_slice()) as *const _;
            *out_count = count;
        }
    }
}

/// Find the masternode(s) `text` names — an IP (`1.2.3.4`, `1.2.3.4:9999`,
/// a DAPI URL), a proTxHash (display hex, as explorers and dashmate print
/// it), or a private key (owner / voting / payout WIF or hex, operator BLS
/// hex, Tenderdash node key in dashmate's base64 or hex). For a key, each
/// match says which role(s) it fills (`matched_key_roles`), so the host can
/// pre-fill that key field.
///
/// `search_platform` additionally asks Platform for owner / payout roles of
/// a pasted secp256k1 key (one `getIdentityByNonUniquePublicKeyHash` per
/// key — it tells DAPI which key hash the user holds, so it is opt-in).
/// `out_platform_lookup` reports that step: 0 not needed (no secp key),
/// 1 not requested, 2 done, 3 unavailable (`out_platform_error` carries the
/// reason; free with `platform_wallet_string_free`). Local matches stand
/// either way.
///
/// Errors: `ErrorInvalidParameter` with a user-facing message when the text
/// can't be read (empty, unrecognized, a WIF for the other network, a node
/// key whose public half doesn't match); `ErrorMasternodeListUnavailable`
/// when the DML hasn't synced yet. An empty match list with `Success` means
/// "nothing on the list by that locator".
///
/// # Safety
/// `text` must be a valid NUL-terminated UTF-8 string; `out_matches`,
/// `out_count`, `out_platform_lookup` must be writable;
/// `out_platform_error` may be null. Free the matches with
/// [`platform_wallet_manager_free_masternode_matches`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_locate_masternode(
    manager_handle: Handle,
    text: *const c_char,
    search_platform: bool,
    out_matches: *mut *const MasternodeLocateMatchFFI,
    out_count: *mut usize,
    out_platform_lookup: *mut u8,
    out_platform_error: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(text);
    check_ptr!(out_matches);
    check_ptr!(out_count);
    check_ptr!(out_platform_lookup);
    *out_matches = std::ptr::null();
    *out_count = 0;
    *out_platform_lookup = PlatformLookup::NotNeeded.as_u8();
    if !out_platform_error.is_null() {
        *out_platform_error = std::ptr::null_mut();
    }

    let text = unwrap_result_or_return!(CStr::from_ptr(text).to_str()).to_string();

    // Snapshot the locator (SPV / SDK handles + the wallets' own masternodes)
    // under the handle guard, then run the lookup — which may round-trip to
    // Platform — on a worker without holding anything.
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        manager.masternode_locator_blocking()
    });
    let Some(locator) = option else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "invalid platform wallet manager handle",
        );
    };

    let outcome = block_on_worker(async move {
        locator
            .locate(&text, LocateOptions { search_platform })
            .await
    });

    match outcome {
        Ok(result) => {
            write_matches(
                result,
                out_matches,
                out_count,
                out_platform_lookup,
                out_platform_error,
            );
            PlatformWalletFFIResult::ok()
        }
        Err(MasternodeLocateError::Parse(e)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            e.to_string(),
        ),
        Err(MasternodeLocateError::ListUnavailable) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorMasternodeListUnavailable,
            "the masternode list is not available yet",
        ),
    }
}

/// Free an array returned by [`platform_wallet_manager_locate_masternode`],
/// including each entry's heap C string.
///
/// # Safety
/// `entries` / `count` must be exactly what the locate call returned; call
/// once.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_free_masternode_matches(
    entries: *mut MasternodeLocateMatchFFI,
    count: usize,
) {
    if entries.is_null() || count == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(entries, count);
    for entry in slice.iter() {
        if !entry.service_address.is_null() {
            drop(CString::from_raw(entry.service_address));
        }
    }
    drop(Box::from_raw(slice as *mut [MasternodeLocateMatchFFI]));
}

/// Check `key_text` against the `role` key of the masternode `pro_tx_hash`
/// (32 wire bytes). `role` is a `MasternodeKeyRole` discriminant (0 owner,
/// 1 voting, 2 operator, 3 platform node, 4 owner payout, 5 operator
/// payout). `out_verification`: 0 matches, 1 does not match, 2 unverifiable
/// (the reference for that role isn't known — e.g. the owner key hash of a
/// node that isn't one of this wallet's and whose registration details
/// haven't been fetched). Unverifiable is NOT a pass.
///
/// The reference comes from the DML entry (voting / operator / platform
/// node) merged with the owning wallet's record (owner / payout) when the
/// node is one of a loaded wallet's masternodes.
///
/// Errors: `ErrorInvalidParameter` when `key_text` isn't a key of the
/// role's curve (or is a WIF for the other network), or `role` is out of
/// range; `NotFound` when neither the list nor any wallet knows
/// `pro_tx_hash`.
///
/// # Safety
/// `pro_tx_hash` must point at 32 readable bytes; `key_text` must be a valid
/// NUL-terminated UTF-8 string; `out_verification` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_masternode_verify_key(
    manager_handle: Handle,
    pro_tx_hash: *const u8,
    role: u8,
    key_text: *const c_char,
    out_verification: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(pro_tx_hash);
    check_ptr!(key_text);
    check_ptr!(out_verification);
    *out_verification = KeyVerification::Unverifiable.as_u8();

    let Some(role) = MasternodeKeyRole::from_u8(role) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("unknown masternode key role {role}"),
        );
    };
    let target: [u8; 32] = std::ptr::read(pro_tx_hash as *const [u8; 32]);
    let key_text = unwrap_result_or_return!(CStr::from_ptr(key_text).to_str()).to_string();

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        (
            manager.sdk().network,
            manager.masternode_key_reference_blocking(&target),
        )
    });
    let Some((network, reference)) = option else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "invalid platform wallet manager handle",
        );
    };
    let Some(reference) = reference else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "no masternode with this proTxHash on the list or in a loaded wallet",
        );
    };

    match platform_wallet::masternode::verify_masternode_key_text(
        &reference, role, &key_text, network,
    ) {
        Ok(verification) => {
            *out_verification = verification.as_u8();
            PlatformWalletFFIResult::ok()
        }
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            e.to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_wallet::masternode::{LocatorMatchKind, MasternodeListSummary};

    fn summary() -> MasternodeListSummary {
        MasternodeListSummary {
            pro_tx_hash: [1u8; 32],
            service_address: Some("1.2.3.4:9999".parse().unwrap()),
            platform_http_port: Some(443),
            operator_public_key: [2u8; 48],
            voting_key_id: [3u8; 20],
            platform_node_id: Some([4u8; 20]),
            is_valid: true,
            is_evonode: true,
        }
    }

    #[test]
    fn match_marshals_fields_and_role_mask() {
        let m = MasternodeLocateMatch {
            summary: summary(),
            matched_by: LocatorMatchKind::Key,
            matched_keys: vec![MasternodeKeyRole::Owner, MasternodeKeyRole::Voting],
            in_wallet: Some([9u8; 32]),
        };
        let ffi = match_ffi(&m);
        assert_eq!(ffi.pro_tx_hash, [1u8; 32]);
        assert_eq!(
            unsafe { CStr::from_ptr(ffi.service_address) }
                .to_str()
                .unwrap(),
            "1.2.3.4:9999"
        );
        assert!(ffi.has_platform_http_port);
        assert_eq!(ffi.platform_http_port, 443);
        assert!(ffi.has_platform_node_id);
        assert!(ffi.is_evonode);
        assert_eq!(ffi.matched_by, 2);
        assert_eq!(ffi.matched_key_roles, 0b11);
        assert!(ffi.in_wallet);
        assert_eq!(ffi.wallet_id, [9u8; 32]);
        let entries = Box::into_raw(vec![ffi].into_boxed_slice()) as *mut MasternodeLocateMatchFFI;
        unsafe { platform_wallet_manager_free_masternode_matches(entries, 1) };
    }

    #[test]
    fn null_args_are_rejected_and_out_params_initialised() {
        let mut matches: *const MasternodeLocateMatchFFI = std::ptr::dangling();
        let mut count = 7usize;
        let mut lookup = 9u8;
        let text = CString::new("1.2.3.4").unwrap();
        let mut r = unsafe {
            platform_wallet_manager_locate_masternode(
                0,
                text.as_ptr(),
                false,
                &mut matches,
                &mut count,
                &mut lookup,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert!(matches.is_null());
        assert_eq!(count, 0);
        assert_eq!(lookup, 0);
        unsafe { platform_wallet_ffi_result_free(&mut r) };

        let mut r = unsafe {
            platform_wallet_manager_locate_masternode(
                0,
                std::ptr::null(),
                false,
                &mut matches,
                &mut count,
                &mut lookup,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
    }

    #[test]
    fn verify_rejects_an_unknown_role_before_touching_handles() {
        let hash = [0u8; 32];
        let key = CString::new("x").unwrap();
        let mut out = 0u8;
        let mut r = unsafe {
            platform_wallet_manager_masternode_verify_key(
                0,
                hash.as_ptr(),
                42,
                key.as_ptr(),
                &mut out,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
        assert_eq!(out, 2, "unverifiable until proven otherwise");
        unsafe { platform_wallet_ffi_result_free(&mut r) };
    }
}

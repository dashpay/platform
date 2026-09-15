//! FFI bindings for DashPay profile read/write.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

use platform_wallet::{DashPayProfile, PaymentAddressUpdate, ProfileUpdate};
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Payment address update: action 0 keeps, 1 sets, and 2 removes the property.
/// For action 1, `bytes` must point to `len` readable bytes for the call duration.
#[repr(C)]
pub struct PaymentAddressUpdateFFI {
    pub action: u32,
    pub bytes: *const u8,
    pub len: usize,
}

unsafe fn decode_address_update(
    input: *const PaymentAddressUpdateFFI,
) -> Result<PaymentAddressUpdate, PlatformWalletFFIResult> {
    if input.is_null() {
        return Ok(PaymentAddressUpdate::Keep);
    }
    let input = &*input;
    match input.action {
        0 => Ok(PaymentAddressUpdate::Keep),
        2 => Ok(PaymentAddressUpdate::Remove),
        1 if !input.bytes.is_null() && matches!(input.len, 21 | 43) => Ok(
            PaymentAddressUpdate::Set(std::slice::from_raw_parts(input.bytes, input.len).to_vec()),
        ),
        _ => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "Invalid payment address update".to_string(),
        )),
    }
}

/// Flat FFI view of a [`DashPayProfile`].
#[repr(C)]
pub struct DashPayProfileFFI {
    pub display_name: *mut c_char,
    pub public_message: *mut c_char,
    pub avatar_url: *mut c_char,
    pub avatar_hash_is_some: bool,
    pub avatar_hash: [u8; 32],
    pub avatar_fingerprint_is_some: bool,
    pub avatar_fingerprint: [u8; 8],
    pub core_payment_address_is_some: bool,
    pub core_payment_address: [u8; 21],
    pub platform_payment_address_is_some: bool,
    pub platform_payment_address: [u8; 21],
    pub shielded_address_is_some: bool,
    pub shielded_address: [u8; 43],
}

impl DashPayProfileFFI {
    pub fn empty() -> Self {
        Self {
            display_name: ptr::null_mut(),
            public_message: ptr::null_mut(),
            avatar_url: ptr::null_mut(),
            avatar_hash_is_some: false,
            avatar_hash: [0u8; 32],
            avatar_fingerprint_is_some: false,
            avatar_fingerprint: [0u8; 8],
            core_payment_address_is_some: false,
            core_payment_address: [0; 21],
            platform_payment_address_is_some: false,
            platform_payment_address: [0; 21],
            shielded_address_is_some: false,
            shielded_address: [0; 43],
        }
    }

    fn from_profile(profile: &DashPayProfile) -> Self {
        let display_name = option_string_to_c(profile.display_name.as_deref());
        let public_message = option_string_to_c(profile.public_message.as_deref());
        let avatar_url = option_string_to_c(profile.avatar_url.as_deref());

        let (avatar_hash_is_some, avatar_hash) = match profile.avatar_hash {
            Some(h) => (true, h),
            None => (false, [0u8; 32]),
        };
        let (avatar_fingerprint_is_some, avatar_fingerprint) = match profile.avatar_fingerprint {
            Some(f) => (true, f),
            None => (false, [0u8; 8]),
        };

        Self {
            display_name,
            public_message,
            avatar_url,
            avatar_hash_is_some,
            avatar_hash,
            avatar_fingerprint_is_some,
            avatar_fingerprint,
            core_payment_address_is_some: profile
                .core_payment_address
                .as_ref()
                .is_some_and(|a| a.len() == 21),
            core_payment_address: profile
                .core_payment_address
                .as_deref()
                .and_then(|a| a.try_into().ok())
                .unwrap_or([0; 21]),
            platform_payment_address_is_some: profile
                .platform_payment_address
                .as_ref()
                .is_some_and(|a| a.len() == 21),
            platform_payment_address: profile
                .platform_payment_address
                .as_deref()
                .and_then(|a| a.try_into().ok())
                .unwrap_or([0; 21]),
            shielded_address_is_some: profile
                .shielded_address
                .as_ref()
                .is_some_and(|a| a.len() == 43),
            shielded_address: profile
                .shielded_address
                .as_deref()
                .and_then(|a| a.try_into().ok())
                .unwrap_or([0; 43]),
        }
    }
}

fn option_string_to_c(s: Option<&str>) -> *mut c_char {
    match s {
        Some(value) => match std::ffi::CString::new(value) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

pub(crate) unsafe fn decode_opt_c_str(
    ptr: *const c_char,
) -> Result<Option<String>, PlatformWalletFFIResult> {
    if ptr.is_null() {
        return Ok(None);
    }
    let s = CStr::from_ptr(ptr).to_str()?;
    Ok(Some(s.to_string()))
}

/// Read the cached DashPay profile for a [`ManagedIdentity`] handle.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_dashpay_profile(
    identity_handle: Handle,
    out_profile: *mut DashPayProfileFFI,
    out_has_profile: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_profile);
    check_ptr!(out_has_profile);
    // Zero-init the out-params before any fallible work so an early return
    // (bad id, missing wallet) leaves a safe all-null struct rather than
    // uninitialized memory — `DashPayProfileFFI` owns C-string pointers a
    // caller might otherwise free on the error path.
    unsafe {
        *out_profile = DashPayProfileFFI::empty();
        *out_has_profile = false;
    }

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity.dashpay().profile.clone()
    });
    let profile_opt = unwrap_option_or_return!(option);
    match profile_opt {
        Some(profile) => unsafe {
            *out_profile = DashPayProfileFFI::from_profile(&profile);
            *out_has_profile = true;
        },
        None => unsafe {
            *out_profile = DashPayProfileFFI::empty();
            *out_has_profile = false;
        },
    }
    PlatformWalletFFIResult::ok()
}

/// Live in-memory DashPay sync state for a [`ManagedIdentity`] — the collection
/// counts plus the high-water sync cursors. All scalars (no heap), so no free
/// is needed. The cursors are NOT persisted (they reset to `None` on cold
/// restart), so reading the live handle is the only way to inspect them; the
/// counts let a debugger compare the in-memory state against the persisted
/// SwiftData rows.
#[repr(C)]
pub struct DashPaySyncStateFFI {
    pub established_contacts: u32,
    pub incoming_requests: u32,
    pub sent_requests: u32,
    pub ignored_senders: u32,
    /// Total cached contact-profile entries (present + negative-cache).
    pub contact_profiles: u32,
    /// Of `contact_profiles`, how many hold a present profile (the rest are
    /// confirmed-absent negative-cache entries).
    pub present_contact_profiles: u32,
    pub dashpay_payments: u32,
    pub has_dashpay_profile: bool,
    pub has_high_water_received: bool,
    pub high_water_received_ms: u64,
    pub has_high_water_sent: bool,
    pub high_water_sent_ms: u64,
}

/// Read the live [`ManagedIdentity`] DashPay sync state — see
/// [`DashPaySyncStateFFI`]. The cursors are in-memory only (not persisted).
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_dashpay_sync_state(
    identity_handle: Handle,
    out_state: *mut DashPaySyncStateFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_state);
    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        let (has_received, received) = match identity.dashpay().high_water_received_ms() {
            Some(v) => (true, v),
            None => (false, 0),
        };
        let (has_sent, sent) = match identity.dashpay().high_water_sent_ms() {
            Some(v) => (true, v),
            None => (false, 0),
        };
        DashPaySyncStateFFI {
            established_contacts: identity.dashpay().established_contacts().len() as u32,
            incoming_requests: identity.dashpay().incoming_contact_requests().len() as u32,
            sent_requests: identity.dashpay().sent_contact_requests().len() as u32,
            ignored_senders: identity.dashpay().ignored_senders().len() as u32,
            contact_profiles: identity.dashpay().contact_profiles.len() as u32,
            present_contact_profiles: identity
                .dashpay()
                .contact_profiles
                .values()
                .filter(|e| e.profile.is_some())
                .count() as u32,
            dashpay_payments: identity.dashpay().payments.len() as u32,
            has_dashpay_profile: identity.dashpay().profile.is_some(),
            has_high_water_received: has_received,
            high_water_received_ms: received,
            has_high_water_sent: has_sent,
            high_water_sent_ms: sent,
        }
    });
    let state = unwrap_option_or_return!(option);
    unsafe { *out_state = state };
    PlatformWalletFFIResult::ok()
}

/// Read the cached DashPay profile for a specific identity owned by a wallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_dashpay_profile(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_profile: *mut DashPayProfileFFI,
    out_has_profile: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_profile);
    check_ptr!(out_has_profile);
    // Zero-init the out-params before any fallible work so an early return
    // (bad id, missing wallet) leaves a safe all-null struct rather than
    // uninitialized memory — `DashPayProfileFFI` owns C-string pointers a
    // caller might otherwise free on the error path.
    unsafe {
        *out_profile = DashPayProfileFFI::empty();
        *out_has_profile = false;
    }

    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });

    // Clone only the `Option<DashPayProfile>` field, not the whole
    // `ManagedIdentity` (which carries the full Identity plus the
    // established/sent/incoming BTreeMaps and payment history). The two
    // unwraps preserve the caller contract unchanged: a missing wallet or
    // identity is a NotFound error, a present identity with no profile is a
    // successful read with `has_profile == false`.
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wm = wallet.wallet_manager().blocking_read();
        let info = wm.get_wallet_info(&wallet.wallet_id())?;
        let managed = info.identity_manager.managed_identity(&id)?;
        Some(managed.dashpay().profile.clone())
    });
    let inner = unwrap_option_or_return!(option);
    let profile = unwrap_option_or_return!(inner);
    match profile {
        Some(profile) => unsafe {
            *out_profile = DashPayProfileFFI::from_profile(&profile);
            *out_has_profile = true;
        },
        None => unsafe {
            *out_profile = DashPayProfileFFI::empty();
            *out_has_profile = false;
        },
    }
    PlatformWalletFFIResult::ok()
}

/// Read the cached profile of a **contact** (by contact identity id) under
/// the given owner identity. `out_has_profile` is false when the owner has no
/// cached entry for that contact, or the entry is confirmed-absent (the
/// contact published no profile on Platform). Populated by the background
/// contact-profile sync; covers established contacts and pending senders.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_contact_profile(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    contact_identity_id: *const u8,
    out_profile: *mut DashPayProfileFFI,
    out_has_profile: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_profile);
    check_ptr!(out_has_profile);
    // Zero-init the out-params before any fallible work so an early return
    // (bad id, missing wallet) leaves a safe all-null struct rather than
    // uninitialized memory — `DashPayProfileFFI` owns C-string pointers a
    // caller might otherwise free on the error path.
    unsafe {
        *out_profile = DashPayProfileFFI::empty();
        *out_has_profile = false;
    }

    let owner = unwrap_result_or_return!(unsafe { read_identifier(owner_identity_id) });
    let contact = unwrap_result_or_return!(unsafe { read_identifier(contact_identity_id) });

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wm = wallet.wallet_manager().blocking_read();
        let info = wm.get_wallet_info(&wallet.wallet_id())?;
        info.identity_manager
            .managed_identity(&owner)
            .and_then(|m| m.dashpay().contact_profiles.get(&contact).cloned())
    });
    let entry = unwrap_option_or_return!(option);
    match entry.and_then(|e| e.profile) {
        Some(profile) => unsafe {
            *out_profile = DashPayProfileFFI::from_profile(&profile);
            *out_has_profile = true;
        },
        None => unsafe {
            *out_profile = DashPayProfileFFI::empty();
            *out_has_profile = false;
        },
    }
    PlatformWalletFFIResult::ok()
}

/// Release strings owned by a [`DashPayProfileFFI`].
#[no_mangle]
pub unsafe extern "C" fn dashpay_profile_ffi_free(profile: *mut DashPayProfileFFI) {
    if profile.is_null() {
        return;
    }
    let profile = unsafe { &mut *profile };
    crate::platform_wallet_string_free(profile.display_name);
    crate::platform_wallet_string_free(profile.public_message);
    crate::platform_wallet_string_free(profile.avatar_url);
    profile.display_name = std::ptr::null_mut();
    profile.public_message = std::ptr::null_mut();
    profile.avatar_url = std::ptr::null_mut();
}

/// Fetch DashPay profile documents for every managed identity on the
/// wallet and refresh the local cache.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_sync_dashpay_profiles(
    wallet_handle: Handle,
    out_synced_count: *mut u32,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.dashpay().sync_profiles().await })
    });
    let result = unwrap_option_or_return!(option);
    let count = unwrap_result_or_return!(result);
    if !out_synced_count.is_null() {
        unsafe { *out_synced_count = count };
    }
    PlatformWalletFFIResult::ok()
}

/// Create or update a DashPay profile using an externally-supplied signer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_create_or_update_dashpay_profile_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    display_name: *const c_char,
    public_message: *const c_char,
    avatar_url: *const c_char,
    avatar_bytes: *const u8,
    avatar_bytes_len: usize,
    do_create: bool,
    signer_handle: *mut SignerHandle,
    out_profile: *mut DashPayProfileFFI,
) -> PlatformWalletFFIResult {
    platform_wallet_create_or_update_dashpay_profile_with_addresses_with_signer(
        wallet_handle,
        identity_id,
        display_name,
        public_message,
        avatar_url,
        avatar_bytes,
        avatar_bytes_len,
        ptr::null(),
        ptr::null(),
        ptr::null(),
        do_create,
        signer_handle,
        out_profile,
    )
}

/// Create or update a profile with explicit keep/set/remove payment address operations.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_create_or_update_dashpay_profile_with_addresses_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    display_name: *const c_char,
    public_message: *const c_char,
    avatar_url: *const c_char,
    avatar_bytes: *const u8,
    avatar_bytes_len: usize,
    core_payment_address: *const PaymentAddressUpdateFFI,
    platform_payment_address: *const PaymentAddressUpdateFFI,
    shielded_address: *const PaymentAddressUpdateFFI,
    do_create: bool,
    signer_handle: *mut SignerHandle,
    out_profile: *mut DashPayProfileFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_profile);
    // `DashPayProfileFFI` owns heap C-string pointer fields freed by
    // `dashpay_profile_ffi_free`; publish the empty sentinel before any
    // fallible work so an error path never leaves uninitialized stack bytes
    // in those pointer fields. Matches the read-side helpers in this file.
    *out_profile = DashPayProfileFFI::empty();
    check_ptr!(signer_handle);

    let id = unwrap_result_or_return!(read_identifier(identity_id));

    let display_name = unwrap_result_or_return!(decode_opt_c_str(display_name));
    let public_message = unwrap_result_or_return!(decode_opt_c_str(public_message));
    let avatar_url = unwrap_result_or_return!(decode_opt_c_str(avatar_url));

    let avatar_bytes_vec: Option<Vec<u8>> = if avatar_bytes.is_null() || avatar_bytes_len == 0 {
        None
    } else {
        Some(std::slice::from_raw_parts(avatar_bytes, avatar_bytes_len).to_vec())
    };

    let core_payment_address =
        unwrap_result_or_return!(decode_address_update(core_payment_address));
    let platform_payment_address =
        unwrap_result_or_return!(decode_address_update(platform_payment_address));
    let shielded_address = unwrap_result_or_return!(decode_address_update(shielded_address));

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, move |wallet| {
        let identity = wallet.identity().clone();
        let input = ProfileUpdate {
            display_name,
            public_message,
            avatar_url,
            avatar_bytes: avatar_bytes_vec,
            core_payment_address,
            platform_payment_address,
            shielded_address,
        };

        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            if do_create {
                identity
                    .dashpay()
                    .create_profile_with_external_signer(&id, input, signer)
                    .await
            } else {
                identity
                    .dashpay()
                    .update_profile_with_external_signer(&id, input, signer)
                    .await
            }
        })
    });
    let result = unwrap_option_or_return!(option);
    let profile = unwrap_result_or_return!(result);
    *out_profile = DashPayProfileFFI::from_profile(&profile);
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;
    use std::collections::BTreeMap;

    fn make_test_identity() -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from([7u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    #[test]
    fn null_signer_leaves_profile_safe_to_free() {
        unsafe {
            let mut out = std::mem::MaybeUninit::<DashPayProfileFFI>::uninit();
            let result = platform_wallet_create_or_update_dashpay_profile_with_signer(
                0,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                false,
                ptr::null_mut(),
                out.as_mut_ptr(),
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            let mut out = out.assume_init();
            assert!(out.display_name.is_null());
            assert!(!out.shielded_address_is_some);
            dashpay_profile_ffi_free(&mut out);
        }
    }

    #[test]
    fn payment_address_updates_preserve_distinct_operations() {
        unsafe {
            assert!(matches!(
                decode_address_update(ptr::null()).unwrap(),
                PaymentAddressUpdate::Keep
            ));
            let mut update = PaymentAddressUpdateFFI {
                action: 2,
                bytes: ptr::null(),
                len: 0,
            };
            assert!(matches!(
                decode_address_update(&update).unwrap(),
                PaymentAddressUpdate::Remove
            ));
            update.action = 1;
            assert!(decode_address_update(&update).is_err());
            let address = [7u8; 43];
            update.bytes = address.as_ptr();
            update.len = address.len();
            match decode_address_update(&update).unwrap() {
                PaymentAddressUpdate::Set(bytes) => assert_eq!(bytes, address),
                _ => panic!("expected set operation"),
            }
            update.action = 3;
            assert!(decode_address_update(&update).is_err());
        }
    }

    #[test]
    fn profile_addresses_copy_to_owned_fixed_buffers() {
        let profile = DashPayProfile {
            core_payment_address: Some(vec![1; 21]),
            platform_payment_address: Some(vec![2; 21]),
            shielded_address: Some(vec![3; 43]),
            ..Default::default()
        };
        let mut ffi = DashPayProfileFFI::from_profile(&profile);
        assert!(ffi.core_payment_address_is_some);
        assert!(ffi.platform_payment_address_is_some);
        assert!(ffi.shielded_address_is_some);
        drop(profile);
        assert_eq!(ffi.core_payment_address, [1; 21]);
        assert_eq!(ffi.platform_payment_address, [2; 21]);
        assert_eq!(ffi.shielded_address, [3; 43]);
        unsafe { dashpay_profile_ffi_free(&mut ffi) };
    }

    #[test]
    fn test_get_profile_absent_returns_false_flag() {
        unsafe {
            let managed = platform_wallet::ManagedIdentity::new(make_test_identity(), 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let mut out = DashPayProfileFFI::empty();
            let mut has_profile = true;

            let result = managed_identity_get_dashpay_profile(handle, &mut out, &mut has_profile);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(!has_profile);
            assert!(out.display_name.is_null());
            assert!(out.public_message.is_null());
            assert!(out.avatar_url.is_null());
            assert!(!out.avatar_hash_is_some);
            assert!(!out.avatar_fingerprint_is_some);

            dashpay_profile_ffi_free(&mut out);
            crate::managed_identity_destroy(handle);
        }
    }

    #[test]
    fn test_get_profile_present_copies_fields() {
        unsafe {
            let mut managed = platform_wallet::ManagedIdentity::new(make_test_identity(), 0);
            let mut hash = [0u8; 32];
            hash[0] = 0xAB;
            *managed.dashpay_profile_mut() = Some(DashPayProfile {
                display_name: Some("Alice".to_string()),
                bio: Some("Bio text".to_string()),
                avatar_url: Some("https://example.com/a.png".to_string()),
                avatar_hash: Some(hash),
                avatar_fingerprint: Some([1, 2, 3, 4, 5, 6, 7, 8]),
                public_message: Some("Hello world".to_string()),
                ..Default::default()
            });
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let mut out = DashPayProfileFFI::empty();
            let mut has_profile = false;

            let result = managed_identity_get_dashpay_profile(handle, &mut out, &mut has_profile);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(has_profile);

            let display = std::ffi::CStr::from_ptr(out.display_name).to_str().unwrap();
            let public = std::ffi::CStr::from_ptr(out.public_message)
                .to_str()
                .unwrap();
            let url = std::ffi::CStr::from_ptr(out.avatar_url).to_str().unwrap();
            assert_eq!(display, "Alice");
            assert_eq!(public, "Hello world");
            assert_eq!(url, "https://example.com/a.png");
            assert!(out.avatar_hash_is_some);
            assert_eq!(out.avatar_hash[0], 0xAB);
            assert!(out.avatar_fingerprint_is_some);
            assert_eq!(out.avatar_fingerprint, [1, 2, 3, 4, 5, 6, 7, 8]);

            dashpay_profile_ffi_free(&mut out);
            crate::managed_identity_destroy(handle);
        }
    }

    #[test]
    fn test_get_profile_invalid_handle() {
        unsafe {
            let mut out = DashPayProfileFFI::empty();
            let mut has_profile = false;

            let result =
                managed_identity_get_dashpay_profile(9_999_999, &mut out, &mut has_profile);
            assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);

            dashpay_profile_ffi_free(&mut out);
        }
    }
}

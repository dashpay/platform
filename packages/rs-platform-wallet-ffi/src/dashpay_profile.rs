//! FFI bindings for DashPay profile read/write.
//!
//! Exposes three classes of entry point:
//!
//! 1. **Local cache read** — `managed_identity_get_dashpay_profile`
//!    returns whatever [`DashPayProfile`](platform_wallet::DashPayProfile)
//!    is currently cached on a [`ManagedIdentity`] in
//!    `MANAGED_IDENTITY_STORAGE`. Sync, no network.
//!
//! 2. **Platform sync** — `platform_wallet_sync_dashpay_profiles`
//!    queries the DashPay contract for profile documents owned by each
//!    managed identity and updates the local cache. Blocks the calling
//!    thread; drives the work on an 8 MB tokio worker thread (see
//!    `runtime::block_on_worker`) because proof verification recurses
//!    deeply.
//!
//! 3. **Platform write** — `platform_wallet_create_dashpay_profile` /
//!    `platform_wallet_update_dashpay_profile` broadcast a `profile`
//!    document create / replace transition, then refresh the local
//!    cache. Also block + run on the worker.
//!
//! Optional C-string inputs (`display_name`, `public_message`,
//! `avatar_url`) accept `null` for "field not provided". Avatar bytes
//! use the standard `(ptr, len)` pair with `(null, 0)` meaning "no
//! avatar"; platform-wallet computes the SHA-256 hash + dHash
//! fingerprint from the bytes before dropping them.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

use platform_wallet::{DashPayProfile, ProfileUpdate};

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;

/// Flat FFI view of a [`DashPayProfile`].
///
/// `display_name` / `public_message` / `avatar_url` are heap-allocated
/// C strings that the caller releases with
/// [`dashpay_profile_ffi_free`]; any of them may be null when the
/// underlying `Option` is `None`.
///
/// `avatar_hash` and `avatar_fingerprint` are inline so no separate
/// allocation is needed; their `_is_some` flag tells the caller
/// whether to read them.
#[repr(C)]
pub struct DashPayProfileFFI {
    /// UTF-8 NUL-terminated display name, or `null`.
    pub display_name: *mut c_char,
    /// UTF-8 NUL-terminated public message / bio, or `null`.
    pub public_message: *mut c_char,
    /// UTF-8 NUL-terminated avatar URL, or `null`.
    pub avatar_url: *mut c_char,
    /// `true` iff `avatar_hash` carries a valid SHA-256 digest.
    pub avatar_hash_is_some: bool,
    /// SHA-256 of the avatar image bytes. Ignore unless
    /// `avatar_hash_is_some`.
    pub avatar_hash: [u8; 32],
    /// `true` iff `avatar_fingerprint` carries a valid dHash.
    pub avatar_fingerprint_is_some: bool,
    /// Perceptual dHash fingerprint (64 bits). Ignore unless
    /// `avatar_fingerprint_is_some`.
    pub avatar_fingerprint: [u8; 8],
}

impl DashPayProfileFFI {
    /// Build an all-null / default-zeroed instance — the caller uses
    /// this as the out-param initial value.
    pub fn empty() -> Self {
        Self {
            display_name: ptr::null_mut(),
            public_message: ptr::null_mut(),
            avatar_url: ptr::null_mut(),
            avatar_hash_is_some: false,
            avatar_hash: [0u8; 32],
            avatar_fingerprint_is_some: false,
            avatar_fingerprint: [0u8; 8],
        }
    }

    /// Convert a cached [`DashPayProfile`] into its FFI shape,
    /// heap-allocating the string fields. The returned struct takes
    /// ownership of those allocations; the caller must release them
    /// via [`dashpay_profile_ffi_free`].
    ///
    /// The DashPay contract folds `bio` and `publicMessage` onto a
    /// single on-chain field, so the FFI only surfaces
    /// `public_message` to avoid a spurious duplicate in Swift.
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
        }
    }
}

/// Allocate a C string from a Rust string slice, or return `null` when
/// the input is `None`. Used by [`DashPayProfileFFI::from_profile`] —
/// the caller releases ownership via [`dashpay_profile_ffi_free`].
fn option_string_to_c(s: Option<&str>) -> *mut c_char {
    match s {
        Some(value) => match std::ffi::CString::new(value) {
            Ok(c_str) => c_str.into_raw(),
            // Strings containing interior NULs get dropped silently —
            // there's nothing valid to return, and profile fields
            // should never contain NUL in practice.
            Err(_) => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

/// Decode an optional UTF-8 C string input parameter.
///
/// Returns:
/// - `Ok(None)` when the pointer is null (absent field).
/// - `Ok(Some(String))` when valid UTF-8.
/// - `Err(&'static str)` describing which field failed UTF-8
///   validation so the FFI can surface a specific error message.
unsafe fn decode_opt_c_str(
    ptr: *const c_char,
    field: &'static str,
) -> Result<Option<String>, String> {
    if ptr.is_null() {
        return Ok(None);
    }
    match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(s) => Ok(Some(s.to_string())),
        Err(_) => Err(format!("{field} is not valid UTF-8")),
    }
}

/// Read the cached DashPay profile for a [`ManagedIdentity`] handle.
///
/// `out_has_profile` reflects whether the managed identity has a
/// profile cached; `out_profile` is only populated when the flag is
/// `true`. On success the caller owns any non-null strings inside
/// `out_profile` and must release them with
/// [`dashpay_profile_ffi_free`].
///
/// No network traffic — consult `platform_wallet_sync_dashpay_profiles`
/// first if you want the freshest data.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_dashpay_profile(
    identity_handle: Handle,
    out_profile: *mut DashPayProfileFFI,
    out_has_profile: *mut bool,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_profile.is_null() || out_has_profile.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided for profile out-params",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            match &identity.dashpay_profile {
                Some(profile) => {
                    unsafe {
                        *out_profile = DashPayProfileFFI::from_profile(profile);
                        *out_has_profile = true;
                    }
                    PlatformWalletFFIResult::Success
                }
                None => {
                    unsafe {
                        *out_profile = DashPayProfileFFI::empty();
                        *out_has_profile = false;
                    }
                    PlatformWalletFFIResult::Success
                }
            }
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

/// Read the cached DashPay profile for a specific identity owned by
/// a [`PlatformWallet`](platform_wallet::PlatformWallet) handle.
///
/// Convenience for UI layers that track identities by ID and don't
/// hold a live [`ManagedIdentity`] handle. Looks the identity up via
/// the wallet's `PlatformWalletInfo` under a read lock, then copies
/// the cached profile into an FFI struct. `out_has_profile` reflects
/// whether the identity has a profile cached; `out_profile` is only
/// populated when the flag is `true`.
///
/// Returns `ErrorIdentityNotFound` when the identity is unknown to
/// this wallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_dashpay_profile(
    wallet_handle: Handle,
    identity_id: IdentifierBytes,
    out_profile: *mut DashPayProfileFFI,
    out_has_profile: *mut bool,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_profile.is_null() || out_has_profile.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided for profile out-params",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let id = match identity_id.to_identifier() {
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
            // `blocking_read` is safe here — this FFI entry point is
            // invoked from a (non-tokio) caller thread, matching the
            // same pattern used by
            // `try_match_incoming_dashpay_address`.
            let wm = wallet.wallet_manager().blocking_read();
            let info = match wm.get_wallet_info(&wallet.wallet_id()) {
                Some(i) => i,
                None => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorInvalidHandle,
                                "Wallet info not found for wallet handle",
                            );
                        }
                    }
                    return PlatformWalletFFIResult::ErrorInvalidHandle;
                }
            };

            let managed = match info.identity_manager.managed_identity(&id) {
                Some(m) => m,
                None => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorIdentityNotFound,
                                format!("Identity {id} not found in wallet"),
                            );
                        }
                    }
                    return PlatformWalletFFIResult::ErrorIdentityNotFound;
                }
            };

            match &managed.dashpay_profile {
                Some(profile) => {
                    unsafe {
                        *out_profile = DashPayProfileFFI::from_profile(profile);
                        *out_has_profile = true;
                    }
                    PlatformWalletFFIResult::Success
                }
                None => {
                    unsafe {
                        *out_profile = DashPayProfileFFI::empty();
                        *out_has_profile = false;
                    }
                    PlatformWalletFFIResult::Success
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

/// Release strings owned by a [`DashPayProfileFFI`] previously populated
/// by this FFI. Safe to call on `empty()` profiles — each string pointer
/// is checked for `null` before freeing.
#[no_mangle]
pub unsafe extern "C" fn dashpay_profile_ffi_free(profile: DashPayProfileFFI) {
    // Each field is independently heap-owned. `platform_wallet_string_free`
    // is a no-op on null.
    crate::platform_wallet_string_free(profile.display_name);
    crate::platform_wallet_string_free(profile.public_message);
    crate::platform_wallet_string_free(profile.avatar_url);
}

/// Fetch DashPay profile documents for every managed identity on the
/// wallet and refresh the local cache.
///
/// `out_synced_count` is populated with the number of identities for
/// which a profile document was found and cached. Identities with no
/// on-chain profile have their cached profile cleared (if any).
///
/// Blocks until the sync completes — the actual async work runs on an
/// 8 MB tokio worker thread via `block_on_worker`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_sync_dashpay_profiles(
    wallet_handle: Handle,
    out_synced_count: *mut u32,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            // Cheap Arc clone — same generic specialization as
            // `platform_wallet_register_identity_from_addresses`.
            let identity = wallet.identity().clone();
            let result = block_on_worker(async move { identity.sync_profiles().await });
            match result {
                Ok(count) => {
                    if !out_synced_count.is_null() {
                        unsafe { *out_synced_count = count };
                    }
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("sync_dashpay_profiles failed: {e}"),
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

/// Shared body for `create` / `update`. `do_create` picks between the
/// two operation paths in `DashPayWallet`. On success the caller owns
/// strings inside `out_profile` and must free them with
/// [`dashpay_profile_ffi_free`].
unsafe fn create_or_update_profile(
    wallet_handle: Handle,
    identity_id: IdentifierBytes,
    display_name: *const c_char,
    public_message: *const c_char,
    avatar_url: *const c_char,
    avatar_bytes: *const u8,
    avatar_bytes_len: usize,
    do_create: bool,
    out_profile: *mut DashPayProfileFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_profile.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_profile is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let id = match identity_id.to_identifier() {
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

    // Decode the three optional C strings up-front so we can bail with
    // ErrorUtf8Conversion before touching the wallet handle.
    let display_name = match unsafe { decode_opt_c_str(display_name, "display_name") } {
        Ok(v) => v,
        Err(msg) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        msg,
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };
    let public_message = match unsafe { decode_opt_c_str(public_message, "public_message") } {
        Ok(v) => v,
        Err(msg) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        msg,
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };
    let avatar_url = match unsafe { decode_opt_c_str(avatar_url, "avatar_url") } {
        Ok(v) => v,
        Err(msg) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        msg,
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };

    // `(null, 0)` is the canonical "no avatar" encoding. A non-null
    // pointer with a non-zero length copies the bytes into an owned
    // `Vec<u8>` so the FFI can safely hand it to the async worker.
    let avatar_bytes_vec: Option<Vec<u8>> = if avatar_bytes.is_null() || avatar_bytes_len == 0 {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(avatar_bytes, avatar_bytes_len) }.to_vec())
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, move |wallet| {
            let identity = wallet.identity().clone();
            let input = ProfileUpdate {
                display_name,
                public_message,
                avatar_url,
                avatar_bytes: avatar_bytes_vec,
            };

            let result = block_on_worker(async move {
                if do_create {
                    identity.create_profile(&id, input).await
                } else {
                    identity.update_profile(&id, input).await
                }
            });

            match result {
                Ok(profile) => {
                    unsafe {
                        *out_profile = DashPayProfileFFI::from_profile(&profile);
                    }
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            let tag = if do_create { "create" } else { "update" };
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("{tag}_dashpay_profile failed: {e}"),
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

/// Create a new DashPay profile document on Platform and refresh the
/// local cache with the result.
///
/// See [`create_or_update_profile`] for parameter semantics. Returns
/// `ErrorWalletOperation` when a profile already exists for the
/// identity — the caller should use
/// [`platform_wallet_update_dashpay_profile`] in that case.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_create_dashpay_profile(
    wallet_handle: Handle,
    identity_id: IdentifierBytes,
    display_name: *const c_char,
    public_message: *const c_char,
    avatar_url: *const c_char,
    avatar_bytes: *const u8,
    avatar_bytes_len: usize,
    out_profile: *mut DashPayProfileFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    unsafe {
        create_or_update_profile(
            wallet_handle,
            identity_id,
            display_name,
            public_message,
            avatar_url,
            avatar_bytes,
            avatar_bytes_len,
            /* do_create = */ true,
            out_profile,
            out_error,
        )
    }
}

/// Update an existing DashPay profile document. Returns
/// `ErrorWalletOperation` when no profile is on Platform yet — the
/// caller should use [`platform_wallet_create_dashpay_profile`] in
/// that case.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_update_dashpay_profile(
    wallet_handle: Handle,
    identity_id: IdentifierBytes,
    display_name: *const c_char,
    public_message: *const c_char,
    avatar_url: *const c_char,
    avatar_bytes: *const u8,
    avatar_bytes_len: usize,
    out_profile: *mut DashPayProfileFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    unsafe {
        create_or_update_profile(
            wallet_handle,
            identity_id,
            display_name,
            public_message,
            avatar_url,
            avatar_bytes,
            avatar_bytes_len,
            /* do_create = */ false,
            out_profile,
            out_error,
        )
    }
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
    fn test_get_profile_absent_returns_false_flag() {
        unsafe {
            let managed = platform_wallet::ManagedIdentity::new(make_test_identity(), 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let mut out = DashPayProfileFFI::empty();
            let mut has_profile = true; // start true so we can observe the write
            let mut error = PlatformWalletFFIError::success();

            let result = managed_identity_get_dashpay_profile(
                handle,
                &mut out,
                &mut has_profile,
                &mut error,
            );
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert!(!has_profile);
            assert!(out.display_name.is_null());
            assert!(out.public_message.is_null());
            assert!(out.avatar_url.is_null());
            assert!(!out.avatar_hash_is_some);
            assert!(!out.avatar_fingerprint_is_some);

            dashpay_profile_ffi_free(out);
            crate::managed_identity_destroy(handle);
        }
    }

    #[test]
    fn test_get_profile_present_copies_fields() {
        unsafe {
            let mut managed = platform_wallet::ManagedIdentity::new(make_test_identity(), 0);
            let mut hash = [0u8; 32];
            hash[0] = 0xAB;
            managed.dashpay_profile = Some(DashPayProfile {
                display_name: Some("Alice".to_string()),
                bio: Some("Bio text".to_string()),
                avatar_url: Some("https://example.com/a.png".to_string()),
                avatar_hash: Some(hash),
                avatar_fingerprint: Some([1, 2, 3, 4, 5, 6, 7, 8]),
                public_message: Some("Hello world".to_string()),
            });
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let mut out = DashPayProfileFFI::empty();
            let mut has_profile = false;
            let mut error = PlatformWalletFFIError::success();

            let result = managed_identity_get_dashpay_profile(
                handle,
                &mut out,
                &mut has_profile,
                &mut error,
            );
            assert_eq!(result, PlatformWalletFFIResult::Success);
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

            dashpay_profile_ffi_free(out);
            crate::managed_identity_destroy(handle);
        }
    }

    #[test]
    fn test_get_profile_invalid_handle() {
        unsafe {
            let mut out = DashPayProfileFFI::empty();
            let mut has_profile = false;
            let mut error = PlatformWalletFFIError::success();

            let result = managed_identity_get_dashpay_profile(
                9_999_999, /* bogus handle */
                &mut out,
                &mut has_profile,
                &mut error,
            );
            assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);

            dashpay_profile_ffi_free(out);
            platform_wallet_ffi_error_free(error);
        }
    }
}

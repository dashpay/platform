use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};
use platform_wallet::wallet::persister::{NoPlatformPersistence, WalletPersister};
use platform_wallet::IdentityManager;
use std::sync::Arc;

pub(crate) fn ffi_noop_persister() -> WalletPersister {
    WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
}

/// Create a new empty IdentityManager
#[no_mangle]
pub unsafe extern "C" fn identity_manager_create(
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_handle);

    let manager = IdentityManager::default();
    let handle = IDENTITY_MANAGER_STORAGE.insert(manager);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::ok()
}

/// Add a managed identity to the manager.
///
/// Stand-alone identity-manager handles aren't bound to a wallet, so
/// the identity lands in the out-of-wallet bucket — the same place
/// observed identities go. Real wallet flows route through
/// [`crate::IdentityWallet`] APIs which thread `wallet_id` themselves.
#[no_mangle]
pub unsafe extern "C" fn identity_manager_add_identity(
    manager_handle: Handle,
    identity_handle: Handle,
) -> PlatformWalletFFIResult {
    let identity_option =
        MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| identity.clone());
    let identity = unwrap_option_or_return!(identity_option);

    let option = IDENTITY_MANAGER_STORAGE.with_item_mut(manager_handle, |manager| {
        manager.add_out_of_wallet_identity(identity.identity, &ffi_noop_persister())
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Remove an identity from the manager.
///
/// `identity_id` is a `*const u8` pointing at a 32-byte identifier
/// buffer. Pointer-passing rather than by-value `IdentifierBytes`
/// keeps the ABI safe across `@_silgen_name` (Swift would otherwise
/// hand the callee a garbage register slot for >16-byte aggregates).
#[no_mangle]
pub unsafe extern "C" fn identity_manager_remove_identity(
    manager_handle: Handle,
    identity_id: *const u8,
) -> PlatformWalletFFIResult {
    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });

    let option = IDENTITY_MANAGER_STORAGE.with_item_mut(manager_handle, |manager| {
        manager.remove_identity(&id, &ffi_noop_persister())
    });
    let result = unwrap_option_or_return!(option);
    if result.is_err() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorIdentityNotFound,
            "Identity not found",
        );
    }
    PlatformWalletFFIResult::ok()
}

/// Get an identity by ID. `identity_id` is a `*const u8` to a
/// 32-byte buffer; see [`identity_manager_remove_identity`] for the
/// rationale on pointer-passing vs by-value.
#[no_mangle]
pub unsafe extern "C" fn identity_manager_get_identity(
    manager_handle: Handle,
    identity_id: *const u8,
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_handle);

    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });

    let option = IDENTITY_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        manager.managed_identity(&id).cloned()
    });
    let inner = unwrap_option_or_return!(option);
    let identity = unwrap_option_or_return!(inner);
    unsafe { *out_handle = MANAGED_IDENTITY_STORAGE.insert(identity) };
    PlatformWalletFFIResult::ok()
}

/// Get all identity IDs across both buckets.
#[no_mangle]
pub unsafe extern "C" fn identity_manager_get_all_identity_ids(
    manager_handle: Handle,
    out_array: *mut IdentifierArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);
    // Sentinel first: the handle lookup below is fallible, and
    // `platform_wallet_identifier_array_free` reconstructs a `Vec` from any
    // non-null pointer/count pair — see `IdentifierArray::empty`.
    unsafe { *out_array = IdentifierArray::empty() };

    let option =
        IDENTITY_MANAGER_STORAGE.with_item(manager_handle, |manager| manager.identity_ids());
    let ids = unwrap_option_or_return!(option);
    unsafe { *out_array = IdentifierArray::new(ids) };
    PlatformWalletFFIResult::ok()
}

/// Get the count of identities across both buckets.
#[no_mangle]
pub unsafe extern "C" fn identity_manager_get_identity_count(
    manager_handle: Handle,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_count);

    let option =
        IDENTITY_MANAGER_STORAGE.with_item(manager_handle, |manager| manager.identity_count());
    *out_count = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Destroy IdentityManager and free resources
#[no_mangle]
pub unsafe extern "C" fn identity_manager_destroy(
    manager_handle: Handle,
) -> PlatformWalletFFIResult {
    if IDENTITY_MANAGER_STORAGE.remove(manager_handle).is_some() {
        PlatformWalletFFIResult::ok()
    } else {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Invalid manager handle",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::prelude::Identifier;
    use platform_wallet::ManagedIdentity;
    use std::collections::BTreeMap;

    fn create_test_identity() -> Identity {
        let id = Identifier::from([1u8; 32]);
        let mut public_keys = BTreeMap::new();

        public_keys.insert(
            0,
            IdentityPublicKey::V0(
                dpp::identity::identity_public_key::v0::IdentityPublicKeyV0 {
                    id: 0,
                    key_type: KeyType::ECDSA_SECP256K1,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::MASTER,
                    read_only: false,
                    data: dpp::platform_value::BinaryData::new(vec![2u8; 33]),
                    disabled_at: None,
                    contract_bounds: None,
                },
            ),
        );

        let identity_v0 = IdentityV0 {
            id,
            public_keys,
            balance: 1000,
            revision: 1,
        };
        Identity::V0(identity_v0)
    }

    #[test]
    fn test_create_identity_manager() {
        unsafe {
            let mut handle: Handle = NULL_HANDLE;

            let result = identity_manager_create(&mut handle);

            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_ne!(handle, NULL_HANDLE);

            identity_manager_destroy(handle);
        }
    }

    #[test]
    fn test_get_identity_count() {
        unsafe {
            let mut handle: Handle = NULL_HANDLE;

            identity_manager_create(&mut handle);

            let mut count: usize = 0;
            let result = identity_manager_get_identity_count(handle, &mut count);

            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(count, 0);

            identity_manager_destroy(handle);
        }
    }

    #[test]
    fn test_add_and_lookup_out_of_wallet_identity() {
        // Standalone manager handle has no wallet — `add_identity`
        // routes into the out-of-wallet bucket. Verify lookup works
        // round-trip and the count reflects the insert.
        unsafe {
            let mut manager_handle: Handle = NULL_HANDLE;

            identity_manager_create(&mut manager_handle);

            let identity = create_test_identity();
            let id_bytes: [u8; 32] = [1u8; 32];
            let managed_identity = ManagedIdentity::new(identity, 0);
            let identity_handle = MANAGED_IDENTITY_STORAGE.insert(managed_identity);

            identity_manager_add_identity(manager_handle, identity_handle);

            let mut count: usize = 0;
            identity_manager_get_identity_count(manager_handle, &mut count);
            assert_eq!(count, 1);

            let mut got: Handle = NULL_HANDLE;
            let result = identity_manager_get_identity(manager_handle, id_bytes.as_ptr(), &mut got);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_ne!(got, NULL_HANDLE);

            identity_manager_destroy(manager_handle);
        }
    }

    #[test]
    fn test_destroy_invalid_handle() {
        unsafe {
            let result = identity_manager_destroy(9999);
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        }
    }
}

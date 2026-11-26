//! FFI binding for removing an identity from the device's live wallets —
//! [`platform_wallet_manager_remove_identity_from_device`].
//!
//! Thin marshalling over
//! [`PlatformWalletManager::remove_identity_from_device`](platform_wallet::PlatformWalletManager::remove_identity_from_device):
//! the refusal guards, the removal from every holding wallet and the
//! persister emission all live in `rs-platform-wallet`. Unlike
//! `identity_manager_remove_identity`, which works on a standalone
//! `IdentityManager` with a no-op persister, this acts on the wallets the
//! host runs.

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::unwrap_option_or_return;
use crate::PLATFORM_WALLET_MANAGER_STORAGE;
use dpp::prelude::Identifier;

/// Remove the identity `identity_id` from every wallet of the manager that
/// manages it. Call it before deleting the host's own copy of the identity:
/// on a refusal nothing was removed and the host should delete nothing.
///
/// # Parameters
/// - `manager_handle` — platform-wallet manager handle.
/// - `identity_id` — the 32-byte identity id.
/// - `out_was_managed` — set to `true` when at least one wallet managed the
///   identity and removed it, `false` when none did (an identity the host
///   loaded on its own). Left `false` on error.
///
/// # Returns
/// `Success`, or the typed wallet error of the refusal: a public contact
/// request whose outcome is unconfirmed, a contact request in flight or a
/// journaled private sender, or private payments not restored yet. An unknown
/// manager handle is `NotFound`.
///
/// # Safety
/// `identity_id` must point to 32 readable bytes and `out_was_managed` to a
/// writable `bool`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_remove_identity_from_device(
    manager_handle: Handle,
    identity_id: *const [u8; 32],
    out_was_managed: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_was_managed);
    unsafe { *out_was_managed = false };
    check_ptr!(identity_id);
    let identity_id = Identifier::from(unsafe { *identity_id });

    // A panic must come back as an error, not unwind through this C frame
    // (which aborts the host process).
    catch_panic_to_code(
        "platform_wallet_manager_remove_identity_from_device",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        "The identity may still be in a wallet; remove it again.",
        || {
            let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
                runtime().block_on(manager.remove_identity_from_device(&identity_id))
            });
            match unwrap_option_or_return!(option) {
                Ok(was_managed) => {
                    unsafe { *out_was_managed = was_managed };
                    PlatformWalletFFIResult::ok()
                }
                Err(error) => error.into(),
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::mock_manager;
    use std::ptr;

    #[test]
    fn should_report_an_unmanaged_identity_and_refuse_bad_input() {
        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(mock_manager());
        let id = [7u8; 32];
        let mut was_managed = true;

        let result = unsafe {
            platform_wallet_manager_remove_identity_from_device(handle, &id, &mut was_managed)
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!was_managed);

        let result = unsafe {
            platform_wallet_manager_remove_identity_from_device(
                handle,
                ptr::null(),
                &mut was_managed,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        PLATFORM_WALLET_MANAGER_STORAGE.remove(handle);
        let result = unsafe {
            platform_wallet_manager_remove_identity_from_device(handle, &id, &mut was_managed)
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
    }
}

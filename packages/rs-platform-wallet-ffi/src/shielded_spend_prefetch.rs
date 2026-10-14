//! FFI entry point that prepares an upcoming shielded spend.
//!
//! Every shielded spend (transfer, unshield, withdraw, identity top-up /
//! create from the pool) must pick an anchor Platform has recorded, which
//! takes a round trip that downloads and proof-verifies Platform's recorded
//! anchor set. Hosts call
//! [`platform_wallet_manager_shielded_prefetch_spend_anchors`] when a send
//! screen opens so that round trip overlaps the user filling in the form
//! instead of delaying the send. Feature-gated behind `shielded`.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;

/// Prefetch Platform's recorded shielded-anchor set for the next spend on
/// `handle`'s network, in the background.
///
/// Returns as soon as the fetch is queued; it runs on the FFI runtime. A
/// shielded send started within the next ~30 s, before any shielded sync
/// grows the commitment tree, reuses the prefetched set and skips that round
/// trip; otherwise the send fetches for itself exactly as without this call.
/// Cheap to call repeatedly (a no-op while a usable set is cached), and a
/// failed fetch is only logged — it never affects a later send.
///
/// Returns `ErrorInvalidHandle` for an unknown `handle` and
/// `ErrorWalletOperation` when shielded support isn't configured on the
/// manager.
#[no_mangle]
pub extern "C" fn platform_wallet_manager_shielded_prefetch_spend_anchors(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let lookup = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.shielded_coordinator())
    });
    let coordinator = match lookup {
        None => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                format!("invalid manager handle: {handle}"),
            );
        }
        Some(None) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "shielded support not configured — call platform_wallet_manager_configure_shielded first",
            );
        }
        Some(Some(coordinator)) => coordinator,
    };
    runtime().spawn(async move {
        if let Err(e) = coordinator.prefetch_recorded_anchors().await {
            tracing::debug!(
                error = %e,
                "shielded spend-anchor prefetch failed; the next spend fetches for itself"
            );
        }
    });
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_reject_an_unknown_manager_handle() {
        let mut result = platform_wallet_manager_shielded_prefetch_spend_anchors(NULL_HANDLE);
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        unsafe { crate::platform_wallet_ffi_result_free(&mut result) };
    }

    #[test]
    fn should_reject_a_manager_without_shielded_support() {
        use crate::event_handler::EventHandlerCallbacks;
        use crate::manager::{platform_wallet_manager_create, platform_wallet_manager_destroy};
        use crate::persistence::PersistenceCallbacks;

        unsafe extern "C" fn begin(_: *mut std::ffi::c_void, _: *const u8) -> i32 {
            0
        }
        unsafe extern "C" fn end(_: *mut std::ffi::c_void, _: *const u8, _: bool) -> i32 {
            0
        }
        let sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        let persistence = PersistenceCallbacks {
            on_changeset_begin_fn: Some(begin),
            on_changeset_end_fn: Some(end),
            ..Default::default()
        };
        let events = EventHandlerCallbacks {
            context: std::ptr::null_mut(),
            on_wallet_event_fn: None,
            on_error_fn: None,
            on_platform_address_sync_completed_fn: None,
            on_shielded_sync_completed_fn: None,
            on_shielded_sync_progress_fn: None,
            on_shielded_tree_progress_fn: None,
            release_fn: None,
        };
        let mut handle = NULL_HANDLE;
        let created = unsafe {
            platform_wallet_manager_create(
                &sdk as *const dash_sdk::Sdk as *const std::ffi::c_void,
                &persistence,
                &events,
                &mut handle,
            )
        };
        assert_eq!(created.code, PlatformWalletFFIResultCode::Success);

        let mut result = platform_wallet_manager_shielded_prefetch_spend_anchors(handle);
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        unsafe { crate::platform_wallet_ffi_result_free(&mut result) };
        assert_eq!(
            unsafe { platform_wallet_manager_destroy(handle) }.code,
            PlatformWalletFFIResultCode::Success
        );
    }
}

//! FFI helpers for address-funding fee estimates.

use dpp::state_transition::address_funding_from_asset_lock_transition::calculate_address_funding_from_asset_lock_min_required_fee;

use crate::check_ptr;
use crate::error::*;
use crate::handle::{Handle, PLATFORM_WALLET_MANAGER_STORAGE};

/// Estimate the static minimum fee reserve (in credits) for an
/// `AddressFundingFromAssetLockTransition`.
///
/// This is the same consensus admission-floor formula used by
/// `AddressFundingFromAssetLockTransition::calculate_min_required_fee`, but
/// parameterized by counts so hosts can reserve Core lock value before the
/// transition exists. It is intentionally not a state-aware real-fee quote and
/// performs no network request.
///
/// # Safety
/// `out_fee` must point to 8 writable bytes (a `u64`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_address_funding_estimate_fee(
    handle: Handle,
    input_count: usize,
    output_count: usize,
    out_fee: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_fee);

    let Some(platform_version) =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| manager.sdk().version())
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            format!("invalid manager handle: {handle}"),
        );
    };

    *out_fee = calculate_address_funding_from_asset_lock_min_required_fee(
        input_count,
        output_count,
        platform_version,
    );
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn begin_changeset(
        _context: *mut std::os::raw::c_void,
        _wallet_id: *const u8,
    ) -> i32 {
        0
    }

    unsafe extern "C" fn end_changeset(
        _context: *mut std::os::raw::c_void,
        _wallet_id: *const u8,
        _success: bool,
    ) -> i32 {
        0
    }

    fn create_mock_manager(protocol_version: u32) -> Handle {
        let version =
            dpp::version::PlatformVersion::get(protocol_version).expect("protocol version");
        let sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(version)
            .build()
            .expect("mock sdk");
        let persistence = crate::persistence::PersistenceCallbacks {
            on_changeset_begin_fn: Some(begin_changeset),
            on_changeset_end_fn: Some(end_changeset),
            ..Default::default()
        };
        let events = crate::event_handler::EventHandlerCallbacks {
            context: std::ptr::null_mut(),
            on_wallet_event_fn: None,
            on_error_fn: None,
            on_platform_address_sync_completed_fn: None,
            on_shielded_sync_completed_fn: None,
            on_shielded_sync_progress_fn: None,
            on_shielded_tree_progress_fn: None,
            release_fn: None,
        };
        let mut handle: Handle = 0;
        let result = unsafe {
            crate::manager::platform_wallet_manager_create(
                &sdk as *const dash_sdk::Sdk as *const std::os::raw::c_void,
                &persistence,
                &events,
                &mut handle,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        handle
    }

    #[test]
    fn estimate_fee_uses_address_funding_min_required_fee() {
        unsafe {
            let handle =
                create_mock_manager(dpp::version::LATEST_PLATFORM_VERSION.protocol_version);
            let mut fee: u64 = 0;

            let result = platform_wallet_address_funding_estimate_fee(handle, 0, 2, &mut fee);

            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(fee, 62_000_000);

            let destroy = crate::manager::platform_wallet_manager_destroy(handle);
            assert_eq!(destroy.code, PlatformWalletFFIResultCode::Success);
        }
    }

    #[test]
    fn estimate_fee_preserves_one_output_minimum_for_zero_outputs() {
        unsafe {
            let handle =
                create_mock_manager(dpp::version::LATEST_PLATFORM_VERSION.protocol_version);
            let mut fee: u64 = 0;

            let result = platform_wallet_address_funding_estimate_fee(handle, 0, 0, &mut fee);

            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(fee, 56_000_000);

            let destroy = crate::manager::platform_wallet_manager_destroy(handle);
            assert_eq!(destroy.code, PlatformWalletFFIResultCode::Success);
        }
    }

    #[test]
    fn estimate_fee_rejects_unknown_manager_handle() {
        unsafe {
            let mut fee: u64 = 0;
            let result = platform_wallet_address_funding_estimate_fee(0, 0, 2, &mut fee);

            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        }
    }

    #[test]
    fn estimate_fee_rejects_null_out_pointer() {
        unsafe {
            let result =
                platform_wallet_address_funding_estimate_fee(0, 0, 2, std::ptr::null_mut());

            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
    }
}

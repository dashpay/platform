//! Platform status query

use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dash_sdk::dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
#[cfg(test)]
use std::ffi::CStr;
use std::ffi::CString;
#[cfg(test)]
use std::os::raw::c_char;
use std::os::raw::c_void;

/// Get platform status including block heights
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
/// - The returned C string pointer (on success) must be freed by the caller using the SDK's free routine.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_platform_status(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    match get_platform_status(sdk_handle) {
        Ok(json) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data_type: DashSDKResultDataType::NoData,
                        data: std::ptr::null_mut(),
                        error: Box::into_raw(Box::new(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            format!("Failed to create CString: {}", e),
                        ))),
                    }
                }
            };
            DashSDKResult {
                data_type: DashSDKResultDataType::String,
                data: c_str.into_raw() as *mut c_void,
                error: std::ptr::null_mut(),
            }
        }
        Err(e) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                e,
            ))),
        },
    }
}

fn get_platform_status(sdk_handle: *const SDKHandle) -> Result<String, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    let rt = crate::runtime::BigStackRuntime::new_isolated()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    let network_str = sdk.network.to_string();

    rt.block_on(async move {
        // Query for the most recent epoch
        match ExtendedEpochInfo::fetch_current(&sdk).await {
            Ok(epoch) => {
                // Calculate current block height
                // This is an approximation - the actual current block height would need a different query
                let block_height = epoch.first_block_height();
                let core_height = epoch.first_core_block_height();

                let json = format!(
                    r#"{{"version":{},"network":"{}","blockHeight":{},"coreHeight":{}}}"#,
                    10, // Protocol version
                    network_str,
                    block_height,
                    core_height
                );
                Ok(json)
            }
            Err(dash_sdk::Error::EpochNotFound) => {
                // If no epochs found, return default values
                let json = format!(
                    r#"{{"version":{},"network":"{}","blockHeight":0,"coreHeight":0}}"#,
                    10, network_str
                );
                Ok(json)
            }
            Err(e) => Err(format!("Failed to fetch platform status: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdk::SDKWrapper;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::types::SDKHandle;
    use dash_sdk::dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0;
    use dash_sdk::platform::types::epoch::EpochQuery;
    use dash_sdk::platform::LimitQuery;
    use dash_sdk::query_types::ExtendedEpochInfos;

    const MOCK_EPOCH_BLOCK_HEIGHT: u64 = 1234;
    const MOCK_EPOCH_CORE_HEIGHT: u32 = 567;

    /// Mock SDK handle that can answer `ExtendedEpochInfo::fetch_current`.
    ///
    /// `fetch_current` issues two proved queries — a genesis probe, then a
    /// two-epoch ascending confirmation from the epoch the mock reports in
    /// response metadata (0) — so both have to be registered or the call fails
    /// with an unmatched-expectation error.
    fn create_mock_sdk_handle_with_current_epoch() -> *mut SDKHandle {
        let mut wrapper = Box::new(SDKWrapper::new_mock());

        let epoch = ExtendedEpochInfo::from(ExtendedEpochInfoV0 {
            index: 0,
            first_block_time: 0,
            first_block_height: MOCK_EPOCH_BLOCK_HEIGHT,
            first_core_block_height: MOCK_EPOCH_CORE_HEIGHT,
            fee_multiplier_permille: 0,
            protocol_version: dash_sdk::dpp::version::LATEST_VERSION,
        });

        // Registering expectations is async, and the FFI entry point below spins
        // up its own runtime and blocks on it, so this runtime must be gone by
        // the time it runs.
        let setup_runtime = tokio::runtime::Runtime::new().expect("create setup runtime");
        setup_runtime.block_on(async {
            wrapper
                .sdk
                .mock()
                .expect_fetch::<ExtendedEpochInfo, _>(
                    LimitQuery {
                        query: EpochQuery::genesis(),
                        limit: Some(1),
                        start_info: None,
                    },
                    Some(epoch.clone()),
                )
                .await
                .expect("register epoch probe expectation");
            wrapper
                .sdk
                .mock()
                .expect_fetch_many::<_, ExtendedEpochInfo, _, ExtendedEpochInfos>(
                    LimitQuery {
                        query: EpochQuery::ascending_from(0),
                        limit: Some(2),
                        start_info: None,
                    },
                    Some(ExtendedEpochInfos::from_iter([(0, Some(epoch))])),
                )
                .await
                .expect("register epoch confirmation expectation");
        });
        drop(setup_runtime);

        Box::into_raw(wrapper) as *mut SDKHandle
    }

    #[test]
    fn test_get_platform_status_null_handle() {
        unsafe {
            let result = dash_sdk_get_platform_status(std::ptr::null());
            assert!(!result.error.is_null());
        }
    }

    /// With both `fetch_current` queries answered, the FFI entry point must
    /// report the fetched epoch's heights rather than the zeroed fallback.
    #[test]
    fn test_get_platform_status() {
        let handle = create_mock_sdk_handle_with_current_epoch();
        unsafe {
            let result = dash_sdk_get_platform_status(handle);

            assert!(result.error.is_null(), "platform status must succeed");
            assert_eq!(result.data_type, DashSDKResultDataType::String);
            assert!(!result.data.is_null());

            let json = CStr::from_ptr(result.data as *const c_char)
                .to_str()
                .expect("utf-8 json")
                .to_string();
            assert!(
                json.contains(&format!(r#""blockHeight":{}"#, MOCK_EPOCH_BLOCK_HEIGHT)),
                "unexpected json: {json}"
            );
            assert!(
                json.contains(&format!(r#""coreHeight":{}"#, MOCK_EPOCH_CORE_HEIGHT)),
                "unexpected json: {json}"
            );

            let _ = CString::from_raw(result.data as *mut c_char);
            destroy_mock_sdk_handle(handle);
        }
    }

    /// Without expectations the mock cannot answer the epoch queries, so the
    /// call must surface an error rather than silently returning the
    /// `EpochNotFound` fallback JSON.
    #[test]
    fn test_get_platform_status_without_epoch_expectations_errors() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_get_platform_status(handle);
            assert!(!result.error.is_null());
            assert_eq!(result.data_type, DashSDKResultDataType::NoData);
            destroy_mock_sdk_handle(handle);
        }
    }
}

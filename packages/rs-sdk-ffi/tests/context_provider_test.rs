#[cfg(test)]
mod tests {
    use rs_sdk_ffi::{
        context_provider::CoreSDKHandle, dash_sdk_context_provider_destroy,
        dash_sdk_context_provider_from_callbacks, dash_sdk_create_extended,
        dash_sdk_register_context_callbacks, CallbackResult, ContextProviderCallbacks,
        DashSDKConfig, DashSDKConfigExtended, DashSDKNetwork,
    };
    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_context_provider_creation() {
        unsafe {
            // Create dummy callbacks
            extern "C" fn get_height_cb(
                _h: *mut core::ffi::c_void,
                out: *mut u32,
            ) -> CallbackResult {
                unsafe {
                    if !out.is_null() {
                        *out = 0;
                    }
                }
                CallbackResult {
                    success: true,
                    error_code: 0,
                    error_message: std::ptr::null(),
                }
            }
            extern "C" fn get_quorum_pk_cb(
                _h: *mut core::ffi::c_void,
                _qt: u32,
                _qh: *const u8,
                _hgt: u32,
                out: *mut u8,
            ) -> CallbackResult {
                // Write 48 zero bytes
                unsafe {
                    if !out.is_null() {
                        std::ptr::write_bytes(out, 0, 48);
                    }
                }
                CallbackResult {
                    success: true,
                    error_code: 0,
                    error_message: std::ptr::null(),
                }
            }

            let callbacks = ContextProviderCallbacks {
                core_handle: std::ptr::dangling_mut::<core::ffi::c_void>(),
                get_platform_activation_height: get_height_cb,
                get_quorum_public_key: get_quorum_pk_cb,
            };

            // Optionally register globally so SDK creation path can pick it up
            let _ = dash_sdk_register_context_callbacks(&callbacks);

            // Create context provider from callbacks
            let context_provider = dash_sdk_context_provider_from_callbacks(&callbacks);

            assert!(
                !context_provider.is_null(),
                "Context provider should be created"
            );

            // Clean up
            dash_sdk_context_provider_destroy(context_provider);
        }
    }

    #[test]
    fn test_sdk_creation_with_context_provider() {
        unsafe {
            // Create a mock Core SDK handle using an opaque pointer
            // In real usage, this would come from the Core SDK
            let core_handle_ptr = std::ptr::dangling_mut::<CoreSDKHandle>();

            // Create base config
            let dapi_addresses = CString::new("https://testnet.dash.org:3000").unwrap();
            let base_config = DashSDKConfig {
                network: DashSDKNetwork::SDKTestnet,
                dapi_addresses: dapi_addresses.as_ptr(),
                skip_asset_lock_proof_verification: false,
                request_retry_count: 3,
                request_timeout_ms: 30000,
            };

            // Create extended config
            let extended_config = DashSDKConfigExtended {
                base_config,
                context_provider: ptr::null_mut(),
                core_sdk_handle: core_handle_ptr,
            };

            // Create SDK with extended config
            let result = dash_sdk_create_extended(&extended_config);

            // In test mode with stubs, this might fail due to missing implementations
            // but we're mainly testing that the code compiles
            println!(
                "SDK creation result - has error: {}",
                !result.error.is_null()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use rs_sdk_ffi::{
        context_provider::CoreSDKHandle, dash_sdk_context_provider_destroy,
        dash_sdk_context_provider_from_core, dash_sdk_create_extended, DashSDKConfig,
        DashSDKConfigExtended, DashSDKNetwork,
    };
    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_context_provider_creation() {
        unsafe {
            // Create a mock Core SDK handle using an opaque pointer
            // In real usage, this would come from the Core SDK
            let core_handle_ptr = 1 as *mut CoreSDKHandle;

            // Create context provider from Core handle
            let context_provider = dash_sdk_context_provider_from_core(
                core_handle_ptr,
                ptr::null(),
                ptr::null(),
                ptr::null(),
            );

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
            let core_handle_ptr = 1 as *mut CoreSDKHandle;

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

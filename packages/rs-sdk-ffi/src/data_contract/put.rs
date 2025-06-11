use crate::sdk::SDKWrapper;
use crate::{
    DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType, DataContractHandle,
    FFIError, IOSSigner, SDKHandle, SignerHandle,
};
use dash_sdk::platform::{DataContract, IdentityPublicKey};

/// Put data contract to platform (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_put_to_platform(
    sdk_handle: *mut SDKHandle,
    data_contract_handle: *const DataContractHandle,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const IOSSigner);

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Put data contract to platform using the PutContract trait
        use dash_sdk::platform::transition::put_contract::PutContract;

        let state_transition = data_contract
            .put_to_platform(
                &wrapper.sdk,
                identity_public_key.clone(),
                signer,
                None, // settings (use defaults)
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to put data contract to platform: {}", e))
            })?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
            FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => DashSDKResult::success_binary(serialized_data),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Put data contract to platform and wait for confirmation (broadcast state transition and wait for response)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_put_to_platform_and_wait(
    sdk_handle: *mut SDKHandle,
    data_contract_handle: *const DataContractHandle,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const IOSSigner);

    let result: Result<DataContract, FFIError> = wrapper.runtime.block_on(async {
        // Put data contract to platform and wait for response
        use dash_sdk::platform::transition::put_contract::PutContract;

        let confirmed_contract = data_contract
            .put_to_platform_and_wait_for_response(
                &wrapper.sdk,
                identity_public_key.clone(),
                signer,
                None, // settings (use defaults)
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!(
                    "Failed to put data contract to platform and wait: {}",
                    e
                ))
            })?;

        Ok(confirmed_contract)
    });

    match result {
        Ok(confirmed_contract) => {
            let handle = Box::into_raw(Box::new(confirmed_contract)) as *mut DataContractHandle;
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultDataContractHandle,
            )
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::*;
    use crate::types::{IdentityPublicKeyHandle, SignerHandle};
    use std::ptr;

    #[test]
    fn test_dash_sdk_data_contract_put_to_platform_null_parameters() {
        unsafe {
            // Test with null SDK handle
            let data_contract = create_mock_data_contract();
            let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const IdentityPublicKeyHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;

            let result = dash_sdk_data_contract_put_to_platform(
                ptr::null_mut(),
                data_contract_handle,
                identity_public_key_handle,
                signer_handle,
            );

            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut IOSSigner);

            // Test with null data contract handle
            let sdk_handle = create_mock_sdk_handle();
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const IdentityPublicKeyHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;

            let result = dash_sdk_data_contract_put_to_platform(
                sdk_handle,
                ptr::null(),
                identity_public_key_handle,
                signer_handle,
            );

            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            destroy_mock_sdk_handle(sdk_handle);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut IOSSigner);

            // Test with null identity public key handle
            let sdk_handle = create_mock_sdk_handle();
            let data_contract = create_mock_data_contract();
            let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;

            let result = dash_sdk_data_contract_put_to_platform(
                sdk_handle,
                data_contract_handle,
                ptr::null(),
                signer_handle,
            );

            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            destroy_mock_sdk_handle(sdk_handle);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(signer_handle as *mut IOSSigner);

            // Test with null signer handle
            let sdk_handle = create_mock_sdk_handle();
            let data_contract = create_mock_data_contract();
            let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const IdentityPublicKeyHandle;

            let result = dash_sdk_data_contract_put_to_platform(
                sdk_handle,
                data_contract_handle,
                identity_public_key_handle,
                ptr::null(),
            );

            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            destroy_mock_sdk_handle(sdk_handle);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
        }
    }

    #[test]
    fn test_dash_sdk_data_contract_put_to_platform_and_wait_null_parameters() {
        unsafe {
            // Test with null SDK handle
            let data_contract = create_mock_data_contract();
            let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const IdentityPublicKeyHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;

            let result = dash_sdk_data_contract_put_to_platform_and_wait(
                ptr::null_mut(),
                data_contract_handle,
                identity_public_key_handle,
                signer_handle,
            );

            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut IOSSigner);

            // Test with null data contract handle
            let sdk_handle = create_mock_sdk_handle();
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const IdentityPublicKeyHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;

            let result = dash_sdk_data_contract_put_to_platform_and_wait(
                sdk_handle,
                ptr::null(),
                identity_public_key_handle,
                signer_handle,
            );

            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            destroy_mock_sdk_handle(sdk_handle);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut IOSSigner);

            // Test with null identity public key handle
            let sdk_handle = create_mock_sdk_handle();
            let data_contract = create_mock_data_contract();
            let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;

            let result = dash_sdk_data_contract_put_to_platform_and_wait(
                sdk_handle,
                data_contract_handle,
                ptr::null(),
                signer_handle,
            );

            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            destroy_mock_sdk_handle(sdk_handle);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(signer_handle as *mut IOSSigner);

            // Test with null signer handle
            let sdk_handle = create_mock_sdk_handle();
            let data_contract = create_mock_data_contract();
            let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const IdentityPublicKeyHandle;

            let result = dash_sdk_data_contract_put_to_platform_and_wait(
                sdk_handle,
                data_contract_handle,
                identity_public_key_handle,
                ptr::null(),
            );

            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            destroy_mock_sdk_handle(sdk_handle);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
        }
    }

    #[test]
    fn test_dash_sdk_data_contract_put_to_platform_valid_parameters() {
        unsafe {
            let sdk_handle = create_mock_sdk_handle();
            let data_contract = create_mock_data_contract();
            let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const IdentityPublicKeyHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;

            let result = dash_sdk_data_contract_put_to_platform(
                sdk_handle,
                data_contract_handle,
                identity_public_key_handle,
                signer_handle,
            );

            // Since this is a mock SDK, it will fail when trying to actually put to platform
            // But we can verify that it gets past parameter validation
            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_ne!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            destroy_mock_sdk_handle(sdk_handle);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut IOSSigner);
        }
    }

    #[test]
    fn test_dash_sdk_data_contract_put_to_platform_and_wait_valid_parameters() {
        unsafe {
            let sdk_handle = create_mock_sdk_handle();
            let data_contract = create_mock_data_contract();
            let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const IdentityPublicKeyHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;

            let result = dash_sdk_data_contract_put_to_platform_and_wait(
                sdk_handle,
                data_contract_handle,
                identity_public_key_handle,
                signer_handle,
            );

            // Since this is a mock SDK, it will fail when trying to actually put to platform
            // But we can verify that it gets past parameter validation
            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_ne!(error.code, DashSDKErrorCode::InvalidParameter);

            // Clean up
            destroy_mock_sdk_handle(sdk_handle);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut IOSSigner);
        }
    }

    #[test]
    fn test_result_types() {
        unsafe {
            // Test that put_to_platform returns binary data type on success
            let sdk_handle = create_mock_sdk_handle();
            let data_contract = create_mock_data_contract();
            let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const IdentityPublicKeyHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;

            let _result = dash_sdk_data_contract_put_to_platform(
                sdk_handle,
                data_contract_handle,
                identity_public_key_handle,
                signer_handle,
            );

            // The actual result will have an error since we're using a mock SDK
            // But we can still verify the function compiles and runs without panicking

            // Clean up
            destroy_mock_sdk_handle(sdk_handle);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut IOSSigner);
        }
    }
}

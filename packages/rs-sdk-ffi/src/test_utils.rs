#[cfg(test)]
#[allow(clippy::module_inception, dead_code)]
pub mod test_utils {
    use crate::sdk::SDKWrapper;
    use crate::signer::VTableSigner;
    use crate::types::{DashSDKPutSettings, SDKHandle};
    use dash_sdk::dpp::data_contract::DataContractFactory;
    use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dash_sdk::dpp::platform_value::platform_value;
    use dash_sdk::dpp::platform_value::BinaryData;
    use dash_sdk::dpp::prelude::{DataContract, Identifier};
    use dash_sdk::platform::transition::put_settings::PutSettings;
    use std::ffi::CString;

    // Helper function to create a mock SDK handle
    pub fn create_mock_sdk_handle() -> *mut SDKHandle {
        let wrapper = Box::new(SDKWrapper::new_mock());
        Box::into_raw(wrapper) as *mut SDKHandle
    }

    // Helper function to destroy a mock SDK handle
    pub fn destroy_mock_sdk_handle(handle: *mut SDKHandle) {
        unsafe {
            crate::sdk::dash_sdk_destroy(handle);
        }
    }

    // Helper function to create a mock identity public key
    pub fn create_mock_identity_public_key() -> Box<IdentityPublicKey> {
        create_mock_identity_public_key_with_id(1)
    }

    // Helper function to create a mock identity public key with specific ID
    pub fn create_mock_identity_public_key_with_id(id: u64) -> Box<IdentityPublicKey> {
        Box::new(IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: id as u32,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
            contract_bounds: None,
        }))
    }

    // Mock sign callback for testing
    pub unsafe extern "C" fn mock_sign_callback(
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
        _data: *const u8,
        _data_len: usize,
        result_len: *mut usize,
    ) -> *mut u8 {
        let signature = [0u8; 64];
        *result_len = signature.len();
        let ptr = libc::malloc(signature.len()) as *mut u8;
        if !ptr.is_null() {
            std::ptr::copy_nonoverlapping(signature.as_ptr(), ptr, signature.len());
        }
        ptr
    }

    // Mock can sign callback for testing
    pub unsafe extern "C" fn mock_can_sign_callback(
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
    ) -> bool {
        true
    }

    // Helper function to create a mock signer
    pub fn create_mock_signer() -> Box<VTableSigner> {
        // Create a mock signer vtable
        let vtable = Box::new(crate::signer::SignerVTable {
            sign: mock_sign_vtable_callback,
            can_sign_with: mock_can_sign_vtable_callback,
            destroy: mock_destroy_callback,
        });

        Box::new(VTableSigner {
            signer_ptr: std::ptr::null_mut(),
            vtable: Box::into_raw(vtable),
        })
    }

    // Mock sign callback for vtable
    unsafe extern "C" fn mock_sign_vtable_callback(
        _signer: *const std::os::raw::c_void,
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
        _data: *const u8,
        _data_len: usize,
        result_len: *mut usize,
    ) -> *mut u8 {
        let signature = [0u8; 64];
        *result_len = signature.len();
        let ptr = libc::malloc(signature.len()) as *mut u8;
        if !ptr.is_null() {
            std::ptr::copy_nonoverlapping(signature.as_ptr(), ptr, signature.len());
        }
        ptr
    }

    // Mock can sign callback for vtable
    unsafe extern "C" fn mock_can_sign_vtable_callback(
        _signer: *const std::os::raw::c_void,
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
    ) -> bool {
        true
    }

    // Mock destroy callback
    unsafe extern "C" fn mock_destroy_callback(_signer: *mut std::os::raw::c_void) {
        // No-op for mock
    }

    // Helper function to create a valid transition owner ID
    pub fn create_valid_transition_owner_id() -> [u8; 32] {
        [1u8; 32]
    }

    // Helper function to create a valid recipient/target identity ID
    pub fn create_valid_recipient_id() -> [u8; 32] {
        [2u8; 32]
    }

    // Helper function to create default put settings
    pub fn create_put_settings() -> DashSDKPutSettings {
        DashSDKPutSettings {
            connect_timeout_ms: 0,
            timeout_ms: 0,
            retries: 0,
            ban_failed_address: false,
            identity_nonce_stale_time_s: 0,
            user_fee_increase: 0,
            allow_signing_with_any_security_level: false,
            allow_signing_with_any_purpose: false,
            wait_timeout_ms: 0,
        }
    }

    // Helper function to convert DashSDKPutSettings to PutSettings
    pub fn convert_put_settings(settings: DashSDKPutSettings) -> PutSettings {
        use dash_sdk::dapi_client::RequestSettings;
        use std::time::Duration;

        PutSettings {
            request_settings: RequestSettings {
                timeout: Some(Duration::from_millis(settings.timeout_ms)),
                retries: Some(settings.retries as usize),
                ban_failed_address: Some(settings.ban_failed_address),
                ..Default::default()
            },
            identity_nonce_stale_time_s: Some(settings.identity_nonce_stale_time_s),
            user_fee_increase: Some(settings.user_fee_increase),
            state_transition_creation_options: None,
            wait_timeout: if settings.wait_timeout_ms > 0 {
                Some(Duration::from_millis(settings.wait_timeout_ms))
            } else {
                None
            },
        }
    }

    // Helper function to create a C string
    pub fn create_c_string(s: &str) -> *mut std::os::raw::c_char {
        CString::new(s).unwrap().into_raw()
    }

    // Helper function to cleanup a C string pointer
    pub unsafe fn cleanup_c_string(ptr: *mut std::os::raw::c_char) {
        if !ptr.is_null() {
            let _ = CString::from_raw(ptr);
        }
    }

    // Helper function to cleanup an optional C string pointer
    pub unsafe fn cleanup_optional_c_string(ptr: *const std::os::raw::c_char) {
        if !ptr.is_null() {
            let _ = CString::from_raw(ptr as *mut std::os::raw::c_char);
        }
    }

    // Helper function to create a mock data contract
    pub fn create_mock_data_contract() -> Box<DataContract> {
        let protocol_version = 1;

        let documents = platform_value!({
            "testDoc": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "position": 0
                    },
                    "age": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 150,
                        "position": 1
                    }
                },
                "required": ["name"],
                "additionalProperties": false
            }
        });

        let factory = DataContractFactory::new(protocol_version).expect("Failed to create factory");

        let owner_id = Identifier::from_bytes(&[1u8; 32]).unwrap();
        let identity_nonce = 1u64;

        let created_contract = factory
            .create_with_value_config(owner_id, identity_nonce, documents, None, None)
            .expect("Failed to create data contract");

        Box::new(created_contract.data_contract().clone())
    }
}

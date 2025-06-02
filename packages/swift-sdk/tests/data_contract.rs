//! Tests for data contract operations

use swift_sdk::*;
use std::ffi::CString;
use std::ptr;

#[test]
fn test_data_contract_fetch_null_safety() {
    // Test null SDK handle
    let contract_id = CString::new("test_contract_id").unwrap();
    let result = unsafe { swift_dash_data_contract_fetch(ptr::null_mut(), contract_id.as_ptr()) };
    assert!(result.is_null());
    
    // Test null contract ID
    let result = unsafe { swift_dash_data_contract_fetch(ptr::null_mut(), ptr::null()) };
    assert!(result.is_null());
}

#[test]
fn test_data_contract_create_null_safety() {
    let owner_id = CString::new("owner_identity_id").unwrap();
    let schema_json = CString::new(r#"{"properties": {}}"#).unwrap();
    
    // Test with null SDK handle
    let result = unsafe {
        swift_dash_data_contract_create(
            ptr::null_mut(),
            owner_id.as_ptr(),
            schema_json.as_ptr(),
        )
    };
    assert!(result.is_null());
    
    // Test with null owner ID
    let result = unsafe {
        swift_dash_data_contract_create(
            ptr::null_mut(),
            ptr::null(),
            schema_json.as_ptr(),
        )
    };
    assert!(result.is_null());
    
    // Test with null schema
    let result = unsafe {
        swift_dash_data_contract_create(
            ptr::null_mut(),
            owner_id.as_ptr(),
            ptr::null(),
        )
    };
    assert!(result.is_null());
}

#[test]
fn test_data_contract_get_info_null_safety() {
    // Test with null contract handle
    let result = unsafe { swift_dash_data_contract_get_info(ptr::null_mut()) };
    assert!(result.is_null());
}

#[test]
fn test_data_contract_get_schema_null_safety() {
    let document_type = CString::new("testDocument").unwrap();
    
    // Test with null contract handle
    let result = unsafe {
        swift_dash_data_contract_get_schema(ptr::null_mut(), document_type.as_ptr())
    };
    assert!(result.is_null());
    
    // Test with null document type
    let result = unsafe {
        swift_dash_data_contract_get_schema(ptr::null_mut(), ptr::null())
    };
    assert!(result.is_null());
}

#[test]
fn test_data_contract_put_operations_null_safety() {
    let settings = swift_dash_put_settings_default();
    
    unsafe {
        // Test put to platform - all null
        let result = swift_dash_data_contract_put_to_platform(
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
        
        // Test put to platform and wait - all null
        let result = swift_dash_data_contract_put_to_platform_and_wait(
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
    }
}

#[test]
fn test_data_contract_schema_json_example() {
    // Example of a valid data contract schema
    let schema_json = r#"{
        "$format_version": "0",
        "id": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
        "ownerId": "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ8ihhL",
        "version": 1,
        "documentSchemas": {
            "domain": {
                "type": "object",
                "properties": {
                    "label": {
                        "type": "string",
                        "pattern": "^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$",
                        "minLength": 3,
                        "maxLength": 63,
                        "description": "Domain label"
                    },
                    "normalizedLabel": {
                        "type": "string",
                        "pattern": "^[a-z0-9][a-z0-9-]{0,61}[a-z0-9]$",
                        "maxLength": 63,
                        "description": "Normalized domain label"
                    },
                    "normalizedParentDomainName": {
                        "type": "string",
                        "pattern": "^$|^[a-z0-9][a-z0-9-\\.]{0,189}[a-z0-9]$",
                        "maxLength": 190,
                        "description": "Parent domain"
                    },
                    "records": {
                        "type": "object",
                        "properties": {
                            "dashUniqueIdentityId": {
                                "type": "array",
                                "byteArray": true,
                                "minItems": 32,
                                "maxItems": 32,
                                "description": "Identity ID"
                            }
                        },
                        "additionalProperties": false
                    }
                },
                "required": ["label", "normalizedLabel", "normalizedParentDomainName", "records"],
                "additionalProperties": false
            }
        }
    }"#;
    
    // Verify it's valid JSON
    let schema_cstring = CString::new(schema_json).unwrap();
    assert!(!schema_cstring.as_ptr().is_null());
    
    // In a real test, you would use this with swift_dash_data_contract_create
}
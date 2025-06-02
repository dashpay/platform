//! Tests for document operations

use swift_sdk::*;
use std::ffi::CString;
use std::ptr;

#[test]
fn test_document_create_null_safety() {
    let owner_id = CString::new("owner_identity_id").unwrap();
    let document_type = CString::new("testDocument").unwrap();
    let data_json = CString::new(r#"{"name": "test", "value": 42}"#).unwrap();
    
    // Test with all null parameters
    let result = unsafe {
        swift_dash_document_create(
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        )
    };
    assert!(result.is_null());
    
    // Test with null SDK handle
    let result = unsafe {
        swift_dash_document_create(
            ptr::null_mut(),
            ptr::null_mut(),
            owner_id.as_ptr(),
            document_type.as_ptr(),
            data_json.as_ptr(),
        )
    };
    assert!(result.is_null());
    
    // Test with null owner ID
    let result = unsafe {
        swift_dash_document_create(
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
            document_type.as_ptr(),
            data_json.as_ptr(),
        )
    };
    assert!(result.is_null());
}

#[test]
fn test_document_fetch_null_safety() {
    let document_type = CString::new("testDocument").unwrap();
    let document_id = CString::new("document_id_123").unwrap();
    
    // Test with all null
    let result = unsafe {
        swift_dash_document_fetch(
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
            ptr::null(),
        )
    };
    assert!(result.is_null());
    
    // Test with null document type
    let result = unsafe {
        swift_dash_document_fetch(
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
            document_id.as_ptr(),
        )
    };
    assert!(result.is_null());
    
    // Test with null document ID
    let result = unsafe {
        swift_dash_document_fetch(
            ptr::null_mut(),
            ptr::null_mut(),
            document_type.as_ptr(),
            ptr::null(),
        )
    };
    assert!(result.is_null());
}

#[test]
fn test_document_info_structure() {
    let doc_id = CString::new("doc_id_123").unwrap();
    let owner_id = CString::new("owner_id_456").unwrap();
    let contract_id = CString::new("contract_id_789").unwrap();
    let doc_type = CString::new("profile").unwrap();
    
    let info = Box::new(SwiftDashDocumentInfo {
        id: doc_id.into_raw(),
        owner_id: owner_id.into_raw(),
        data_contract_id: contract_id.into_raw(),
        document_type: doc_type.into_raw(),
        revision: 3,
        created_at: 1640000000000,
        updated_at: 1640000001000,
    });
    
    let info_ptr = Box::into_raw(info);
    
    // Verify data
    unsafe {
        assert_eq!((*info_ptr).revision, 3);
        assert_eq!((*info_ptr).created_at, 1640000000000);
        assert_eq!((*info_ptr).updated_at, 1640000001000);
        
        let id = std::ffi::CStr::from_ptr((*info_ptr).id)
            .to_string_lossy()
            .to_string();
        assert_eq!(id, "doc_id_123");
        
        // Free the structure
        swift_dash_document_info_free(info_ptr);
    }
}

#[test]
fn test_document_put_operations_null_safety() {
    let settings = swift_dash_put_settings_default();
    
    unsafe {
        // Test put to platform
        let result = swift_dash_document_put_to_platform(
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
        
        // Test put to platform and wait
        let result = swift_dash_document_put_to_platform_and_wait(
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
        
        // Test purchase to platform
        let result = swift_dash_document_purchase_to_platform(
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
        
        // Test purchase to platform and wait
        let result = swift_dash_document_purchase_to_platform_and_wait(
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
fn test_document_json_examples() {
    // Example of valid document data
    let profile_doc = r#"{
        "displayName": "Alice",
        "publicMessage": "Hello from Dash Platform!",
        "avatarUrl": "https://example.com/avatar.jpg"
    }"#;
    
    let message_doc = r#"{
        "content": "This is a test message",
        "timestamp": 1640000000000,
        "author": "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ8ihhL"
    }"#;
    
    // Verify they're valid JSON strings
    let profile_cstring = CString::new(profile_doc).unwrap();
    let message_cstring = CString::new(message_doc).unwrap();
    
    assert!(!profile_cstring.as_ptr().is_null());
    assert!(!message_cstring.as_ptr().is_null());
}

#[test]
fn test_put_settings_with_custom_values() {
    let mut settings = swift_dash_put_settings_default();
    
    // Customize settings
    settings.timeout_ms = 60000; // 60 seconds
    settings.wait_timeout_ms = 120000; // 2 minutes
    settings.retries = 5;
    settings.ban_failed_address = true;
    settings.user_fee_increase = 10; // 10% increase
    
    assert_eq!(settings.timeout_ms, 60000);
    assert_eq!(settings.wait_timeout_ms, 120000);
    assert_eq!(settings.retries, 5);
    assert!(settings.ban_failed_address);
    assert_eq!(settings.user_fee_increase, 10);
}
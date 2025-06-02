//! Tests for identity operations

use swift_sdk::*;
use std::ffi::CString;
use std::ptr;

#[test]
fn test_identity_fetch_with_null_parameters() {
    // Test null SDK handle
    let identity_id = CString::new("test_id").unwrap();
    let result = unsafe { swift_dash_identity_fetch(ptr::null_mut(), identity_id.as_ptr()) };
    assert!(result.is_null());
    
    // Test null identity ID (assuming we have a valid SDK handle)
    // Note: In real tests, you'd have a proper SDK handle
    let result = unsafe { swift_dash_identity_fetch(ptr::null_mut(), ptr::null()) };
    assert!(result.is_null());
}

#[test]
fn test_identity_info_structure() {
    // Test that we can create and free identity info structures
    let test_id = CString::new("test_identity_id").unwrap();
    
    let info = Box::new(SwiftDashIdentityInfo {
        id: test_id.into_raw(),
        balance: 1000000,
        revision: 1,
        public_keys_count: 2,
    });
    
    let info_ptr = Box::into_raw(info);
    
    // Read back the values
    unsafe {
        assert_eq!((*info_ptr).balance, 1000000);
        assert_eq!((*info_ptr).revision, 1);
        assert_eq!((*info_ptr).public_keys_count, 2);
        
        // Free the structure
        swift_dash_identity_info_free(info_ptr);
    }
}

#[test]
fn test_binary_data_handling() {
    // Test binary data structure
    let test_data = vec![1u8, 2, 3, 4, 5];
    let data_len = test_data.len();
    let data_ptr = test_data.as_ptr() as *mut u8;
    std::mem::forget(test_data); // Prevent deallocation
    
    let binary_data = Box::new(SwiftDashBinaryData {
        data: data_ptr,
        len: data_len,
    });
    
    let binary_data_ptr = Box::into_raw(binary_data);
    
    // Verify data
    unsafe {
        assert_eq!((*binary_data_ptr).len, 5);
        let slice = std::slice::from_raw_parts((*binary_data_ptr).data, (*binary_data_ptr).len);
        assert_eq!(slice, &[1, 2, 3, 4, 5]);
        
        // Free the structure
        swift_dash_binary_data_free(binary_data_ptr);
    }
}

#[test]
fn test_transfer_credits_result_structure() {
    let recipient_id = CString::new("recipient_test_id").unwrap();
    let test_data = vec![0xAB; 64]; // Simulated transaction data
    let data_len = test_data.len();
    let data_ptr = test_data.as_ptr() as *mut u8;
    std::mem::forget(test_data); // Prevent deallocation
    
    let result = Box::new(SwiftDashTransferCreditsResult {
        amount: 50000,
        recipient_id: recipient_id.into_raw(),
        transaction_data: data_ptr,
        transaction_data_len: data_len,
    });
    
    let result_ptr = Box::into_raw(result);
    
    // Verify data
    unsafe {
        assert_eq!((*result_ptr).amount, 50000);
        assert_eq!((*result_ptr).transaction_data_len, 64);
        
        let recipient = std::ffi::CStr::from_ptr((*result_ptr).recipient_id)
            .to_string_lossy()
            .to_string();
        assert_eq!(recipient, "recipient_test_id");
        
        // Free the structure
        swift_dash_transfer_credits_result_free(result_ptr);
    }
}

#[test]
fn test_identity_put_operations_null_safety() {
    // Test that put operations handle null parameters safely
    let settings = swift_dash_put_settings_default();
    
    unsafe {
        // Test put with instant lock - all null
        let result = swift_dash_identity_put_to_platform_with_instant_lock(
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
        
        // Test put with instant lock and wait - all null
        let result = swift_dash_identity_put_to_platform_with_instant_lock_and_wait(
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
        
        // Test put with chain lock - all null
        let result = swift_dash_identity_put_to_platform_with_chain_lock(
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
        
        // Test put with chain lock and wait - all null
        let result = swift_dash_identity_put_to_platform_with_chain_lock_and_wait(
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
fn test_identity_transfer_credits_null_safety() {
    let recipient_id = CString::new("recipient_id").unwrap();
    let settings = swift_dash_put_settings_default();
    
    unsafe {
        // Test with null SDK handle
        let result = swift_dash_identity_transfer_credits(
            ptr::null_mut(),
            ptr::null_mut(),
            recipient_id.as_ptr(),
            1000,
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
        
        // Test with null recipient ID
        let result = swift_dash_identity_transfer_credits(
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
            1000,
            0,
            ptr::null_mut(),
            &settings,
        );
        assert!(result.is_null());
    }
}
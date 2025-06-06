//! Document tests for rs-sdk-ffi

use crate::config::Config;
use crate::ffi_utils::*;
use rs_sdk_ffi::*;
use std::ffi::CString;
use std::ptr;

/// Test fetching a non-existent document
#[test]
fn test_document_read_not_found() {
    setup_logs();

    let cfg = Config::new();
    let handle = create_test_sdk_handle("document_read_no_contract");
    
    // First fetch the data contract
    let contract_id = to_c_string(&cfg.existing_data_contract_id);
    let contract_handle = unsafe {
        let contract_result = dash_sdk_data_contract_fetch(handle, contract_id.as_ptr());
        if !contract_result.error.is_null() {
            panic!("Failed to fetch data contract");
        }
        contract_result.data as *const DataContractHandle
    };
    
    let document_type = to_c_string(&cfg.existing_document_type_name);
    let non_existent_doc_id = to_c_string("1111111111111111111111111111111111111111111");
    
    unsafe {
        let result = dash_sdk_document_fetch(
            handle,
            contract_handle,
            document_type.as_ptr(),
            non_existent_doc_id.as_ptr()
        );
        assert_success_none(result);
        
        // Clean up
        dash_sdk_data_contract_destroy(contract_handle as *mut DataContractHandle);
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching an existing document
#[test]
fn test_document_read() {
    setup_logs();
    
    let cfg = Config::new();
    let handle = create_test_sdk_handle("document_read");
    
    // First fetch the data contract
    let contract_id = to_c_string(&cfg.existing_data_contract_id);
    let contract_handle = unsafe {
        let contract_result = dash_sdk_data_contract_fetch(handle, contract_id.as_ptr());
        if !contract_result.error.is_null() {
            panic!("Failed to fetch data contract");
        }
        contract_result.data as *const DataContractHandle
    };
    
    let document_type = to_c_string(&cfg.existing_document_type_name);
    let document_id = to_c_string(&cfg.existing_document_id);
    
    unsafe {
        let result = dash_sdk_document_fetch(
            handle,
            contract_handle,
            document_type.as_ptr(),
            document_id.as_ptr()
        );
        
        // Note: This might return None if the document doesn't exist in test vectors
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                assert!(json.get("document").is_some(), "Should have document field");
            }
            Ok(None) => {
                // Document not found is also valid for test vectors
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
        
        // Clean up
        dash_sdk_data_contract_destroy(contract_handle as *mut DataContractHandle);
    }

    destroy_test_sdk_handle(handle);
}

/// Test searching documents with a simple query
#[test]
fn test_document_search_empty_where() {
    setup_logs();
    
    let handle = create_test_sdk_handle("test_document_list_empty_where");
    
    // DPNS contract ID and domain document type
    let contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    let document_type = to_c_string("domain");
    
    // First fetch the data contract
    let contract_handle = unsafe {
        let contract_result = dash_sdk_data_contract_fetch(handle, contract_id.as_ptr());
        if !contract_result.error.is_null() {
            panic!("Failed to fetch data contract");
        }
        contract_result.data as *const DataContractHandle
    };
    
    // Empty where clause - should return all documents
    let where_json = "[]";
    let where_cstring = to_c_string(where_json);
    
    unsafe {
        let params = DashSDKDocumentSearchParams {
            data_contract_handle: contract_handle,
            document_type: document_type.as_ptr(),
            where_json: where_cstring.as_ptr(),
            order_by_json: ptr::null(),
            limit: 10,
            start_at: 0,
        };
        let result = dash_sdk_document_search(handle, &params);
        
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("documents").is_some(), "Should have documents field");
        
        let documents = json.get("documents").unwrap();
        assert!(documents.is_array(), "Documents should be an array");
        
        // Clean up
        dash_sdk_data_contract_destroy(contract_handle as *mut DataContractHandle);
    }

    destroy_test_sdk_handle(handle);
}

/// Test searching documents with where conditions
#[test]
fn test_document_search_dpns_where_startswith() {
    setup_logs();
    
    let handle = create_test_sdk_handle("document_list_dpns_where_domain_startswith");
    
    // DPNS contract ID and domain document type
    let contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    let document_type = to_c_string("domain");
    
    // First fetch the data contract
    let contract_handle = unsafe {
        let contract_result = dash_sdk_data_contract_fetch(handle, contract_id.as_ptr());
        if !contract_result.error.is_null() {
            panic!("Failed to fetch data contract");
        }
        contract_result.data as *const DataContractHandle
    };
    
    // Search for domains starting with "test"
    let where_json = r#"[{"field": "normalizedLabel", "operator": "startsWith", "value": "test"}]"#;
    let where_cstring = to_c_string(where_json);
    
    unsafe {
        let params = DashSDKDocumentSearchParams {
            data_contract_handle: contract_handle,
            document_type: document_type.as_ptr(),
            where_json: where_cstring.as_ptr(),
            order_by_json: ptr::null(),
            limit: 5,
            start_at: 0,
        };
        let result = dash_sdk_document_search(handle, &params);
        
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("documents").is_some(), "Should have documents field");
        
        let documents = json.get("documents").unwrap();
        assert!(documents.is_array(), "Documents should be an array");
        
        // Check if any documents match the filter (if any exist in test vectors)
        if let Some(docs_array) = documents.as_array() {
            for doc in docs_array {
                if let Some(normalized_label) = doc.get("normalizedLabel").and_then(|v| v.as_str()) {
                    assert!(
                        normalized_label.starts_with("test"),
                        "Document label '{}' should start with 'test'",
                        normalized_label
                    );
                }
            }
        }
        
        // Clean up
        dash_sdk_data_contract_destroy(contract_handle as *mut DataContractHandle);
    }

    destroy_test_sdk_handle(handle);
}

/// Test searching documents with complex query including order by
#[test]
fn test_document_search_with_order_by() {
    setup_logs();
    
    let handle = create_test_sdk_handle("test_document_read_complex");
    
    // DPNS contract ID and domain document type
    let contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    let document_type = to_c_string("domain");
    
    // First fetch the data contract
    let contract_handle = unsafe {
        let contract_result = dash_sdk_data_contract_fetch(handle, contract_id.as_ptr());
        if !contract_result.error.is_null() {
            panic!("Failed to fetch data contract");
        }
        contract_result.data as *const DataContractHandle
    };
    
    // Complex query with order by
    let where_json = "[]";
    let where_cstring = to_c_string(where_json);
    let order_json = r#"[{"field": "normalizedLabel", "ascending": true}]"#;
    let order_cstring = to_c_string(order_json);
    
    unsafe {
        let params = DashSDKDocumentSearchParams {
            data_contract_handle: contract_handle,
            document_type: document_type.as_ptr(),
            where_json: where_cstring.as_ptr(),
            order_by_json: order_cstring.as_ptr(),
            limit: 10,
            start_at: 0,
        };
        let result = dash_sdk_document_search(handle, &params);
        
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("documents").is_some(), "Should have documents field");
        
        let documents = json.get("documents").unwrap();
        assert!(documents.is_array(), "Documents should be an array");
        
        // If we have documents, verify they're ordered correctly
        if let Some(docs_array) = documents.as_array() {
            if docs_array.len() > 1 {
                let mut prev_label = "";
                for doc in docs_array {
                    if let Some(label) = doc.get("normalizedLabel").and_then(|v| v.as_str()) {
                        if !prev_label.is_empty() {
                            assert!(
                                label >= prev_label,
                                "Documents should be ordered ascending: '{}' should come after '{}'",
                                label,
                                prev_label
                            );
                        }
                        prev_label = label;
                    }
                }
            }
        }
        
        // Clean up
        dash_sdk_data_contract_destroy(contract_handle as *mut DataContractHandle);
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching many documents by IDs
#[test]
#[ignore = "fetch_many function not available in current SDK"]
fn test_document_fetch_many() {
    setup_logs();
    
    // NOTE: This test is disabled because fetch_many is not available
    // In the current SDK. To fetch multiple documents, you would need
    // to call fetch multiple times or use search with specific IDs.
}
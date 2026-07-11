use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::drive::grovedb::{query_result_type::Path, Element};
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::KeysInPath;
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches path elements
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `path_json` - JSON array of path elements (hex-encoded byte arrays)
/// * `keys_json` - JSON array of keys (hex-encoded byte arrays)
///
/// # Returns
/// * JSON array of elements or null if not found
/// * Error message if operation fails
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - `path_json` and `keys_json` must be valid, non-null pointers to NUL-terminated C strings that remain valid for the duration of the call.
/// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_path_elements(
    sdk_handle: *const SDKHandle,
    path_json: *const c_char,
    keys_json: *const c_char,
) -> DashSDKResult {
    match get_path_elements(sdk_handle, path_json, keys_json) {
        Ok(Some(json)) => {
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
        Ok(None) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
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

fn get_path_elements(
    sdk_handle: *const SDKHandle,
    path_json: *const c_char,
    keys_json: *const c_char,
) -> Result<Option<String>, String> {
    // Check for null pointers
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }
    if path_json.is_null() {
        return Err("Path JSON is null".to_string());
    }
    if keys_json.is_null() {
        return Err("Keys JSON is null".to_string());
    }

    let rt = crate::runtime::BigStackRuntime::new_isolated()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let path_str = unsafe {
        CStr::from_ptr(path_json)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in path: {}", e))?
    };
    let keys_str = unsafe {
        CStr::from_ptr(keys_json)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in keys: {}", e))?
    };
    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        // Parse path JSON array
        let path_array: Vec<String> = serde_json::from_str(path_str)
            .map_err(|e| format!("Failed to parse path JSON: {}", e))?;

        // Accept either hex-encoded bytes or plain strings for path elements
        let path: Path = path_array
            .into_iter()
            .map(|s| match hex::decode(&s) {
                Ok(bytes) => Ok(bytes),
                Err(_) => Ok(s.into_bytes()),
            })
            .collect::<Result<Vec<Vec<u8>>, String>>()?;

        // Parse keys JSON array
        let keys_array: Vec<String> = serde_json::from_str(keys_str)
            .map_err(|e| format!("Failed to parse keys JSON: {}", e))?;

        // Accept either hex-encoded bytes or plain strings for keys
        let keys: Vec<Vec<u8>> = keys_array
            .into_iter()
            .map(|s| match hex::decode(&s) {
                Ok(bytes) => Ok(bytes),
                Err(_) => Ok(s.into_bytes()),
            })
            .collect::<Result<Vec<Vec<u8>>, String>>()?;

        let query = KeysInPath { path, keys };

        match Element::fetch_many(&sdk, query).await {
            Ok(elements) => {
                if elements.is_empty() {
                    return Ok(None);
                }

                let elements_json: Vec<String> = elements
                    .iter()
                    .filter_map(|(key, element_opt)| {
                        element_opt.as_ref().map(|element| {
                            format!(
                                r#"{{"key":"{}","element":"{}","type":"{}"}}"#,
                                hex::encode(key),
                                format_element_data(element),
                                format_element_type(element),
                            )
                        })
                    })
                    .collect();

                Ok(Some(format!("[{}]", elements_json.join(","))))
            }
            Err(e) => Err(format!("Failed to fetch path elements: {}", e)),
        }
    })
}

/// Renders the data side of an `Element` for the JSON `element` field.
///
/// `NonCounted` and `NotSummed` are transparent wrappers around another
/// element; we render them as `non_counted(<inner>)` / `not_summed(<inner>)`
/// so that the wrapped element's value is still visible to the caller.
fn format_element_data(element: &Element) -> String {
    match element {
        Element::Item(data, _) => hex::encode(data),
        Element::Reference(reference, _, _) => format!("{:?}", reference),
        Element::Tree(_, _) => "tree".to_string(),
        Element::SumTree(_, _, _) => "sum_tree".to_string(),
        Element::SumItem(value, _) => format!("sum_item:{}", value),
        Element::BigSumTree(_, value, _) => format!("big_sum_tree:{}", value),
        Element::CountTree(_, count, _) => format!("count_tree:{}", count),
        Element::CountSumTree(_, count, sum, _) => {
            format!("count_sum_tree:{}:{}", count, sum)
        }
        Element::ItemWithSumItem(data, sum, _) => {
            format!("item_with_sum_item:{}:{}", hex::encode(data), sum)
        }
        Element::ReferenceWithSumItem(reference, _, sum, _) => {
            format!("reference_with_sum_item:{:?}:{}", reference, sum)
        }
        Element::ProvableCountTree(_, count, _) => format!("provable_count_tree:{}", count),
        Element::ProvableCountSumTree(_, count, sum, _) => {
            format!("provable_count_sum_tree:{}:{}", count, sum)
        }
        Element::ProvableCountProvableSumTree(_, count, sum, _) => {
            format!("provable_count_provable_sum_tree:{}:{}", count, sum)
        }
        Element::ProvableSumTree(_, sum, _) => format!("provable_sum_tree:{}", sum),
        Element::CommitmentTree(_, _, _) => "commitment_tree".to_string(),
        Element::MmrTree(_, _) => "mmr_tree".to_string(),
        Element::BulkAppendTree(_, _, _) => "bulk_append_tree".to_string(),
        Element::DenseAppendOnlyFixedSizeTree(_, _, _) => {
            "dense_append_only_fixed_size_tree".to_string()
        }
        Element::NonCounted(inner) => format!("non_counted({})", format_element_data(inner)),
        Element::NotSummed(inner) => format!("not_summed({})", format_element_data(inner)),
        Element::NotCountedOrSummed(inner) => {
            format!("not_counted_or_summed({})", format_element_data(inner))
        }
    }
}

/// Renders the variant name of an `Element` for the JSON `type` field.
fn format_element_type(element: &Element) -> String {
    match element {
        Element::Item(_, _) => "item".to_string(),
        Element::Reference(_, _, _) => "reference".to_string(),
        Element::Tree(_, _) => "tree".to_string(),
        Element::SumTree(_, _, _) => "sum_tree".to_string(),
        Element::SumItem(_, _) => "sum_item".to_string(),
        Element::BigSumTree(_, _, _) => "big_sum_tree".to_string(),
        Element::CountTree(_, _, _) => "count_tree".to_string(),
        Element::CountSumTree(_, _, _, _) => "count_sum_tree".to_string(),
        Element::ItemWithSumItem(_, _, _) => "item_with_sum_item".to_string(),
        Element::ReferenceWithSumItem(_, _, _, _) => "reference_with_sum_item".to_string(),
        Element::ProvableCountTree(_, _, _) => "provable_count_tree".to_string(),
        Element::ProvableCountSumTree(_, _, _, _) => "provable_count_sum_tree".to_string(),
        Element::ProvableCountProvableSumTree(_, _, _, _) => {
            "provable_count_provable_sum_tree".to_string()
        }
        Element::ProvableSumTree(_, _, _) => "provable_sum_tree".to_string(),
        Element::CommitmentTree(_, _, _) => "commitment_tree".to_string(),
        Element::MmrTree(_, _) => "mmr_tree".to_string(),
        Element::BulkAppendTree(_, _, _) => "bulk_append_tree".to_string(),
        Element::DenseAppendOnlyFixedSizeTree(_, _, _) => {
            "dense_append_only_fixed_size_tree".to_string()
        }
        Element::NonCounted(inner) => format!("non_counted({})", format_element_type(inner)),
        Element::NotSummed(inner) => format!("not_summed({})", format_element_type(inner)),
        Element::NotCountedOrSummed(inner) => {
            format!("not_counted_or_summed({})", format_element_type(inner))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_path_elements_null_handle() {
        unsafe {
            let result = dash_sdk_system_get_path_elements(
                std::ptr::null(),
                CString::new(r#"["00"]"#).unwrap().as_ptr(),
                CString::new(r#"["01"]"#).unwrap().as_ptr(),
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_path_elements_null_path() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_system_get_path_elements(
                handle,
                std::ptr::null(),
                CString::new(r#"["01"]"#).unwrap().as_ptr(),
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}

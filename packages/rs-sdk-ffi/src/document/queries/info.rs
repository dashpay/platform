//! Document information and lifecycle operations

use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::platform_value::Value;
use std::ffi::CString;

use crate::types::{
    DashSDKDocumentField, DashSDKDocumentFieldType, DashSDKDocumentInfo, DocumentHandle,
};

/// Get document information
///
/// # Safety
/// - `document_handle` must be a valid, non-null pointer to a `DocumentHandle` that remains valid for the duration of the call.
/// - Returns a heap-allocated `DashSDKDocumentInfo` pointer on success; caller must free it using the SDK-provided free function.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_get_info(
    document_handle: *const DocumentHandle,
) -> *mut DashSDKDocumentInfo {
    if document_handle.is_null() {
        return std::ptr::null_mut();
    }

    let document = &*(document_handle as *const Document);

    let id_str = match CString::new(document.id().to_string(Encoding::Base58)) {
        Ok(s) => s.into_raw(),
        Err(_) => return std::ptr::null_mut(),
    };

    let owner_id_str = match CString::new(document.owner_id().to_string(Encoding::Base58)) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            crate::types::dash_sdk_string_free(id_str);
            return std::ptr::null_mut();
        }
    };

    // Document doesn't have data_contract_id, use placeholder
    let data_contract_id_str = match CString::new("unknown") {
        Ok(s) => s.into_raw(),
        Err(_) => {
            crate::types::dash_sdk_string_free(id_str);
            crate::types::dash_sdk_string_free(owner_id_str);
            return std::ptr::null_mut();
        }
    };

    // Document doesn't have document_type_name, use placeholder
    let document_type_str = match CString::new("unknown") {
        Ok(s) => s.into_raw(),
        Err(_) => {
            crate::types::dash_sdk_string_free(id_str);
            crate::types::dash_sdk_string_free(owner_id_str);
            crate::types::dash_sdk_string_free(data_contract_id_str);
            return std::ptr::null_mut();
        }
    };

    // Extract document properties (data fields)
    let properties = document.properties();
    let mut data_fields = Vec::new();

    for (key, value) in properties.iter() {
        let field_name = match CString::new(key.clone()) {
            Ok(s) => s.into_raw(),
            Err(_) => continue,
        };

        let (field_type, value_str, int_value, float_value, bool_value) = match value {
            Value::Text(s) => {
                let val_str = match CString::new(s.clone()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldString,
                    val_str,
                    0i64,
                    0.0f64,
                    false,
                )
            }
            Value::I128(n) => {
                let val_str = match CString::new(n.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldInteger,
                    val_str,
                    *n as i64,
                    0.0f64,
                    false,
                )
            }
            Value::I64(n) => {
                let val_str = match CString::new(n.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldInteger,
                    val_str,
                    *n,
                    0.0f64,
                    false,
                )
            }
            Value::I32(n) => {
                let val_str = match CString::new(n.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldInteger,
                    val_str,
                    *n as i64,
                    0.0f64,
                    false,
                )
            }
            Value::I16(n) => {
                let val_str = match CString::new(n.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldInteger,
                    val_str,
                    *n as i64,
                    0.0f64,
                    false,
                )
            }
            Value::U128(n) => {
                let val_str = match CString::new(n.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldInteger,
                    val_str,
                    *n as i64,
                    0.0f64,
                    false,
                )
            }
            Value::U64(n) => {
                let val_str = match CString::new(n.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldInteger,
                    val_str,
                    *n as i64,
                    0.0f64,
                    false,
                )
            }
            Value::U32(n) => {
                let val_str = match CString::new(n.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldInteger,
                    val_str,
                    *n as i64,
                    0.0f64,
                    false,
                )
            }
            Value::U16(n) => {
                let val_str = match CString::new(n.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldInteger,
                    val_str,
                    *n as i64,
                    0.0f64,
                    false,
                )
            }
            Value::U8(n) => {
                let val_str = match CString::new(n.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldInteger,
                    val_str,
                    *n as i64,
                    0.0f64,
                    false,
                )
            }
            Value::Float(f) => {
                let val_str = match CString::new(f.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldFloat,
                    val_str,
                    0i64,
                    *f,
                    false,
                )
            }
            Value::Bool(b) => {
                let val_str = match CString::new(b.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldBoolean,
                    val_str,
                    0i64,
                    0.0f64,
                    *b,
                )
            }
            Value::Null => {
                let val_str = match CString::new("null") {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldNull,
                    val_str,
                    0i64,
                    0.0f64,
                    false,
                )
            }
            Value::Bytes(bytes) => {
                let hex_str = hex::encode(bytes.as_slice());
                let val_str = match CString::new(hex_str) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldBytes,
                    val_str,
                    0i64,
                    0.0f64,
                    false,
                )
            }
            Value::Array(arr) => {
                // Convert array to JSON string
                let json_str = serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string());
                let val_str = match CString::new(json_str) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldArray,
                    val_str,
                    0i64,
                    0.0f64,
                    false,
                )
            }
            Value::Map(map) => {
                // Convert map to JSON string
                let json_str = serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string());
                let val_str = match CString::new(json_str) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldObject,
                    val_str,
                    0i64,
                    0.0f64,
                    false,
                )
            }
            _ => {
                // For other types, convert to string
                let val_str = match CString::new(format!("{:?}", value)) {
                    Ok(s) => s.into_raw(),
                    Err(_) => {
                        crate::types::dash_sdk_string_free(field_name);
                        continue;
                    }
                };
                (
                    DashSDKDocumentFieldType::FieldString,
                    val_str,
                    0i64,
                    0.0f64,
                    false,
                )
            }
        };

        data_fields.push(DashSDKDocumentField {
            name: field_name,
            field_type,
            value: value_str,
            int_value,
            float_value,
            bool_value,
        });
    }

    // Keep the allocation length and the exported count under the same
    // authority. Some Platform values (notably text containing an embedded
    // NUL) cannot be represented by this C-string ABI and are skipped above;
    // using the original property count would let consumers walk and free
    // past the filtered allocation.
    let data_fields_count = data_fields.len();

    // Convert vector to raw pointer
    let data_fields_ptr = if data_fields_count == 0 {
        std::ptr::null_mut()
    } else {
        let mut fields = data_fields.into_boxed_slice();
        let ptr = fields.as_mut_ptr();
        std::mem::forget(fields);
        ptr
    };

    let info = DashSDKDocumentInfo {
        id: id_str,
        owner_id: owner_id_str,
        data_contract_id: data_contract_id_str,
        document_type: document_type_str,
        revision: document.revision().unwrap_or(0),
        created_at: document.created_at().map(|t| t as i64).unwrap_or(0),
        updated_at: document.updated_at().map(|t| t as i64).unwrap_or(0),
        data_fields_count,
        data_fields: data_fields_ptr,
    };

    Box::into_raw(Box::new(info))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::dash_sdk_document_info_free;
    use dash_sdk::dpp::document::DocumentV0;
    use dash_sdk::dpp::platform_value::Identifier;
    use std::collections::BTreeMap;
    use std::ffi::CStr;

    fn document_with_properties(properties: BTreeMap<String, Value>) -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::new([1; 32]),
            owner_id: Identifier::new([2; 32]),
            properties,
            ..Default::default()
        })
    }

    unsafe fn document_info(document: &Document) -> *mut DashSDKDocumentInfo {
        dash_sdk_document_get_info(document as *const Document as *const DocumentHandle)
    }

    #[test]
    fn embedded_nul_value_does_not_inflate_exported_field_count() {
        let document = document_with_properties(BTreeMap::from([
            ("kept".to_string(), Value::Text("ordinary".to_string())),
            (
                "skipped".to_string(),
                Value::Text("left\0right".to_string()),
            ),
        ]));

        unsafe {
            let info = document_info(&document);
            assert!(!info.is_null());
            assert_eq!((*info).data_fields_count, 1);
            assert!(!(*info).data_fields.is_null());

            let field = &*(*info).data_fields;
            assert_eq!(CStr::from_ptr(field.name).to_bytes(), b"kept");
            assert_eq!(CStr::from_ptr(field.value).to_bytes(), b"ordinary");

            // Exercises the production destructor with the exported count.
            dash_sdk_document_info_free(info);
        }
    }

    #[test]
    fn all_unrepresentable_fields_export_a_null_empty_slice() {
        let document = document_with_properties(BTreeMap::from([(
            "skipped".to_string(),
            Value::Text("left\0right".to_string()),
        )]));

        unsafe {
            let info = document_info(&document);
            assert!(!info.is_null());
            assert_eq!((*info).data_fields_count, 0);
            assert!((*info).data_fields.is_null());
            dash_sdk_document_info_free(info);
        }
    }

    #[test]
    fn multiple_filtered_fields_preserve_exact_allocation_length() {
        let document = document_with_properties(BTreeMap::from([
            ("a-skipped".to_string(), Value::Text("a\0b".to_string())),
            ("m-kept".to_string(), Value::I64(42)),
            ("z-skipped".to_string(), Value::Text("c\0d".to_string())),
        ]));

        unsafe {
            let info = document_info(&document);
            assert!(!info.is_null());
            assert_eq!((*info).data_fields_count, 1);
            assert!(!(*info).data_fields.is_null());
            assert_eq!(
                CStr::from_ptr((*(*info).data_fields).name).to_bytes(),
                b"m-kept"
            );
            dash_sdk_document_info_free(info);
        }
    }
}

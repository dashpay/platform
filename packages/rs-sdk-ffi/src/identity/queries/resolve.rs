//! Name resolution operations

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Arc;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::util::strings::convert_to_homograph_safe_chars;
use dash_sdk::drive::query::{WhereClause, WhereOperator};
use dash_sdk::platform::{DocumentQuery, Fetch};

/// Resolve a name to an identity
///
/// This function takes a name in the format "label.parentdomain" (e.g., "alice.dash")
/// or just "label" for top-level domains, and returns the associated identity ID.
///
/// # Arguments
/// * `sdk_handle` - Handle to the SDK instance
/// * `name` - C string containing the name to resolve
///
/// # Returns
/// * On success: A result containing the resolved identity ID
/// * On error: An error result
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_resolve_name(
    sdk_handle: *const SDKHandle,
    name: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if name.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Name is null".to_string(),
        ));
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid UTF-8 in name".to_string(),
            ));
        }
    };

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;

    // Parse the name into label and parent domain
    let (label, parent_domain) = if let Some(dot_pos) = name_str.rfind('.') {
        let label = &name_str[..dot_pos];
        let parent = &name_str[dot_pos + 1..];
        (label, parent)
    } else {
        // Top-level domain
        (name_str, "dash")
    };

    // Normalize the label and parent domain according to DPNS rules
    let normalized_label = convert_to_homograph_safe_chars(label);
    let normalized_parent_domain = convert_to_homograph_safe_chars(parent_domain);

    // Get DPNS contract ID
    let dpns_contract_id = dash_sdk::dpp::data_contracts::dpns_contract::ID;

    // Execute the async operation
    let result = sdk_wrapper.runtime.block_on(async {
        // Fetch the DPNS data contract
        let data_contract =
            match dash_sdk::platform::DataContract::fetch(sdk, dpns_contract_id).await {
                Ok(Some(contract)) => Arc::new(contract),
                Ok(None) => {
                    return Err(DashSDKError::new(
                        DashSDKErrorCode::NotFound,
                        "DPNS data contract not found".to_string(),
                    ));
                }
                Err(e) => {
                    return Err(DashSDKError::new(
                        DashSDKErrorCode::NetworkError,
                        format!("Failed to fetch DPNS contract: {}", e),
                    ));
                }
            };

        // Create a query for the domain document
        let mut query = match DocumentQuery::new(data_contract, "domain") {
            Ok(q) => q,
            Err(e) => {
                return Err(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create document query: {}", e),
                ));
            }
        };

        // Add where clauses for normalized label and parent domain
        query = query
            .with_where(WhereClause {
                field: "normalizedLabel".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text(normalized_label),
            })
            .with_where(WhereClause {
                field: "normalizedParentDomainName".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text(normalized_parent_domain),
            });

        // Fetch the document
        let document = match Document::fetch(sdk, query).await {
            Ok(Some(doc)) => doc,
            Ok(None) => {
                return Err(DashSDKError::new(
                    DashSDKErrorCode::NotFound,
                    format!("Name '{}' not found", name_str),
                ));
            }
            Err(e) => {
                return Err(DashSDKError::new(
                    DashSDKErrorCode::NetworkError,
                    format!("Failed to fetch domain document: {}", e),
                ));
            }
        };

        // Extract the identity ID from the document
        // Try to get dashUniqueIdentityId first, then dashAliasIdentityId
        let records = match document.get("records") {
            Some(Value::Map(map)) => map,
            _ => {
                return Err(DashSDKError::new(
                    DashSDKErrorCode::InvalidState,
                    "Domain document has no records field".to_string(),
                ));
            }
        };

        // Check for dashUniqueIdentityId first
        if let Some(value) = records
            .iter()
            .find(|(k, _)| k.as_str() == Some("dashUniqueIdentityId"))
        {
            if let Value::Identifier(id) = &value.1 {
                return Ok(id.to_vec());
            }
        }

        // Check for dashAliasIdentityId
        if let Some(value) = records
            .iter()
            .find(|(k, _)| k.as_str() == Some("dashAliasIdentityId"))
        {
            if let Value::Identifier(id) = &value.1 {
                return Ok(id.to_vec());
            }
        }

        Err(DashSDKError::new(
            DashSDKErrorCode::NotFound,
            "No identity ID found in domain records".to_string(),
        ))
    });

    match result {
        Ok(identity_id) => DashSDKResult::success_binary(identity_id),
        Err(e) => DashSDKResult::error(e),
    }
}

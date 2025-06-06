//! Data contract history query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::DataContractHistory;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint};

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Query for data contract history
#[derive(Debug)]
struct DataContractHistoryQuery {
    contract_id: Identifier,
    limit: Option<u32>,
    offset: Option<u32>,
    start_at_ms: u64,
    prove: bool,
}

impl dash_sdk::platform::Query<dapi_grpc::platform::v0::GetDataContractHistoryRequest>
    for DataContractHistoryQuery
{
    fn query(
        self,
        prove: bool,
    ) -> Result<dapi_grpc::platform::v0::GetDataContractHistoryRequest, dash_sdk::Error> {
        use dapi_grpc::platform::v0::get_data_contract_history_request::{
            GetDataContractHistoryRequestV0, Version,
        };

        Ok(dapi_grpc::platform::v0::GetDataContractHistoryRequest {
            version: Some(Version::V0(GetDataContractHistoryRequestV0 {
                id: self.contract_id.to_vec(),
                limit: self.limit,
                offset: self.offset,
                start_at_ms: self.start_at_ms,
                prove: self.prove || prove,
            })),
        })
    }
}

/// Fetch data contract history
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `contract_id`: Base58-encoded contract ID
/// - `limit`: Maximum number of history entries to return (0 for default)
/// - `offset`: Number of entries to skip (for pagination)
/// - `start_at_ms`: Start timestamp in milliseconds (0 for beginning)
///
/// # Returns
/// JSON string containing the data contract history
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_fetch_history(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    limit: c_uint,
    offset: c_uint,
    start_at_ms: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() || contract_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or contract ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid contract ID: {}", e),
            ))
        }
    };

    let result: Result<DataContractHistory, FFIError> = wrapper.runtime.block_on(async {
        // Create the query
        let query = DataContractHistoryQuery {
            contract_id: id,
            limit: if limit == 0 { None } else { Some(limit) },
            offset: if offset == 0 { None } else { Some(offset) },
            start_at_ms,
            prove: true,
        };

        // Fetch data contract history
        DataContractHistory::fetch(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)?
            .ok_or_else(|| FFIError::InternalError("Data contract history not found".to_string()))
    });

    match result {
        Ok(history) => {
            // Convert history to JSON
            let mut json_parts = Vec::new();

            // Add entries
            json_parts.push("\"entries\":[".to_string());
            let entries: Vec<String> = history
                .entries
                .iter()
                .map(|entry| {
                    format!(
                        "{{\"date\":{},\"contract\":{}}}",
                        entry.date,
                        serde_json::to_string(&entry.contract)
                            .unwrap_or_else(|_| "null".to_string())
                    )
                })
                .collect();
            json_parts.push(entries.join(","));
            json_parts.push("]".to_string());

            let json_str = format!("{{{}}}", json_parts.join(""));

            let c_str = match CString::new(json_str) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            DashSDKResult::success_string(c_str.into_raw())
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

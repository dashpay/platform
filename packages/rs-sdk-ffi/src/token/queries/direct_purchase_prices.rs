//! Token direct purchase prices query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dash_sdk::platform::FetchMany;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Get token direct purchase prices
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `token_ids`: Comma-separated list of Base58-encoded token IDs
///
/// # Returns
/// JSON string containing token IDs mapped to their pricing information
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_direct_purchase_prices(
    sdk_handle: *const SDKHandle,
    token_ids: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || token_ids.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or token IDs is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let ids_str = match CStr::from_ptr(token_ids).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse comma-separated token IDs
    let identifiers: Result<Vec<Identifier>, DashSDKError> = ids_str
        .split(',')
        .map(|id_str| {
            Identifier::from_string(id_str.trim(), Encoding::Base58).map_err(|e| {
                DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid token ID: {}", e),
                )
            })
        })
        .collect();

    let identifiers = match identifiers {
        Ok(ids) => ids,
        Err(e) => return DashSDKResult::error(e),
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Fetch token direct purchase prices
        let prices = TokenPricingSchedule::fetch_many(&wrapper.sdk, identifiers.as_slice())
            .await
            .map_err(FFIError::from)?;

        // Convert to JSON string
        let mut json_parts = Vec::new();
        for (token_id, price_opt) in prices {
            let price_json = match price_opt {
                Some(schedule) => {
                    // Create JSON representation of TokenPricingSchedule
                    match schedule {
                        TokenPricingSchedule::SinglePrice(price) => {
                            format!(r#"{{"type":"single_price","price":{}}}"#, price)
                        }
                        TokenPricingSchedule::SetPrices(prices) => {
                            let prices_json: Vec<String> = prices
                                .iter()
                                .map(|(amount, price)| {
                                    format!(r#"{{"amount":{},"price":{}}}"#, amount, price)
                                })
                                .collect();
                            format!(
                                r#"{{"type":"set_prices","prices":[{}]}}"#,
                                prices_json.join(",")
                            )
                        }
                    }
                }
                None => "null".to_string(),
            };
            json_parts.push(format!(
                "\"{}\":{}",
                token_id.to_string(Encoding::Base58),
                price_json
            ));
        }

        Ok(format!("{{{}}}", json_parts.join(",")))
    });

    match result {
        Ok(json_str) => {
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

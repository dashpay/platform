use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use crate::queries::ProofMetadataResponse;
use dash_sdk::platform::{Identifier, FetchMany};
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::tokens::status::TokenStatus;
use dash_sdk::dpp::tokens::status::v0::TokenStatusV0Accessors;
use dash_sdk::dpp::tokens::info::IdentityTokenInfo;
use dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dash_sdk::dpp::tokens::calculate_token_id;

/// Calculate token ID from contract ID and token position
/// 
/// This function calculates the unique token ID based on a data contract ID
/// and the position of the token within that contract.
/// 
/// # Arguments
/// * `contract_id` - The data contract ID in base58 format
/// * `token_position` - The position of the token in the contract (0-indexed)
/// 
/// # Returns
/// The calculated token ID in base58 format
/// 
/// # Example
/// ```javascript
/// const tokenId = await sdk.calculateTokenId("Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv", 0);
/// ```
#[wasm_bindgen]
pub fn calculate_token_id_from_contract(contract_id: &str, token_position: u16) -> Result<String, JsError> {
    // Parse contract ID
    let contract_identifier = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Calculate token ID
    let token_id = Identifier::from(calculate_token_id(
        contract_identifier.as_bytes(),
        token_position,
    ));
    
    // Return as base58 string
    Ok(token_id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58))
}

/// Get the current price of a token by contract ID and position
/// 
/// This is a convenience function that calculates the token ID from the contract ID
/// and position, then fetches the current pricing schedule for that token.
/// 
/// # Arguments
/// * `sdk` - The WasmSdk instance
/// * `contract_id` - The data contract ID in base58 format
/// * `token_position` - The position of the token in the contract (0-indexed)
/// 
/// # Returns
/// An object containing:
/// - `tokenId`: The calculated token ID
/// - `currentPrice`: The current price of the token
/// - `basePrice`: The base price of the token (may be same as current for single price)
/// 
/// # Example
/// ```javascript
/// const priceInfo = await sdk.getTokenPriceByContract(
///     sdk,
///     "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv",
///     0
/// );
/// console.log(`Token ${priceInfo.tokenId} current price: ${priceInfo.currentPrice}`);
/// ```
#[wasm_bindgen]
pub async fn get_token_price_by_contract(
    sdk: &WasmSdk,
    contract_id: &str,
    token_position: u16,
) -> Result<JsValue, JsError> {
    // Calculate token ID
    let token_id_string = calculate_token_id_from_contract(contract_id, token_position)?;
    
    // Parse token ID for the query
    let token_identifier = Identifier::from_string(
        &token_id_string,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Fetch token prices
    let prices_result: drive_proof_verifier::types::TokenDirectPurchasePrices = 
        TokenPricingSchedule::fetch_many(sdk.as_ref(), &[token_identifier][..])
            .await
            .map_err(|e| JsError::new(&format!("Failed to fetch token price: {}", e)))?;
    
    // Extract price information
    if let Some(price_opt) = prices_result.get(&token_identifier) {
        if let Some(schedule) = price_opt.as_ref() {
            let (base_price, current_price) = match &schedule {
                dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(price) => {
                    (price.to_string(), price.to_string())
                },
                dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SetPrices(prices) => {
                    // Use first price as base, last as current
                    let base = prices.first_key_value()
                        .map(|(_, p)| p.to_string())
                        .unwrap_or_else(|| "0".to_string());
                    let current = prices.last_key_value()
                        .map(|(_, p)| p.to_string())
                        .unwrap_or_else(|| "0".to_string());
                    (base, current)
                },
            };
            
            let response = TokenPriceResponse {
                token_id: token_id_string,
                current_price,
                base_price,
            };
            
            serde_wasm_bindgen::to_value(&response)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
        } else {
            Err(JsError::new(&format!("No pricing schedule found for token at contract {} position {}", contract_id, token_position)))
        }
    } else {
        Err(JsError::new(&format!("Token not found at contract {} position {}", contract_id, token_position)))
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IdentityTokenBalanceResponse {
    identity_id: String,
    balance: String,  // String to handle large numbers
}

#[wasm_bindgen]
pub async fn get_identities_token_balances(
    sdk: &WasmSdk,
    identity_ids: Vec<String>,
    token_id: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::tokens::identity_token_balances::IdentitiesTokenBalancesQuery;
    use drive_proof_verifier::types::identity_token_balance::IdentitiesTokenBalances;
    
    // Parse token ID
    let token_identifier = Identifier::from_string(
        token_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse identity IDs
    let identities: Result<Vec<Identifier>, _> = identity_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let identities = identities?;
    
    // Create query
    let query = IdentitiesTokenBalancesQuery {
        identity_ids: identities.clone(),
        token_id: token_identifier,
    };
    
    // Fetch balances
    let balances_result: IdentitiesTokenBalances = TokenAmount::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identities token balances: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<IdentityTokenBalanceResponse> = identity_ids
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            balances_result.get(&id).and_then(|balance_opt| {
                balance_opt.map(|balance| {
                    IdentityTokenBalanceResponse {
                        identity_id: id_str,
                        balance: balance.to_string(),
                    }
                })
            })
        })
        .collect();
    
    serde_wasm_bindgen::to_value(&responses)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TokenInfoResponse {
    token_id: String,
    is_frozen: bool,
}

#[wasm_bindgen]
pub async fn get_identity_token_infos(
    sdk: &WasmSdk,
    identity_id: &str,
    token_ids: Option<Vec<String>>,
    _limit: Option<u32>,
    _offset: Option<u32>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::tokens::token_info::IdentityTokenInfosQuery;
    use drive_proof_verifier::types::token_info::IdentityTokenInfos;
    
    // Parse identity ID
    let identity_identifier = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // If no token IDs specified, we can't query (SDK requires specific token IDs)
    let token_id_strings = token_ids.ok_or_else(|| JsError::new("token_ids are required for this query"))?;
    
    // Parse token IDs
    let tokens: Result<Vec<Identifier>, _> = token_id_strings
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let tokens = tokens?;
    
    // Create query
    let query = IdentityTokenInfosQuery {
        identity_id: identity_identifier,
        token_ids: tokens.clone(),
    };
    
    // Fetch token infos
    let infos_result: IdentityTokenInfos = IdentityTokenInfo::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity token infos: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<TokenInfoResponse> = token_id_strings
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            infos_result.get(&id).and_then(|info_opt| {
                info_opt.as_ref().map(|info| {
                    use dash_sdk::dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
                    
                    // IdentityTokenInfo only contains frozen status
                    let is_frozen = match &info {
                        dash_sdk::dpp::tokens::info::IdentityTokenInfo::V0(v0) => v0.frozen(),
                    };
                    
                    TokenInfoResponse {
                        token_id: id_str,
                        is_frozen,
                    }
                })
            })
        })
        .collect();
    
    serde_wasm_bindgen::to_value(&responses)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IdentityTokenInfoResponse {
    identity_id: String,
    is_frozen: bool,
}

#[wasm_bindgen]
pub async fn get_identities_token_infos(
    sdk: &WasmSdk,
    identity_ids: Vec<String>,
    token_id: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::tokens::token_info::IdentitiesTokenInfosQuery;
    use drive_proof_verifier::types::token_info::IdentitiesTokenInfos;
    
    // Parse token ID
    let token_identifier = Identifier::from_string(
        token_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse identity IDs
    let identities: Result<Vec<Identifier>, _> = identity_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let identities = identities?;
    
    // Create query
    let query = IdentitiesTokenInfosQuery {
        identity_ids: identities.clone(),
        token_id: token_identifier,
    };
    
    // Fetch token infos
    let infos_result: IdentitiesTokenInfos = IdentityTokenInfo::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identities token infos: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<IdentityTokenInfoResponse> = identity_ids
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            infos_result.get(&id).and_then(|info_opt| {
                info_opt.as_ref().map(|info| {
                    use dash_sdk::dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
                    
                    // IdentityTokenInfo only contains frozen status
                    let is_frozen = match &info {
                        dash_sdk::dpp::tokens::info::IdentityTokenInfo::V0(v0) => v0.frozen(),
                    };
                    
                    IdentityTokenInfoResponse {
                        identity_id: id_str,
                        is_frozen,
                    }
                })
            })
        })
        .collect();
    
    serde_wasm_bindgen::to_value(&responses)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TokenStatusResponse {
    token_id: String,
    is_paused: bool,
}

#[wasm_bindgen]
pub async fn get_token_statuses(sdk: &WasmSdk, token_ids: Vec<String>) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::token_status::TokenStatuses;
    
    // Parse token IDs
    let tokens: Result<Vec<Identifier>, _> = token_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let tokens = tokens?;
    
    // Fetch token statuses
    let statuses_result: TokenStatuses = TokenStatus::fetch_many(sdk.as_ref(), tokens.clone())
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token statuses: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<TokenStatusResponse> = token_ids
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            statuses_result.get(&id).and_then(|status_opt| {
                status_opt.as_ref().map(|status| {
                    TokenStatusResponse {
                        token_id: id_str,
                        is_paused: status.paused(),
                    }
                })
            })
        })
        .collect();
    
    serde_wasm_bindgen::to_value(&responses)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TokenPriceResponse {
    token_id: String,
    current_price: String,
    base_price: String,
}

#[wasm_bindgen]
pub async fn get_token_direct_purchase_prices(sdk: &WasmSdk, token_ids: Vec<String>) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::TokenDirectPurchasePrices;
    
    // Parse token IDs
    let tokens: Result<Vec<Identifier>, _> = token_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let tokens = tokens?;
    
    // Fetch token prices - use slice reference
    let prices_result: TokenDirectPurchasePrices = TokenPricingSchedule::fetch_many(sdk.as_ref(), &tokens[..])
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token direct purchase prices: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<TokenPriceResponse> = token_ids
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            prices_result.get(&id).and_then(|price_opt| {
                price_opt.as_ref().map(|schedule| {
                    // Get prices based on the schedule type
                    let (base_price, current_price) = match &schedule {
                        dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(price) => {
                            (price.to_string(), price.to_string())
                        },
                        dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SetPrices(prices) => {
                            // Use first price as base, last as current
                            let base = prices.first_key_value()
                                .map(|(_, p)| p.to_string())
                                .unwrap_or_else(|| "0".to_string());
                            let current = prices.last_key_value()
                                .map(|(_, p)| p.to_string())
                                .unwrap_or_else(|| "0".to_string());
                            (base, current)
                        },
                    };
                    
                    TokenPriceResponse {
                        token_id: id_str,
                        current_price,
                        base_price,
                    }
                })
            })
        })
        .collect();
    
    serde_wasm_bindgen::to_value(&responses)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TokenContractInfoResponse {
    contract_id: String,
    token_contract_position: u16,
}

#[wasm_bindgen]
pub async fn get_token_contract_info(sdk: &WasmSdk, data_contract_id: &str) -> Result<JsValue, JsError> {
    use dash_sdk::dpp::tokens::contract_info::TokenContractInfo;
    use dash_sdk::platform::Fetch;
    
    // Parse contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Fetch token contract info
    let info_result = TokenContractInfo::fetch(sdk.as_ref(), contract_id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token contract info: {}", e)))?;
    
    if let Some(info) = info_result {
        use dash_sdk::dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
        
        // Extract fields based on the enum variant
        let (contract_id, position) = match &info {
            dash_sdk::dpp::tokens::contract_info::TokenContractInfo::V0(v0) => {
                (v0.contract_id(), v0.token_contract_position())
            },
        };
        
        let response = TokenContractInfoResponse {
            contract_id: contract_id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
            token_contract_position: position,
        };
        
        // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        Ok(JsValue::NULL)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LastClaimResponse {
    last_claim_timestamp_ms: u64,
    last_claim_block_height: u64,
}

#[wasm_bindgen]
pub async fn get_token_perpetual_distribution_last_claim(
    sdk: &WasmSdk,
    identity_id: &str,
    token_id: &str,
) -> Result<JsValue, JsError> {
    
    
    
    
    // Parse IDs
    let identity_identifier = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    let token_identifier = Identifier::from_string(
        token_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Use direct gRPC request instead of high-level SDK fetch to avoid proof verification issues
    use dapi_grpc::platform::v0::{
        GetTokenPerpetualDistributionLastClaimRequest,
        get_token_perpetual_distribution_last_claim_request::{
            Version, GetTokenPerpetualDistributionLastClaimRequestV0
        }
    };
    use rs_dapi_client::DapiRequestExecutor;
    
    // Create direct gRPC Request without proofs to avoid context provider issues
    let request = GetTokenPerpetualDistributionLastClaimRequest {
        version: Some(Version::V0(GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_identifier.to_vec(),
            identity_id: identity_identifier.to_vec(),
            contract_info: None, // Not needed for this query
            prove: false, // Use prove: false to avoid proof verification and context provider dependency
        })),
    };
    
    // Execute the gRPC request
    let response = sdk.inner_sdk()
        .execute(request, rs_dapi_client::RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token perpetual distribution last claim: {}", e)))?;
    
    // Extract result from response and convert to our expected format
    let claim_result = match response.inner.version {
        Some(dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::Version::V0(v0)) => {
            match v0.result {
                Some(dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::get_token_perpetual_distribution_last_claim_response_v0::Result::LastClaim(claim)) => {
                    // Convert gRPC response to RewardDistributionMoment equivalent
                    match claim.paid_at {
                        Some(dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::get_token_perpetual_distribution_last_claim_response_v0::last_claim_info::PaidAt::TimestampMs(timestamp)) => {
                            Some((timestamp, 0)) // (timestamp_ms, block_height)
                        },
                        Some(dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::get_token_perpetual_distribution_last_claim_response_v0::last_claim_info::PaidAt::BlockHeight(height)) => {
                            Some((0, height)) // (timestamp_ms, block_height)
                        },
                        Some(dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::get_token_perpetual_distribution_last_claim_response_v0::last_claim_info::PaidAt::Epoch(epoch)) => {
                            Some((0, epoch as u64)) // (timestamp_ms, block_height)
                        },
                        Some(dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::get_token_perpetual_distribution_last_claim_response_v0::last_claim_info::PaidAt::RawBytes(bytes)) => {
                            // Raw bytes format specification (confirmed via server trace logs):
                            // - Total length: 8 bytes (big-endian encoding)
                            // - Bytes 0-3: Timestamp as u32 (seconds since Unix epoch, 0 = no timestamp recorded)
                            // - Bytes 4-7: Block height as u32 (Dash blockchain block number)
                            //
                            // Validation ranges:
                            // - Timestamp: 0 (unset) or >= 1609459200 (Jan 1, 2021 00:00:00 UTC, before Dash Platform mainnet)
                            // - Block height: 0 (invalid) or >= 1 (valid blockchain height)
                            if bytes.len() >= 8 {
                                let timestamp = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64;
                                let block_height = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as u64;
                                
                                // Validate timestamp: must be 0 (unset) or a reasonable Unix timestamp
                                let validated_timestamp = if timestamp != 0 && timestamp < 1609459200 {
                                    web_sys::console::warn_1(&format!("Invalid timestamp in raw bytes: {} (too early)", timestamp).into());
                                    0 // Use 0 for invalid timestamps
                                } else {
                                    timestamp
                                };
                                
                                // Validate block height: must be a positive value
                                let validated_block_height = if block_height == 0 {
                                    web_sys::console::warn_1(&"Invalid block height in raw bytes: 0 (genesis block not expected)".into());
                                    1 // Use minimum valid block height
                                } else {
                                    block_height
                                };
                                
                                Some((validated_timestamp * 1000, validated_block_height)) // Convert timestamp to milliseconds
                            } else if bytes.len() >= 4 {
                                // Fallback: decode only the last 4 bytes as block height
                                let block_height = u32::from_be_bytes([
                                    bytes[bytes.len()-4], bytes[bytes.len()-3], 
                                    bytes[bytes.len()-2], bytes[bytes.len()-1]
                                ]) as u64;
                                
                                // Validate block height
                                let validated_block_height = if block_height == 0 {
                                    web_sys::console::warn_1(&"Invalid block height in fallback parsing: 0".into());
                                    1 // Use minimum valid block height
                                } else {
                                    block_height
                                };
                                
                                Some((0, validated_block_height))
                            } else {
                                web_sys::console::warn_1(&format!("Insufficient raw bytes length: {} (expected 8 or 4)", bytes.len()).into());
                                Some((0, 0))
                            }
                        },
                        None => {
                            None // No paid_at info
                        }
                    }
                },
                Some(dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::get_token_perpetual_distribution_last_claim_response_v0::Result::Proof(_)) => {
                    return Err(JsError::new("Received proof instead of data - this should not happen with prove: false"))
                },
                None => None, // No claim found
            }
        },
        None => {
            return Err(JsError::new("Invalid response version"))
        }
    };
    
    if let Some((timestamp_ms, block_height)) = claim_result {
        let response = LastClaimResponse {
            last_claim_timestamp_ms: timestamp_ms,
            last_claim_block_height: block_height,
        };
        
        // Use json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        response.serialize(&serializer)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        Ok(JsValue::NULL)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TokenTotalSupplyResponse {
    total_supply: String,
}

#[wasm_bindgen]
pub async fn get_token_total_supply(sdk: &WasmSdk, token_id: &str) -> Result<JsValue, JsError> {
    use dash_sdk::dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
    use dash_sdk::platform::Fetch;
    
    // Parse token ID
    let token_identifier = Identifier::from_string(
        token_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Fetch total supply
    let supply_result = TotalSingleTokenBalance::fetch(sdk.as_ref(), token_identifier)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token total supply: {}", e)))?;
    
    if let Some(supply) = supply_result {
        let response = TokenTotalSupplyResponse {
            total_supply: supply.token_supply.to_string(),
        };
        
        // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        Ok(JsValue::NULL)
    }
}

// Proof versions for token queries

#[wasm_bindgen]
pub async fn get_identities_token_balances_with_proof_info(
    sdk: &WasmSdk,
    identity_ids: Vec<String>,
    token_id: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::tokens::identity_token_balances::IdentitiesTokenBalancesQuery;
    
    
    // Parse token ID
    let token_identifier = Identifier::from_string(
        token_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse identity IDs
    let identities: Result<Vec<Identifier>, _> = identity_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let identities = identities?;
    
    // Create query
    let query = IdentitiesTokenBalancesQuery {
        identity_ids: identities.clone(),
        token_id: token_identifier,
    };
    
    // Fetch balances with proof
    let (balances_result, metadata, proof): (drive_proof_verifier::types::identity_token_balance::IdentitiesTokenBalances, _, _) = TokenAmount::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identities token balances with proof: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<IdentityTokenBalanceResponse> = identity_ids
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            balances_result.get(&id).and_then(|balance_opt| {
                balance_opt.map(|balance| {
                    IdentityTokenBalanceResponse {
                        identity_id: id_str,
                        balance: balance.to_string(),
                    }
                })
            })
        })
        .collect();
    
    let response = ProofMetadataResponse {
        data: responses,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_token_statuses_with_proof_info(sdk: &WasmSdk, token_ids: Vec<String>) -> Result<JsValue, JsError> {
    
    
    // Parse token IDs
    let tokens: Result<Vec<Identifier>, _> = token_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let tokens = tokens?;
    
    // Fetch token statuses with proof
    let (statuses_result, metadata, proof) = TokenStatus::fetch_many_with_metadata_and_proof(sdk.as_ref(), tokens.clone(), None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token statuses with proof: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<TokenStatusResponse> = token_ids
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            statuses_result.get(&id).and_then(|status_opt| {
                status_opt.as_ref().map(|status| {
                    TokenStatusResponse {
                        token_id: id_str,
                        is_paused: status.paused(),
                    }
                })
            })
        })
        .collect();
    
    let response = ProofMetadataResponse {
        data: responses,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_token_total_supply_with_proof_info(sdk: &WasmSdk, token_id: &str) -> Result<JsValue, JsError> {
    use dash_sdk::dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
    use dash_sdk::platform::Fetch;
    
    // Parse token ID
    let token_identifier = Identifier::from_string(
        token_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Fetch total supply with proof
    let (supply_result, metadata, proof) = TotalSingleTokenBalance::fetch_with_metadata_and_proof(sdk.as_ref(), token_identifier, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token total supply with proof: {}", e)))?;
    
    let data = if let Some(supply) = supply_result {
        Some(TokenTotalSupplyResponse {
            total_supply: supply.token_supply.to_string(),
        })
    } else {
        None
    };
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

// Additional proof info versions for remaining token queries

#[wasm_bindgen]
pub async fn get_identity_token_infos_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
    token_ids: Option<Vec<String>>,
    _limit: Option<u32>,
    _offset: Option<u32>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::tokens::token_info::IdentityTokenInfosQuery;
    use drive_proof_verifier::types::token_info::IdentityTokenInfos;
    
    // Parse identity ID
    let identity_identifier = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // If no token IDs specified, we can't query (SDK requires specific token IDs)
    let token_id_strings = token_ids.ok_or_else(|| JsError::new("token_ids are required for this query"))?;
    
    // Parse token IDs
    let tokens: Result<Vec<Identifier>, _> = token_id_strings
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let tokens = tokens?;
    
    // Create query
    let query = IdentityTokenInfosQuery {
        identity_id: identity_identifier,
        token_ids: tokens.clone(),
    };
    
    // Fetch token infos with proof
    let (infos_result, metadata, proof): (IdentityTokenInfos, _, _) = IdentityTokenInfo::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity token infos with proof: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<TokenInfoResponse> = token_id_strings
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            infos_result.get(&id).and_then(|info_opt| {
                info_opt.as_ref().map(|info| {
                    use dash_sdk::dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
                    
                    // IdentityTokenInfo only contains frozen status
                    let is_frozen = match &info {
                        dash_sdk::dpp::tokens::info::IdentityTokenInfo::V0(v0) => v0.frozen(),
                    };
                    
                    TokenInfoResponse {
                        token_id: id_str,
                        is_frozen,
                    }
                })
            })
        })
        .collect();
    
    let response = ProofMetadataResponse {
        data: responses,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identities_token_infos_with_proof_info(
    sdk: &WasmSdk,
    identity_ids: Vec<String>,
    token_id: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::tokens::token_info::IdentitiesTokenInfosQuery;
    use drive_proof_verifier::types::token_info::IdentitiesTokenInfos;
    
    // Parse token ID
    let token_identifier = Identifier::from_string(
        token_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse identity IDs
    let identities: Result<Vec<Identifier>, _> = identity_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let identities = identities?;
    
    // Create query
    let query = IdentitiesTokenInfosQuery {
        identity_ids: identities.clone(),
        token_id: token_identifier,
    };
    
    // Fetch token infos with proof
    let (infos_result, metadata, proof): (IdentitiesTokenInfos, _, _) = IdentityTokenInfo::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identities token infos with proof: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<IdentityTokenInfoResponse> = identity_ids
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            infos_result.get(&id).and_then(|info_opt| {
                info_opt.as_ref().map(|info| {
                    use dash_sdk::dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
                    
                    // IdentityTokenInfo only contains frozen status
                    let is_frozen = match &info {
                        dash_sdk::dpp::tokens::info::IdentityTokenInfo::V0(v0) => v0.frozen(),
                    };
                    
                    IdentityTokenInfoResponse {
                        identity_id: id_str,
                        is_frozen,
                    }
                })
            })
        })
        .collect();
    
    let response = ProofMetadataResponse {
        data: responses,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_token_direct_purchase_prices_with_proof_info(sdk: &WasmSdk, token_ids: Vec<String>) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::TokenDirectPurchasePrices;
    
    // Parse token IDs
    let tokens: Result<Vec<Identifier>, _> = token_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect();
    let tokens = tokens?;
    
    // Fetch token prices with proof - use slice reference
    let (prices_result, metadata, proof): (TokenDirectPurchasePrices, _, _) = TokenPricingSchedule::fetch_many_with_metadata_and_proof(sdk.as_ref(), &tokens[..], None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token direct purchase prices with proof: {}", e)))?;
    
    // Convert to response format
    let responses: Vec<TokenPriceResponse> = token_ids
        .into_iter()
        .filter_map(|id_str| {
            let id = Identifier::from_string(
                &id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            ).ok()?;
            
            prices_result.get(&id).and_then(|price_opt| {
                price_opt.as_ref().map(|schedule| {
                    // Get prices based on the schedule type
                    let (base_price, current_price) = match &schedule {
                        dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(price) => {
                            (price.to_string(), price.to_string())
                        },
                        dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SetPrices(prices) => {
                            // Use first price as base, last as current
                            let base = prices.first_key_value()
                                .map(|(_, p)| p.to_string())
                                .unwrap_or_else(|| "0".to_string());
                            let current = prices.last_key_value()
                                .map(|(_, p)| p.to_string())
                                .unwrap_or_else(|| "0".to_string());
                            (base, current)
                        },
                    };
                    
                    TokenPriceResponse {
                        token_id: id_str,
                        current_price,
                        base_price,
                    }
                })
            })
        })
        .collect();
    
    let response = ProofMetadataResponse {
        data: responses,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_token_contract_info_with_proof_info(sdk: &WasmSdk, data_contract_id: &str) -> Result<JsValue, JsError> {
    use dash_sdk::dpp::tokens::contract_info::TokenContractInfo;
    use dash_sdk::platform::Fetch;
    
    // Parse contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Fetch token contract info with proof
    let (info_result, metadata, proof) = TokenContractInfo::fetch_with_metadata_and_proof(sdk.as_ref(), contract_id, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token contract info with proof: {}", e)))?;
    
    let data = if let Some(info) = info_result {
        use dash_sdk::dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
        
        // Extract fields based on the enum variant
        let (contract_id, position) = match &info {
            dash_sdk::dpp::tokens::contract_info::TokenContractInfo::V0(v0) => {
                (v0.contract_id(), v0.token_contract_position())
            },
        };
        
        Some(TokenContractInfoResponse {
            contract_id: contract_id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
            token_contract_position: position,
        })
    } else {
        None
    };
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_token_perpetual_distribution_last_claim_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
    token_id: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::query::TokenLastClaimQuery;
    use dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
    use dash_sdk::platform::Fetch;
    
    // Parse IDs
    let identity_identifier = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    let token_identifier = Identifier::from_string(
        token_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Create query
    let query = TokenLastClaimQuery {
        token_id: token_identifier,
        identity_id: identity_identifier,
    };
    
    // Fetch last claim info with proof
    let (claim_result, metadata, proof) = RewardDistributionMoment::fetch_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token perpetual distribution last claim with proof: {}", e)))?;
    
    let data = if let Some(moment) = claim_result {
        // Extract timestamp and block height based on the moment type
        // Since we need both timestamp and block height in the response,
        // we'll return the moment value and type
        let (last_claim_timestamp_ms, last_claim_block_height) = match moment {
            dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment::BlockBasedMoment(height) => {
                (0, height) // No timestamp available for block-based
            },
            dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment::TimeBasedMoment(timestamp) => {
                (timestamp, 0) // No block height available for time-based
            },
            dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment::EpochBasedMoment(epoch) => {
                (0, epoch as u64) // Convert epoch to u64, no timestamp available
            },
        };
        
        Some(LastClaimResponse {
            last_claim_timestamp_ms,
            last_claim_block_height,
        })
    } else {
        None
    };
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}
use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use dash_sdk::platform::{Identifier, FetchMany};
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::tokens::status::TokenStatus;
use dash_sdk::dpp::tokens::status::v0::TokenStatusV0Accessors;
use dash_sdk::dpp::tokens::info::IdentityTokenInfo;
use dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

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
    _with_purchase_info: Option<bool>,
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
    _with_purchase_info: Option<bool>,
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
        
        serde_wasm_bindgen::to_value(&response)
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
    
    // Fetch last claim info
    let claim_result = RewardDistributionMoment::fetch(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch token perpetual distribution last claim: {}", e)))?;
    
    if let Some(moment) = claim_result {
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
        
        let response = LastClaimResponse {
            last_claim_timestamp_ms,
            last_claim_block_height,
        };
        
        serde_wasm_bindgen::to_value(&response)
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
        
        serde_wasm_bindgen::to_value(&response)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        Ok(JsValue::NULL)
    }
}
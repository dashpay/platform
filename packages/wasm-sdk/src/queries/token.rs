use crate::error::WasmSdkError;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::tokens::calculate_token_id;
use dash_sdk::dpp::tokens::info::IdentityTokenInfo;
use dash_sdk::dpp::tokens::status::TokenStatus;
use dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dash_sdk::platform::{FetchMany, Identifier};
use js_sys::{BigInt, Map};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::tokens::{IdentityTokenInfoWasm, TokenContractInfoWasm, TokenStatusWasm};

#[wasm_bindgen(js_name = "TokenPriceInfo")]
#[derive(Clone)]
pub struct TokenPriceInfoWasm {
    token_id: IdentifierWasm,
    current_price: String,
    base_price: String,
}

impl TokenPriceInfoWasm {
    fn new(token_id: IdentifierWasm, current_price: String, base_price: String) -> Self {
        Self {
            token_id,
            current_price,
            base_price,
        }
    }
}

#[wasm_bindgen(js_class = TokenPriceInfo)]
impl TokenPriceInfoWasm {
    #[wasm_bindgen(getter = "tokenId")]
    pub fn token_id(&self) -> IdentifierWasm {
        self.token_id
    }

    #[wasm_bindgen(getter = "currentPrice")]
    pub fn current_price(&self) -> String {
        self.current_price.clone()
    }

    #[wasm_bindgen(getter = "basePrice")]
    pub fn base_price(&self) -> String {
        self.base_price.clone()
    }
}

#[wasm_bindgen(js_name = "TokenLastClaim")]
#[derive(Clone)]
pub struct TokenLastClaimWasm {
    last_claim_timestamp_ms: u64,
    last_claim_block_height: u64,
}

impl TokenLastClaimWasm {
    fn new(last_claim_timestamp_ms: u64, last_claim_block_height: u64) -> Self {
        Self {
            last_claim_timestamp_ms,
            last_claim_block_height,
        }
    }
}

#[wasm_bindgen(js_class = TokenLastClaim)]
impl TokenLastClaimWasm {
    #[wasm_bindgen(getter = "lastClaimTimestampMs")]
    pub fn last_claim_timestamp_ms(&self) -> u64 {
        self.last_claim_timestamp_ms
    }

    #[wasm_bindgen(getter = "lastClaimBlockHeight")]
    pub fn last_claim_block_height(&self) -> u64 {
        self.last_claim_block_height
    }
}

#[wasm_bindgen(js_name = "TokenTotalSupply")]
#[derive(Clone)]
pub struct TokenTotalSupplyWasm {
    total_supply: u64,
}

impl TokenTotalSupplyWasm {
    fn new(total_supply: u64) -> Self {
        Self { total_supply }
    }
}

#[wasm_bindgen(js_class = TokenTotalSupply)]
impl TokenTotalSupplyWasm {
    #[wasm_bindgen(getter = "totalSupply")]
    pub fn total_supply(&self) -> BigInt {
        BigInt::from(self.total_supply)
    }
}

#[wasm_bindgen]
impl WasmSdk {
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
    #[wasm_bindgen(js_name = "calculateTokenIdFromContract")]
    pub fn calculate_token_id_from_contract(
        contract_id: &str,
        token_position: u16,
    ) -> Result<String, WasmSdkError> {
        // Parse contract ID
        let contract_identifier = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

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
    /// console.log(`Token ${priceInfo.tokenId.base58()} current price: ${priceInfo.currentPrice}`);
    /// ```
    #[wasm_bindgen(js_name = "getTokenPriceByContract")]
    pub async fn get_token_price_by_contract(
        &self,
        contract_id: &str,
        token_position: u16,
    ) -> Result<TokenPriceInfoWasm, WasmSdkError> {
        // Calculate token ID
        let token_id_string = Self::calculate_token_id_from_contract(contract_id, token_position)?;

        // Parse token ID for the query
        let token_identifier = Identifier::from_string(
            &token_id_string,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;
        let token_identifier_wasm = IdentifierWasm::from(token_identifier.clone());

        // Fetch token prices
        let prices_result: drive_proof_verifier::types::TokenDirectPurchasePrices =
            TokenPricingSchedule::fetch_many(self.as_ref(), &[token_identifier][..]).await?;

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

                Ok(TokenPriceInfoWasm::new(token_identifier_wasm, current_price, base_price))
            } else {
                Err(WasmSdkError::not_found(format!(
                    "No pricing schedule found for token at contract {} position {}",
                    contract_id, token_position
                )))
            }
        } else {
            Err(WasmSdkError::not_found(format!(
                "Token not found at contract {} position {}",
                contract_id, token_position
            )))
        }
    }

    #[wasm_bindgen(js_name = "getIdentitiesTokenBalances")]
    pub async fn get_identities_token_balances(
        &self,
        identity_ids: Vec<String>,
        token_id: &str,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::tokens::identity_token_balances::IdentitiesTokenBalancesQuery;
        use drive_proof_verifier::types::identity_token_balance::IdentitiesTokenBalances;

        // Parse token ID
        let token_identifier = Identifier::from_string(
            token_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Parse identity IDs
        let identities: Result<Vec<Identifier>, _> = identity_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let identities = identities
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Create query
        let query = IdentitiesTokenBalancesQuery {
            identity_ids: identities.clone(),
            token_id: token_identifier,
        };

        // Fetch balances
        let balances_result: IdentitiesTokenBalances =
            TokenAmount::fetch_many(self.as_ref(), query).await?;

        let balances_map = Map::new();
        for identifier in &identities {
            if let Some(Some(balance)) = balances_result.get(identifier) {
                let key = JsValue::from(IdentifierWasm::from((*identifier).clone()));
                let value = JsValue::from(BigInt::from(*balance as u64));
                balances_map.set(&key, &value);
            }
        }

        Ok(balances_map)
    }

    #[wasm_bindgen(js_name = "getIdentityTokenInfos")]
    pub async fn get_identity_token_infos(
        &self,
        identity_id: &str,
        token_ids: Vec<String>,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::tokens::token_info::IdentityTokenInfosQuery;
        use drive_proof_verifier::types::token_info::IdentityTokenInfos;

        // Parse identity ID
        let identity_identifier = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Parse token IDs
        let tokens: Result<Vec<Identifier>, _> = token_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let tokens = tokens
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Create query
        let query = IdentityTokenInfosQuery {
            identity_id: identity_identifier,
            token_ids: tokens.clone(),
        };

        // Fetch token infos
        let infos_result: IdentityTokenInfos =
            IdentityTokenInfo::fetch_many(self.as_ref(), query).await?;

        let infos_map = Map::new();
        for token in tokens {
            if let Some(Some(info)) = infos_result.get(&token) {
                let info_wasm = IdentityTokenInfoWasm::from(info.clone());
                let key = JsValue::from(IdentifierWasm::from(token));
                let value = JsValue::from(info_wasm);
                infos_map.set(&key, &value);
            }
        }

        Ok(infos_map)
    }

    #[wasm_bindgen(js_name = "getIdentitiesTokenInfos")]
    pub async fn get_identities_token_infos(
        &self,
        identity_ids: Vec<String>,
        token_id: &str,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::tokens::token_info::IdentitiesTokenInfosQuery;
        use drive_proof_verifier::types::token_info::IdentitiesTokenInfos;

        // Parse token ID
        let token_identifier = Identifier::from_string(
            token_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Parse identity IDs
        let identities: Result<Vec<Identifier>, _> = identity_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let identities = identities
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Create query
        let query = IdentitiesTokenInfosQuery {
            identity_ids: identities.clone(),
            token_id: token_identifier,
        };

        // Fetch token infos
        let infos_result: IdentitiesTokenInfos =
            IdentityTokenInfo::fetch_many(self.as_ref(), query).await?;

        let infos_map = Map::new();
        for identity in identities {
            if let Some(Some(info)) = infos_result.get(&identity) {
                let info_wasm = IdentityTokenInfoWasm::from(info.clone());
                let key = JsValue::from(IdentifierWasm::from(identity));
                let value = JsValue::from(info_wasm);
                infos_map.set(&key, &value);
            }
        }

        Ok(infos_map)
    }

    #[wasm_bindgen(js_name = "getTokenStatuses")]
    pub async fn get_token_statuses(
        &self,
        token_ids: Vec<String>,
    ) -> Result<Map, WasmSdkError> {
        use drive_proof_verifier::types::token_status::TokenStatuses;

        // Parse token IDs
        let tokens: Result<Vec<Identifier>, _> = token_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let tokens = tokens
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Fetch token statuses
        let statuses_result: TokenStatuses =
            TokenStatus::fetch_many(self.as_ref(), tokens.clone()).await?;

        let statuses_map = Map::new();
        for token in tokens {
            if let Some(Some(status)) = statuses_result.get(&token) {
                let key = JsValue::from(IdentifierWasm::from(token));
                let value = JsValue::from(TokenStatusWasm::from(status.clone()));
                statuses_map.set(&key, &value);
            }
        }

        Ok(statuses_map)
    }

    #[wasm_bindgen(js_name = "getTokenDirectPurchasePrices")]
    pub async fn get_token_direct_purchase_prices(
        &self,
        token_ids: Vec<String>,
    ) -> Result<Map, WasmSdkError> {
        use drive_proof_verifier::types::TokenDirectPurchasePrices;

        // Parse token IDs
        let tokens: Result<Vec<Identifier>, _> = token_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let tokens = tokens
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Fetch token prices - use slice reference
        let prices_result: TokenDirectPurchasePrices =
            TokenPricingSchedule::fetch_many(self.as_ref(), &tokens[..]).await?;

        // Convert to response format
        let prices_map = Map::new();
        for token in tokens {
            if let Some(Some(schedule)) = prices_result.get(&token) {
                let token_id_wasm = IdentifierWasm::from(token.clone());
                let (base_price, current_price) = match schedule {
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(price) => {
                        (price.to_string(), price.to_string())
                    }
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SetPrices(prices) => {
                        let base = prices
                            .first_key_value()
                            .map(|(_, p)| p.to_string())
                            .unwrap_or_else(|| "0".to_string());
                        let current = prices
                            .last_key_value()
                            .map(|(_, p)| p.to_string())
                            .unwrap_or_else(|| "0".to_string());
                        (base, current)
                    }
                };

                let price_info =
                    TokenPriceInfoWasm::new(token_id_wasm, current_price, base_price);

                let key = JsValue::from(token_id_wasm);
                let value = JsValue::from(price_info);
                prices_map.set(&key, &value);
            }
        }

        Ok(prices_map)
    }

    #[wasm_bindgen(js_name = "getTokenContractInfo")]
    pub async fn get_token_contract_info(
        &self,
        data_contract_id: &str,
    ) -> Result<Option<TokenContractInfoWasm>, WasmSdkError> {
        use dash_sdk::dpp::tokens::contract_info::TokenContractInfo;
        use dash_sdk::platform::Fetch;

        // Parse contract ID
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Fetch token contract info
        let info_result = TokenContractInfo::fetch(self.as_ref(), contract_id).await?;

        Ok(info_result.map(TokenContractInfoWasm::from))
    }

    #[wasm_bindgen(js_name = "getTokenPerpetualDistributionLastClaim")]
    pub async fn get_token_perpetual_distribution_last_claim(
        &self,
        identity_id: &str,
        token_id: &str,
    ) -> Result<Option<TokenLastClaimWasm>, WasmSdkError> {
        // Parse IDs
        let identity_identifier = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let token_identifier = Identifier::from_string(
            token_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Use direct gRPC request instead of high-level SDK fetch to avoid proof verification issues
        use dapi_grpc::platform::v0::{
            get_token_perpetual_distribution_last_claim_request::{
                GetTokenPerpetualDistributionLastClaimRequestV0, Version,
            },
            GetTokenPerpetualDistributionLastClaimRequest,
        };
        use rs_dapi_client::DapiRequestExecutor;

        // Create direct gRPC Request without proofs to avoid context provider issues
        let request = GetTokenPerpetualDistributionLastClaimRequest {
            version: Some(Version::V0(
                GetTokenPerpetualDistributionLastClaimRequestV0 {
                    token_id: token_identifier.to_vec(),
                    identity_id: identity_identifier.to_vec(),
                    contract_info: None, // Not needed for this query
                    prove: false, // Use prove: false to avoid proof verification and context provider dependency
                },
            )),
        };

        // Execute the gRPC request
        let response = self
            .inner_sdk()
            .execute(request, rs_dapi_client::RequestSettings::default())
            .await
            .map_err(|e| {
                WasmSdkError::generic(format!(
                    "Failed to fetch token perpetual distribution last claim: {}",
                    e
                ))
            })?;

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
                                    tracing::warn!(target = "wasm_sdk", timestamp, "Invalid timestamp in raw bytes (too early)");
                                        0 // Use 0 for invalid timestamps
                                    } else {
                                        timestamp
                                    };

                                    // Validate block height: must be a positive value
                                    let validated_block_height = if block_height == 0 {
                                    tracing::warn!(target = "wasm_sdk", "Invalid block height in raw bytes: 0 (genesis block not expected)");
                                        1 // Use minimum valid block height
                                    } else {
                                        block_height
                                    };

                                    Some((validated_timestamp * 1000, validated_block_height)) // Convert timestamp to milliseconds
                                } else if bytes.len() >= 4 {
                                    // Fallback: decode only the last 4 bytes as block height
                                    let block_height = u32::from_be_bytes([
                                        bytes[bytes.len() - 4], bytes[bytes.len() - 3],
                                        bytes[bytes.len() - 2], bytes[bytes.len() - 1]
                                    ]) as u64;

                                    // Validate block height
                                    let validated_block_height = if block_height == 0 {
                                    tracing::warn!(target = "wasm_sdk", "Invalid block height in fallback parsing: 0");
                                        1 // Use minimum valid block height
                                    } else {
                                        block_height
                                    };

                                    Some((0, validated_block_height))
                                } else {
                                    tracing::warn!(target = "wasm_sdk", len = bytes.len(), "Insufficient raw bytes length (expected 8 or 4)");
                                    Some((0, 0))
                                }
                            },
                            None => {
                                None // No paid_at info
                            }
                        }
                    },
                    Some(dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::get_token_perpetual_distribution_last_claim_response_v0::Result::Proof(_)) => {
                        return Err(WasmSdkError::generic("Received proof instead of data - this should not happen with prove: false"))
                    },
                    None => None, // No claim found
                }
            },
            None => {
                return Err(WasmSdkError::generic("Invalid response version"))
            }
        };

        Ok(claim_result.map(|(timestamp_ms, block_height)| {
            TokenLastClaimWasm::new(timestamp_ms, block_height)
        }))
    }

    #[wasm_bindgen(js_name = "getTokenTotalSupply")]
    pub async fn get_token_total_supply(
        &self,
        token_id: &str,
    ) -> Result<Option<TokenTotalSupplyWasm>, WasmSdkError> {
        use dash_sdk::dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
        use dash_sdk::platform::Fetch;

        // Parse token ID
        let token_identifier = Identifier::from_string(
            token_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Fetch total supply
        let supply_result = TotalSingleTokenBalance::fetch(self.as_ref(), token_identifier).await?;

        Ok(supply_result.map(|supply| {
            TokenTotalSupplyWasm::new(supply.token_supply as u64)
        }))
    }

    // Proof versions for token queries

    #[wasm_bindgen(js_name = "getIdentitiesTokenBalancesWithProofInfo")]
    pub async fn get_identities_token_balances_with_proof_info(
        &self,
        identity_ids: Vec<String>,
        token_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::tokens::identity_token_balances::IdentitiesTokenBalancesQuery;

        // Parse token ID
        let token_identifier = Identifier::from_string(
            token_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Parse identity IDs
        let identities: Result<Vec<Identifier>, _> = identity_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let identities = identities
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Create query
        let query = IdentitiesTokenBalancesQuery {
            identity_ids: identities.clone(),
            token_id: token_identifier,
        };

        // Fetch balances with proof
        let (balances_result, metadata, proof): (
            drive_proof_verifier::types::identity_token_balance::IdentitiesTokenBalances,
            _,
            _,
        ) = TokenAmount::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let balances_map = Map::new();
        for identifier in &identities {
            if let Some(Some(balance)) = balances_result.get(identifier) {
                let key = JsValue::from(IdentifierWasm::from((*identifier).clone()));
                let value = JsValue::from(BigInt::from(*balance as u64));
                balances_map.set(&key, &value);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            balances_map, metadata, proof,
        ))
    }

    #[wasm_bindgen(js_name = "getTokenStatusesWithProofInfo")]
    pub async fn get_token_statuses_with_proof_info(
        &self,
        token_ids: Vec<String>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse token IDs
        let tokens: Result<Vec<Identifier>, _> = token_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let tokens = tokens
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Fetch token statuses with proof
        let (statuses_result, metadata, proof) =
            TokenStatus::fetch_many_with_metadata_and_proof(self.as_ref(), tokens.clone(), None)
                .await?;

        let statuses_map = Map::new();
        for token in tokens {
            if let Some(Some(status)) = statuses_result.get(&token) {
                let key = JsValue::from(IdentifierWasm::from(token));
                let value = JsValue::from(TokenStatusWasm::from(status.clone()));
                statuses_map.set(&key, &value);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            statuses_map,
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(js_name = "getTokenTotalSupplyWithProofInfo")]
    pub async fn get_token_total_supply_with_proof_info(
        &self,
        token_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
        use dash_sdk::platform::Fetch;

        // Parse token ID
        let token_identifier = Identifier::from_string(
            token_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Fetch total supply with proof
        let (supply_result, metadata, proof) =
            TotalSingleTokenBalance::fetch_with_metadata_and_proof(
                self.as_ref(),
                token_identifier,
                None,
            )
            .await?;

        let data = supply_result
            .map(|supply| JsValue::from(TokenTotalSupplyWasm::new(supply.token_supply as u64)))
            .unwrap_or(JsValue::NULL);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    // Additional proof info versions for remaining token queries

    #[wasm_bindgen(js_name = "getIdentityTokenInfosWithProofInfo")]
    pub async fn get_identity_token_infos_with_proof_info(
        &self,
        identity_id: &str,
        token_ids: Vec<String>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::tokens::token_info::IdentityTokenInfosQuery;
        use drive_proof_verifier::types::token_info::IdentityTokenInfos;

        // Parse identity ID
        let identity_identifier = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Parse token IDs
        let tokens: Result<Vec<Identifier>, _> = token_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let tokens = tokens
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Create query
        let query = IdentityTokenInfosQuery {
            identity_id: identity_identifier,
            token_ids: tokens.clone(),
        };

        // Fetch token infos with proof
        let (infos_result, metadata, proof): (IdentityTokenInfos, _, _) =
            IdentityTokenInfo::fetch_many_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        let infos_map = Map::new();
        for token in tokens {
            if let Some(Some(info)) = infos_result.get(&token) {
                let info_wasm = IdentityTokenInfoWasm::from(info.clone());
                let key = JsValue::from(IdentifierWasm::from(token));
                let value = JsValue::from(info_wasm);
                infos_map.set(&key, &value);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            infos_map, metadata, proof,
        ))
    }

    #[wasm_bindgen(js_name = "getIdentitiesTokenInfosWithProofInfo")]
    pub async fn get_identities_token_infos_with_proof_info(
        &self,
        identity_ids: Vec<String>,
        token_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::tokens::token_info::IdentitiesTokenInfosQuery;
        use drive_proof_verifier::types::token_info::IdentitiesTokenInfos;

        // Parse token ID
        let token_identifier = Identifier::from_string(
            token_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Parse identity IDs
        let identities: Result<Vec<Identifier>, _> = identity_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let identities = identities
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Create query
        let query = IdentitiesTokenInfosQuery {
            identity_ids: identities.clone(),
            token_id: token_identifier,
        };

        // Fetch token infos with proof
        let (infos_result, metadata, proof): (IdentitiesTokenInfos, _, _) =
            IdentityTokenInfo::fetch_many_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        let infos_map = Map::new();
        for identity in identities {
            if let Some(Some(info)) = infos_result.get(&identity) {
                let info_wasm = IdentityTokenInfoWasm::from(info.clone());
                let key = JsValue::from(IdentifierWasm::from(identity));
                let value = JsValue::from(info_wasm);
                infos_map.set(&key, &value);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            infos_map, metadata, proof,
        ))
    }

    #[wasm_bindgen(js_name = "getTokenDirectPurchasePricesWithProofInfo")]
    pub async fn get_token_direct_purchase_prices_with_proof_info(
        &self,
        token_ids: Vec<String>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use drive_proof_verifier::types::TokenDirectPurchasePrices;

        // Parse token IDs
        let tokens: Result<Vec<Identifier>, _> = token_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect();
        let tokens = tokens
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Fetch token prices with proof - use slice reference
        let (prices_result, metadata, proof): (TokenDirectPurchasePrices, _, _) =
            TokenPricingSchedule::fetch_many_with_metadata_and_proof(
                self.as_ref(),
                &tokens[..],
                None,
            )
            .await?;

        let prices_map = Map::new();
        for token in tokens {
            if let Some(Some(schedule)) = prices_result.get(&token) {
                let token_id_wasm = IdentifierWasm::from(token.clone());
                let (base_price, current_price) = match schedule {
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(price) => {
                        (price.to_string(), price.to_string())
                    }
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SetPrices(prices) => {
                        let base = prices
                            .first_key_value()
                            .map(|(_, p)| p.to_string())
                            .unwrap_or_else(|| "0".to_string());
                        let current = prices
                            .last_key_value()
                            .map(|(_, p)| p.to_string())
                            .unwrap_or_else(|| "0".to_string());
                        (base, current)
                    }
                };

                let price_info =
                    TokenPriceInfoWasm::new(token_id_wasm, current_price, base_price);

                let key = JsValue::from(token_id_wasm);
                let value = JsValue::from(price_info);
                prices_map.set(&key, &value);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            prices_map,
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(js_name = "getTokenContractInfoWithProofInfo")]
    pub async fn get_token_contract_info_with_proof_info(
        &self,
        data_contract_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::dpp::tokens::contract_info::TokenContractInfo;
        use dash_sdk::platform::Fetch;

        // Parse contract ID
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Fetch token contract info with proof
        let (info_result, metadata, proof) =
            TokenContractInfo::fetch_with_metadata_and_proof(self.as_ref(), contract_id, None)
                .await?;

        let data = info_result
            .map(|info| JsValue::from(TokenContractInfoWasm::from(info)))
            .unwrap_or(JsValue::UNDEFINED);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(js_name = "getTokenPerpetualDistributionLastClaimWithProofInfo")]
    pub async fn get_token_perpetual_distribution_last_claim_with_proof_info(
        &self,
        identity_id: &str,
        token_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::query::TokenLastClaimQuery;
        use dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
        use dash_sdk::platform::Fetch;

        // Parse IDs
        let identity_identifier = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let token_identifier = Identifier::from_string(
            token_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

        // Create query
        let query = TokenLastClaimQuery {
            token_id: token_identifier,
            identity_id: identity_identifier,
        };

        // Fetch last claim info with proof
        let (claim_result, metadata, proof) =
            RewardDistributionMoment::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

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

            Some(TokenLastClaimWasm::new(
                last_claim_timestamp_ms,
                last_claim_block_height,
            ))
        } else {
            None
        };

        let data = data.map(JsValue::from).unwrap_or(JsValue::UNDEFINED);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }
}

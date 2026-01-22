use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dash_sdk::dpp::tokens::calculate_token_id;
use dash_sdk::dpp::tokens::info::IdentityTokenInfo;
use dash_sdk::dpp::tokens::status::TokenStatus;
use dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dash_sdk::platform::query::TokenLastClaimQuery;
use dash_sdk::platform::{Fetch, FetchMany, Identifier};
use js_sys::{BigInt, Map};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::{
    IdentifierLikeArrayJs, IdentifierLikeJs, IdentifierWasm, identifiers_from_js_array,
};
use wasm_dpp2::tokens::{IdentityTokenInfoWasm, TokenContractInfoWasm, TokenStatusWasm};

#[wasm_bindgen(js_name = "TokenPriceInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPriceInfoWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "tokenId")]
    pub token_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "currentPrice")]
    pub current_price: String,
    #[wasm_bindgen(getter_with_clone, js_name = "basePrice")]
    pub base_price: String,
}

impl TokenPriceInfoWasm {
    pub(crate) fn new(token_id: IdentifierWasm, current_price: String, base_price: String) -> Self {
        Self {
            token_id,
            current_price,
            base_price,
        }
    }
}

#[wasm_bindgen(js_name = "RewardDistributionMoment")]
pub struct RewardDistributionMomentWasm(RewardDistributionMoment);

#[wasm_bindgen(js_class = RewardDistributionMoment)]
impl RewardDistributionMomentWasm {
    /// Returns the type: "block", "time", or "epoch"
    #[wasm_bindgen(getter = "type")]
    pub fn moment_type(&self) -> String {
        match &self.0 {
            RewardDistributionMoment::BlockBasedMoment(_) => "block".to_string(),
            RewardDistributionMoment::TimeBasedMoment(_) => "time".to_string(),
            RewardDistributionMoment::EpochBasedMoment(_) => "epoch".to_string(),
        }
    }

    /// Returns the block height (only valid when type is "block")
    #[wasm_bindgen(getter = "blockHeight")]
    pub fn block_height(&self) -> Option<u64> {
        match &self.0 {
            RewardDistributionMoment::BlockBasedMoment(height) => Some(*height),
            _ => None,
        }
    }

    /// Returns the timestamp in ms (only valid when type is "time")
    #[wasm_bindgen(getter = "timestampMs")]
    pub fn timestamp_ms(&self) -> Option<u64> {
        match &self.0 {
            RewardDistributionMoment::TimeBasedMoment(ts) => Some(*ts),
            _ => None,
        }
    }

    /// Returns the epoch index (only valid when type is "epoch")
    #[wasm_bindgen(getter = "epochIndex")]
    pub fn epoch_index(&self) -> Option<u16> {
        match &self.0 {
            RewardDistributionMoment::EpochBasedMoment(epoch) => Some(*epoch),
            _ => None,
        }
    }
}

impl From<RewardDistributionMoment> for RewardDistributionMomentWasm {
    fn from(moment: RewardDistributionMoment) -> Self {
        Self(moment)
    }
}

#[wasm_bindgen(js_name = "TokenTotalSupply")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

impl_wasm_serde_conversions!(TokenTotalSupplyWasm, TokenTotalSupply);
impl_wasm_serde_conversions!(TokenPriceInfoWasm, TokenPriceInfo);

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
        #[wasm_bindgen(js_name = "contractId")]
        contract_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "tokenPosition")] token_position: u16,
    ) -> Result<String, WasmSdkError> {
        // Parse contract ID
        let contract_identifier: Identifier = contract_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", err)))?;

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
    /// console.log(`Token ${priceInfo.tokenId.toBase58()} current price: ${priceInfo.currentPrice}`);
    /// ```
    #[wasm_bindgen(js_name = "getTokenPriceByContract")]
    pub async fn get_token_price_by_contract(
        &self,
        #[wasm_bindgen(js_name = "contractId")]
        contract_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "tokenPosition")] token_position: u16,
    ) -> Result<TokenPriceInfoWasm, WasmSdkError> {
        // Parse contract ID
        let contract_identifier: Identifier = contract_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", err)))?;

        // Calculate token ID
        let token_identifier = Identifier::from(calculate_token_id(
            contract_identifier.as_bytes(),
            token_position,
        ));
        let token_identifier_wasm = IdentifierWasm::from(token_identifier);

        // Fetch token prices
        let prices_result: drive_proof_verifier::types::TokenDirectPurchasePrices =
            TokenPricingSchedule::fetch_many(self.as_ref(), &[token_identifier][..]).await?;

        // Extract price information
        if let Some(price_opt) = prices_result.get(&token_identifier) {
            if let Some(schedule) = price_opt.as_ref() {
                let (base_price, current_price) = match &schedule {
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(
                        price,
                    ) => (price.to_string(), price.to_string()),
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SetPrices(
                        prices,
                    ) => {
                        // Use first price as base, last as current
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

                Ok(TokenPriceInfoWasm::new(
                    token_identifier_wasm,
                    current_price,
                    base_price,
                ))
            } else {
                Err(WasmSdkError::not_found(format!(
                    "No pricing schedule found for token at contract {} position {}",
                    IdentifierWasm::from(contract_identifier).to_base58(),
                    token_position
                )))
            }
        } else {
            Err(WasmSdkError::not_found(format!(
                "Token not found at contract {} position {}",
                IdentifierWasm::from(contract_identifier).to_base58(),
                token_position
            )))
        }
    }

    #[wasm_bindgen(
        js_name = "getIdentitiesTokenBalances",
        unchecked_return_type = "Map<Identifier, bigint>"
    )]
    pub async fn get_identities_token_balances(
        &self,
        #[wasm_bindgen(js_name = "identityIds")]
        identity_ids: IdentifierLikeArrayJs,
        #[wasm_bindgen(js_name = "tokenId")]
        token_id: IdentifierLikeJs,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::tokens::identity_token_balances::IdentitiesTokenBalancesQuery;
        use drive_proof_verifier::types::identity_token_balance::IdentitiesTokenBalances;

        // Parse token ID
        let token_identifier: Identifier = token_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", err)))?;

        // Parse identity IDs
        let identities = identifiers_from_js_array(identity_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid identity IDs: {}", err)))?;

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
                let key = JsValue::from(IdentifierWasm::from(*identifier));
                let value = JsValue::from(BigInt::from(*balance));
                balances_map.set(&key, &value);
            }
        }

        Ok(balances_map)
    }

    #[wasm_bindgen(
        js_name = "getIdentityTokenInfos",
        unchecked_return_type = "Map<Identifier, IdentityTokenInfo>"
    )]
    pub async fn get_identity_token_infos(
        &self,
        #[wasm_bindgen(js_name = "identityId")]
        identity_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "tokenIds")]
        token_ids: IdentifierLikeArrayJs,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::tokens::token_info::IdentityTokenInfosQuery;
        use drive_proof_verifier::types::token_info::IdentityTokenInfos;

        // Parse identity ID
        let identity_identifier: Identifier = identity_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err)))?;

        // Parse token IDs
        let tokens = identifiers_from_js_array(token_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token IDs: {}", err)))?;

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

    #[wasm_bindgen(
        js_name = "getIdentitiesTokenInfos",
        unchecked_return_type = "Map<Identifier, IdentityTokenInfo>"
    )]
    pub async fn get_identities_token_infos(
        &self,
        #[wasm_bindgen(js_name = "identityIds")]
        identity_ids: IdentifierLikeArrayJs,
        #[wasm_bindgen(js_name = "tokenId")]
        token_id: IdentifierLikeJs,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::tokens::token_info::IdentitiesTokenInfosQuery;
        use drive_proof_verifier::types::token_info::IdentitiesTokenInfos;

        // Parse token ID
        let token_identifier: Identifier = token_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", err)))?;

        // Parse identity IDs
        let identities = identifiers_from_js_array(identity_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid identity IDs: {}", err)))?;

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

    #[wasm_bindgen(
        js_name = "getTokenStatuses",
        unchecked_return_type = "Map<Identifier, TokenStatus>"
    )]
    pub async fn get_token_statuses(
        &self,
        #[wasm_bindgen(js_name = "tokenIds")]
        token_ids: IdentifierLikeArrayJs,
    ) -> Result<Map, WasmSdkError> {
        use drive_proof_verifier::types::token_status::TokenStatuses;

        // Parse token IDs
        let tokens = identifiers_from_js_array(token_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token IDs: {}", err)))?;

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

    #[wasm_bindgen(
        js_name = "getTokenDirectPurchasePrices",
        unchecked_return_type = "Map<Identifier, TokenPriceInfo>"
    )]
    pub async fn get_token_direct_purchase_prices(
        &self,
        #[wasm_bindgen(js_name = "tokenIds")]
        token_ids: IdentifierLikeArrayJs,
    ) -> Result<Map, WasmSdkError> {
        use drive_proof_verifier::types::TokenDirectPurchasePrices;

        // Parse token IDs
        let tokens = identifiers_from_js_array(token_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token IDs: {}", err)))?;

        // Fetch token prices - use slice reference
        let prices_result: TokenDirectPurchasePrices =
            TokenPricingSchedule::fetch_many(self.as_ref(), &tokens[..]).await?;

        // Convert to response format
        let prices_map = Map::new();
        for token in tokens {
            if let Some(Some(schedule)) = prices_result.get(&token) {
                let token_id_wasm = IdentifierWasm::from(token);
                let (base_price, current_price) = match schedule {
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(
                        price,
                    ) => (price.to_string(), price.to_string()),
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SetPrices(
                        prices,
                    ) => {
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

                let price_info = TokenPriceInfoWasm::new(token_id_wasm, current_price, base_price);

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
        #[wasm_bindgen(js_name = "dataContractId")]
        data_contract_id: IdentifierLikeJs,
    ) -> Result<Option<TokenContractInfoWasm>, WasmSdkError> {
        use dash_sdk::dpp::tokens::contract_info::TokenContractInfo;
        use dash_sdk::platform::Fetch;

        // Parse contract ID
        let contract_id: Identifier = data_contract_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", err)))?;

        // Fetch token contract info
        let info_result = TokenContractInfo::fetch(self.as_ref(), contract_id).await?;

        Ok(info_result.map(TokenContractInfoWasm::from))
    }

    #[wasm_bindgen(js_name = "getTokenPerpetualDistributionLastClaim")]
    pub async fn get_token_perpetual_distribution_last_claim(
        &self,
        #[wasm_bindgen(js_name = "identityId")]
        identity_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "tokenId")]
        token_id: IdentifierLikeJs,
    ) -> Result<Option<RewardDistributionMomentWasm>, WasmSdkError> {
        // Parse IDs
        let identity_identifier: Identifier = identity_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err)))?;
        let token_identifier: Identifier = token_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", err)))?;

        // Prefetch token configuration and add to context provider cache
        // This is required for proof verification to work
        self.prefetch_token_configuration(token_identifier).await?;

        // Create query and fetch via SDK with proof verification
        let query = TokenLastClaimQuery {
            token_id: token_identifier,
            identity_id: identity_identifier,
        };

        let claim_result = RewardDistributionMoment::fetch(self.as_ref(), query).await?;

        Ok(claim_result.map(RewardDistributionMomentWasm::from))
    }

    #[wasm_bindgen(js_name = "getTokenTotalSupply")]
    pub async fn get_token_total_supply(
        &self,
        #[wasm_bindgen(js_name = "tokenId")]
        token_id: IdentifierLikeJs,
    ) -> Result<Option<TokenTotalSupplyWasm>, WasmSdkError> {
        use dash_sdk::dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
        use dash_sdk::platform::Fetch;

        // Parse token ID
        let token_identifier: Identifier = token_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", err)))?;

        // Fetch total supply
        let supply_result = TotalSingleTokenBalance::fetch(self.as_ref(), token_identifier).await?;

        Ok(supply_result.map(|supply| TokenTotalSupplyWasm::new(supply.token_supply as u64)))
    }

    // Proof versions for token queries

    #[wasm_bindgen(
        js_name = "getIdentitiesTokenBalancesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, bigint>>"
    )]
    pub async fn get_identities_token_balances_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityIds")]
        identity_ids: IdentifierLikeArrayJs,
        #[wasm_bindgen(js_name = "tokenId")]
        token_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::tokens::identity_token_balances::IdentitiesTokenBalancesQuery;

        // Parse token ID
        let token_identifier: Identifier = token_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", err)))?;

        // Parse identity IDs
        let identities = identifiers_from_js_array(identity_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid identity IDs: {}", err)))?;

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
                let key = JsValue::from(IdentifierWasm::from(*identifier));
                let value = JsValue::from(BigInt::from(*balance));
                balances_map.set(&key, &value);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            balances_map,
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getTokenStatusesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, TokenStatus>>"
    )]
    pub async fn get_token_statuses_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "tokenIds")]
        token_ids: IdentifierLikeArrayJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse token IDs
        let tokens = identifiers_from_js_array(token_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token IDs: {}", err)))?;

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

    #[wasm_bindgen(
        js_name = "getTokenTotalSupplyWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<TokenTotalSupply | null>"
    )]
    pub async fn get_token_total_supply_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "tokenId")]
        token_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
        use dash_sdk::platform::Fetch;

        // Parse token ID
        let token_identifier: Identifier = token_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", err)))?;

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

    #[wasm_bindgen(
        js_name = "getIdentityTokenInfosWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, IdentityTokenInfo>>"
    )]
    pub async fn get_identity_token_infos_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")]
        identity_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "tokenIds")]
        token_ids: IdentifierLikeArrayJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::tokens::token_info::IdentityTokenInfosQuery;
        use drive_proof_verifier::types::token_info::IdentityTokenInfos;

        // Parse identity ID
        let identity_identifier: Identifier = identity_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err)))?;

        // Parse token IDs
        let tokens = identifiers_from_js_array(token_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token IDs: {}", err)))?;

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

    #[wasm_bindgen(
        js_name = "getIdentitiesTokenInfosWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, IdentityTokenInfo>>"
    )]
    pub async fn get_identities_token_infos_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityIds")]
        identity_ids: IdentifierLikeArrayJs,
        #[wasm_bindgen(js_name = "tokenId")]
        token_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::tokens::token_info::IdentitiesTokenInfosQuery;
        use drive_proof_verifier::types::token_info::IdentitiesTokenInfos;

        // Parse token ID
        let token_identifier: Identifier = token_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", err)))?;

        // Parse identity IDs
        let identities = identifiers_from_js_array(identity_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid identity IDs: {}", err)))?;

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

    #[wasm_bindgen(
        js_name = "getTokenDirectPurchasePricesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, TokenPriceInfo>>"
    )]
    pub async fn get_token_direct_purchase_prices_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "tokenIds")]
        token_ids: IdentifierLikeArrayJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use drive_proof_verifier::types::TokenDirectPurchasePrices;

        // Parse token IDs
        let tokens = identifiers_from_js_array(token_ids)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token IDs: {}", err)))?;

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
                let token_id_wasm = IdentifierWasm::from(token);
                let (base_price, current_price) = match schedule {
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(
                        price,
                    ) => (price.to_string(), price.to_string()),
                    dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SetPrices(
                        prices,
                    ) => {
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

                let price_info = TokenPriceInfoWasm::new(token_id_wasm, current_price, base_price);

                let key = JsValue::from(token_id_wasm);
                let value = JsValue::from(price_info);
                prices_map.set(&key, &value);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            prices_map, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getTokenContractInfoWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<TokenContractInfo | undefined>"
    )]
    pub async fn get_token_contract_info_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "dataContractId")]
        data_contract_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::dpp::tokens::contract_info::TokenContractInfo;
        use dash_sdk::platform::Fetch;

        // Parse contract ID
        let contract_id: Identifier = data_contract_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", err)))?;

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

    #[wasm_bindgen(
        js_name = "getTokenPerpetualDistributionLastClaimWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<TokenLastClaim | undefined>"
    )]
    pub async fn get_token_perpetual_distribution_last_claim_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")]
        identity_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "tokenId")]
        token_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
        use dash_sdk::platform::query::TokenLastClaimQuery;
        use dash_sdk::platform::Fetch;

        // Parse IDs
        let identity_identifier: Identifier = identity_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err)))?;
        let token_identifier: Identifier = token_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", err)))?;

        // Prefetch token configuration and add to context provider cache
        // This is required for proof verification to work
        self.prefetch_token_configuration(token_identifier).await?;

        // Create query
        let query = TokenLastClaimQuery {
            token_id: token_identifier,
            identity_id: identity_identifier,
        };

        // Fetch last claim info with proof
        let (claim_result, metadata, proof) =
            RewardDistributionMoment::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        let data = claim_result
            .map(RewardDistributionMomentWasm::from)
            .map(JsValue::from)
            .unwrap_or(JsValue::UNDEFINED);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }
}

// Internal helper methods for token queries
impl WasmSdk {
    /// Prefetch token configuration and add it to the context provider cache.
    /// This is required for proof verification of token-related queries.
    pub(crate) async fn prefetch_token_configuration(
        &self,
        token_id: Identifier,
    ) -> Result<(), WasmSdkError> {
        use crate::sdk::{LOCAL_TRUSTED_CONTEXT, MAINNET_TRUSTED_CONTEXT, TESTNET_TRUSTED_CONTEXT};
        use dash_sdk::dpp::dashcore::Network;
        use dash_sdk::dpp::data_contract::accessors::v1::DataContractV1Getters;
        use dash_sdk::dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
        use dash_sdk::dpp::tokens::contract_info::TokenContractInfo;

        // Step 1: Check trusted context is initialized before doing any network fetches
        let network = self.network();
        let context_initialized = match network {
            Network::Dash => MAINNET_TRUSTED_CONTEXT.lock().unwrap().is_some(),
            Network::Testnet => TESTNET_TRUSTED_CONTEXT.lock().unwrap().is_some(),
            Network::Regtest => LOCAL_TRUSTED_CONTEXT.lock().unwrap().is_some(),
            _ => false,
        };

        if !context_initialized {
            return Err(WasmSdkError::generic(format!(
                "Trusted context not initialized for network {:?}. Call prefetch methods first.",
                network
            )));
        }

        // Step 2: Fetch TokenContractInfo to get contract_id and position
        let token_contract_info = TokenContractInfo::fetch(self.as_ref(), token_id)
            .await?
            .ok_or_else(|| {
                WasmSdkError::generic(format!(
                    "Token contract info not found for token ID: {}",
                    token_id
                ))
            })?;

        let contract_id = token_contract_info.contract_id();
        let token_position = token_contract_info.token_contract_position();

        // Step 3: Fetch the DataContract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Step 4: Extract the TokenConfiguration from the contract
        let token_configuration = data_contract
            .expected_token_configuration(token_position)
            .map_err(|e| {
                WasmSdkError::generic(format!(
                    "Failed to get token configuration at position {}: {}",
                    token_position, e
                ))
            })?
            .clone();

        // Step 5: Add the token configuration to the trusted context cache
        // We already verified the context is initialized above, so unwrap is safe
        match network {
            Network::Dash => {
                let guard = MAINNET_TRUSTED_CONTEXT.lock().unwrap();
                guard
                    .as_ref()
                    .unwrap()
                    .add_known_token_configuration(token_id, token_configuration);
            }
            Network::Testnet => {
                let guard = TESTNET_TRUSTED_CONTEXT.lock().unwrap();
                guard
                    .as_ref()
                    .unwrap()
                    .add_known_token_configuration(token_id, token_configuration);
            }
            Network::Regtest => {
                let guard = LOCAL_TRUSTED_CONTEXT.lock().unwrap();
                guard
                    .as_ref()
                    .unwrap()
                    .add_known_token_configuration(token_id, token_configuration);
            }
            _ => unreachable!(), // Already checked above
        }

        Ok(())
    }
}

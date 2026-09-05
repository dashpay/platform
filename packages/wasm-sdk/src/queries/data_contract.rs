use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::platform::query::LimitQuery;
use dash_sdk::platform::{DataContract, Fetch, FetchMany, Identifier};
use drive_proof_verifier::types::{DataContractHistory, DataContracts};
use js_sys::{BigInt, Map};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::{IdentifierLikeArrayJs, IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::utils::try_to_vec;
use wasm_dpp2::DataContractWasm;

#[wasm_bindgen(typescript_custom_section)]
const DATA_CONTRACT_HISTORY_QUERY_TS: &'static str = r#"
/**
 * Query parameters for retrieving data contract history.
 */
export interface DataContractHistoryQuery {
  /**
   * Data contract identifier.
   */
  dataContractId: IdentifierLike

  /**
   * Maximum number of entries to return.
   * @default undefined
   */
  limit?: number;

  /**
   * Millisecond timestamp (inclusive) to start from.
   * @default 0
   */
  startAtMs?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DataContractHistoryQuery")]
    pub type DataContractHistoryQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataContractHistoryQueryInput {
    data_contract_id: IdentifierWasm,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    start_at_ms: Option<u64>,
}

struct DataContractHistoryQueryParsed {
    contract_id: Identifier,
    limit: Option<u32>,
    start_at_ms: Option<u64>,
}

fn parse_data_contract_history_query(
    query: DataContractHistoryQueryJs,
) -> Result<DataContractHistoryQueryParsed, WasmSdkError> {
    let input: DataContractHistoryQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "data contract history query",
    )?;
    let DataContractHistoryQueryInput {
        data_contract_id,
        limit,
        start_at_ms,
    } = input;

    let contract_id: Identifier = data_contract_id.into();

    Ok(DataContractHistoryQueryParsed {
        contract_id,
        limit,
        start_at_ms,
    })
}

fn build_limit_query(params: &DataContractHistoryQueryParsed) -> LimitQuery<(Identifier, u64)> {
    LimitQuery {
        query: (params.contract_id, params.start_at_ms.unwrap_or(0)),
        start_info: None,
        limit: params.limit,
    }
}

impl WasmSdk {
    /// Fetch one contract (proved) and seed the trusted-context cache
    /// with it, so the document queries that follow find it there
    /// instead of each fetching it again. Backs `getDataContract`.
    pub(crate) async fn fetch_contract_seeding_cache(
        &self,
        id: Identifier,
    ) -> Result<Option<DataContract>, WasmSdkError> {
        let contract = DataContract::fetch_by_identifier(self.as_ref(), id).await?;
        if let Some(contract) = &contract {
            self.cache_contract(contract.clone());
        }
        Ok(contract)
    }

    /// Fetch several contracts in one round trip (proved) and seed the
    /// trusted-context cache with every one that resolved. Backs
    /// `getDataContracts` — the natural preload call, which until this
    /// seeded nothing and left every first query to refetch.
    pub(crate) async fn fetch_contracts_seeding_cache(
        &self,
        ids: Vec<Identifier>,
    ) -> Result<DataContracts, WasmSdkError> {
        let contracts: DataContracts = DataContract::fetch_many(self.as_ref(), ids).await?;
        for contract in contracts.values().flatten() {
            self.cache_contract(contract.clone());
        }
        Ok(contracts)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getDataContract")]
    pub async fn get_data_contract(
        &self,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> Result<Option<DataContractWasm>, WasmSdkError> {
        let id: Identifier = contract_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", err))
        })?;

        let data_contract = self
            .fetch_contract_seeding_cache(id)
            .await?
            .map(DataContractWasm::from);

        Ok(data_contract)
    }

    #[wasm_bindgen(
        js_name = "getDataContractWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<DataContract>"
    )]
    pub async fn get_data_contract_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let id: Identifier = contract_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", err))
        })?;

        let (contract, metadata, proof) =
            DataContract::fetch_with_metadata_and_proof(self.as_ref(), id, None).await?;

        contract
            .map(|contract| {
                self.cache_contract(contract.clone());
                ProofMetadataResponseWasm::from_sdk_parts(
                    DataContractWasm::from(contract),
                    metadata,
                    proof,
                )
            })
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))
    }

    #[wasm_bindgen(
        js_name = "getDataContractHistory",
        unchecked_return_type = "Map<bigint, DataContract>"
    )]
    pub async fn get_data_contract_history(
        &self,
        query: DataContractHistoryQueryJs,
    ) -> Result<Map, WasmSdkError> {
        let params = parse_data_contract_history_query(query)?;
        let limit_query = build_limit_query(&params);

        let history_result = DataContractHistory::fetch(self.as_ref(), limit_query).await?;

        let history_map = Map::new();

        if let Some(history) = history_result {
            for (block_time_ms, contract) in history {
                let contract_js = JsValue::from(DataContractWasm::from(contract));
                let key = JsValue::from(BigInt::from(block_time_ms));

                history_map.set(&key, &contract_js);
            }
        }

        Ok(history_map)
    }

    #[wasm_bindgen(
        js_name = "getDataContracts",
        unchecked_return_type = "Map<string, DataContract | undefined>"
    )]
    pub async fn get_data_contracts(
        &self,
        ids: IdentifierLikeArrayJs,
    ) -> Result<Map, WasmSdkError> {
        // Parse all contract IDs
        let identifiers: Vec<Identifier> =
            try_to_vec::<IdentifierWasm, _, _>(ids, "ids", "identifier")?;

        // Fetch all contracts, seeding the cache with each one found
        let contracts_result = self.fetch_contracts_seeding_cache(identifiers).await?;

        let contracts_map = Map::new();

        for (id, contract) in contracts_result {
            let key: JsValue = IdentifierWasm::from(id).to_base58().into();
            let value = contract.map(DataContractWasm::from);
            contracts_map.set(&key, &JsValue::from(value));
        }

        Ok(contracts_map)
    }

    // Proof info versions for data contract queries

    #[wasm_bindgen(
        js_name = "getDataContractHistoryWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<bigint, DataContract>>"
    )]
    pub async fn get_data_contract_history_with_proof_info(
        &self,
        query: DataContractHistoryQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let params = parse_data_contract_history_query(query)?;
        let limit_query = build_limit_query(&params);

        let (history_result, metadata, proof) =
            DataContractHistory::fetch_with_metadata_and_proof(self.as_ref(), limit_query, None)
                .await?;

        let history_map = Map::new();

        if let Some(history) = history_result {
            for (block_time_ms, contract) in history {
                let contract_js = JsValue::from(DataContractWasm::from(contract));
                let key = JsValue::from(BigInt::from(block_time_ms));

                history_map.set(&key, &contract_js);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            history_map,
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getDataContractsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, DataContract | undefined>>"
    )]
    pub async fn get_data_contracts_with_proof_info(
        &self,
        ids: IdentifierLikeArrayJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse all contract IDs
        let identifiers: Vec<Identifier> =
            try_to_vec::<IdentifierWasm, _, _>(ids, "ids", "identifier")?;

        // Fetch all contracts with proof
        let (contracts_result, metadata, proof) =
            DataContract::fetch_many_with_metadata_and_proof(self.as_ref(), identifiers, None)
                .await?;

        let contracts_map = Map::new();

        for (id, contract_opt) in contracts_result {
            let key: JsValue = IdentifierWasm::from(id).to_base58().into();
            let value = contract_opt.map(|contract| {
                self.cache_contract(contract.clone());
                DataContractWasm::from(contract)
            });

            contracts_map.set(&key, &JsValue::from(value));
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            contracts_map,
            metadata,
            proof,
        ))
    }
}

#[cfg(test)]
mod tests {
    //! The cache-seeding contract of the contract fetch entry points:
    //! what `getDataContract` / `getDataContracts` fetch must land in
    //! the trusted-context cache, because those calls are how an app
    //! preloads its contracts — and a preload that seeds nothing leaves
    //! every first document query per contract fetching it again.

    use super::*;
    use crate::context_provider::WasmTrustedContext;
    use dash_sdk::dpp::data_contract::accessors::v0::{
        DataContractV0Getters, DataContractV0Setters,
    };
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::Sdk;

    /// Built at the SDK's own platform version, the version the mock
    /// deserializes at, so the round-tripped contract compares equal.
    fn custom_contract(id_byte: u8, platform_version: &PlatformVersion) -> DataContract {
        let mut contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("DPNS contract fixture should load");
        contract.set_id(Identifier::new([id_byte; 32]));
        contract
    }

    #[tokio::test]
    async fn a_single_contract_fetch_seeds_the_cache() {
        let mut inner_sdk = Sdk::new_mock();
        let expected = custom_contract(0xA1, inner_sdk.version());
        let id = expected.id();
        inner_sdk
            .mock()
            .expect_fetch(id, Some(expected.clone()))
            .await
            .expect("mock contract response should be configured");
        let sdk =
            WasmSdk::new_for_testing(inner_sdk, Some(WasmTrustedContext::for_testing(vec![])));
        assert!(sdk.get_cached_contract(&id).is_none());

        let fetched = sdk
            .fetch_contract_seeding_cache(id)
            .await
            .expect("proved contract fetch should succeed");

        assert_eq!(fetched.as_ref(), Some(&expected));
        assert_eq!(sdk.get_cached_contract(&id).as_deref(), Some(&expected));
    }

    #[tokio::test]
    async fn a_batched_contract_fetch_seeds_the_cache_with_every_resolved_contract() {
        let mut inner_sdk = Sdk::new_mock();
        let found = custom_contract(0xA2, inner_sdk.version());
        let missing_id = Identifier::new([0xB3; 32]);
        let ids = vec![found.id(), missing_id];
        let response: DataContracts = [(found.id(), Some(found.clone())), (missing_id, None)]
            .into_iter()
            .collect();
        inner_sdk
            .mock()
            .expect_fetch_many::<Identifier, DataContract, Vec<Identifier>, DataContracts>(
                ids.clone(),
                Some(response),
            )
            .await
            .expect("mock contracts response should be configured");
        let sdk =
            WasmSdk::new_for_testing(inner_sdk, Some(WasmTrustedContext::for_testing(vec![])));

        let fetched = sdk
            .fetch_contracts_seeding_cache(ids)
            .await
            .expect("proved batched contract fetch should succeed");

        assert_eq!(fetched.get(&found.id()), Some(&Some(found.clone())));
        assert_eq!(fetched.get(&missing_id), Some(&None));
        assert_eq!(
            sdk.get_cached_contract(&found.id()).as_deref(),
            Some(&found)
        );
        assert!(
            sdk.get_cached_contract(&missing_id).is_none(),
            "a contract the network does not have seeds nothing"
        );
    }
}

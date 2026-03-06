use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::identity::identities_contract_keys::IdentitiesContractKeys;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey;
use dash_sdk::dpp::identity::Purpose;
use dash_sdk::platform::identities_contract_keys_query::IdentitiesContractKeysQuery;
use dash_sdk::platform::{Fetch, FetchMany, Identifier, Identity, IdentityKeysQuery};
use drive_proof_verifier::types::{IdentityPublicKeys, IndexMap};
use js_sys::{Array, BigInt, Map};
use rs_dapi_client::IntoInner;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::{
    IdentifierLikeArrayJs, IdentifierLikeJs, IdentifierLikeOrUndefinedJs, IdentifierWasm,
};
use wasm_dpp2::identity::public_key::IdentityPublicKeyWasm;
use wasm_dpp2::identity::IdentityWasm;
use wasm_dpp2::{public_key_hash_from_js, PublicKeyHashLikeJs};

#[wasm_bindgen(js_name = "IdentityContractKeys")]
pub struct IdentityContractKeysWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "identityId")]
    pub identity_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub keys: Vec<IdentityPublicKeyWasm>,
}

impl IdentityContractKeysWasm {
    pub(crate) fn new(identity_id: IdentifierWasm, keys: Vec<IdentityPublicKeyWasm>) -> Self {
        IdentityContractKeysWasm { identity_id, keys }
    }
}

#[wasm_bindgen(js_name = "IdentityBalanceAndRevision")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityBalanceAndRevisionWasm {
    balance: u64,
    revision: u64,
}

impl IdentityBalanceAndRevisionWasm {
    fn new(balance: u64, revision: u64) -> Self {
        IdentityBalanceAndRevisionWasm { balance, revision }
    }
}

#[wasm_bindgen(js_class = IdentityBalanceAndRevision)]
impl IdentityBalanceAndRevisionWasm {
    #[wasm_bindgen(getter = "balance")]
    pub fn balance(&self) -> BigInt {
        BigInt::from(self.balance)
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

impl_wasm_serde_conversions!(IdentityBalanceAndRevisionWasm, IdentityBalanceAndRevision);
#[wasm_bindgen(typescript_custom_section)]
const IDENTITIES_CONTRACT_KEYS_QUERY_TS: &'static str = r#"
/**
 * Query parameters for fetching identities' public keys for a contract.
 */
export interface IdentitiesContractKeysQuery {
  /**
   * Identity identifiers to fetch keys for.
   */
  identityIds: Array<IdentifierLike>;

  /**
   * Data contract identifier (reserved for future filtering).
   */
  contractId: IdentifierLike;

  /**
   * Optional list of purposes to include.
   * @default undefined
   */
  purposes?: number[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentitiesContractKeysQuery")]
    pub type IdentitiesContractKeysQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentitiesContractKeysQueryInput {
    #[serde(rename = "identityIds")]
    identity_ids: Vec<IdentifierWasm>,
    #[serde(rename = "contractId")]
    contract_id: IdentifierWasm,
    #[serde(default)]
    purposes: Option<Vec<u32>>,
}

struct IdentitiesContractKeysQueryParsed {
    identity_ids: Vec<Identifier>,
    contract_id: Identifier,
    purposes: Vec<Purpose>,
}

impl TryInto<IdentitiesContractKeysQuery> for IdentitiesContractKeysQueryParsed {
    type Error = WasmSdkError;

    fn try_into(self) -> Result<IdentitiesContractKeysQuery, Self::Error> {
        IdentitiesContractKeysQuery::new(self.identity_ids, self.contract_id, None, self.purposes)
            .map_err(|e| WasmSdkError::generic(format!("Failed to build query: {}", e)))
    }
}

fn parse_identities_contract_keys_query(
    query: IdentitiesContractKeysQueryJs,
) -> Result<IdentitiesContractKeysQueryParsed, WasmSdkError> {
    use dash_sdk::dpp::identity::Purpose;

    let input: IdentitiesContractKeysQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "identities contract keys query",
    )?;

    let purposes = match input.purposes {
        Some(values) => values
            .into_iter()
            .map(|p| {
                let byte: u8 = p.try_into().map_err(|_| {
                    WasmSdkError::invalid_argument(format!("Invalid purpose value: {}", p))
                })?;
                Purpose::try_from(byte).map_err(|e| {
                    WasmSdkError::invalid_argument(format!("Invalid purpose value {}: {}", p, e))
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => vec![
            Purpose::AUTHENTICATION,
            Purpose::ENCRYPTION,
            Purpose::DECRYPTION,
            Purpose::TRANSFER,
            Purpose::SYSTEM,
            Purpose::VOTING,
        ],
    };

    Ok(IdentitiesContractKeysQueryParsed {
        identity_ids: input
            .identity_ids
            .into_iter()
            .map(Identifier::from)
            .collect(),
        contract_id: input.contract_id.into(),
        purposes,
    })
}

#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_KEYS_QUERY_TS: &'static str = r#"
/**
 * Requested key selection strategy.
 */
export type IdentityKeysRequest =
  | {
      /**
       * Fetch all keys associated with the identity.
       */
      type: 'all';
    }
  | {
      /**
       * Fetch only the provided key identifiers.
       */
      type: 'specific';

      /**
       * Public key identifiers to return.
       */
      specificKeyIds: number[];
    }
  | {
      /**
       * Search keys by purpose and security level requirements.
       */
      type: 'search';

      /**
       * Purpose → security level selector map.
       */
      purposeMap: IdentityKeysPurposeMap;
    };

/**
 * Purpose to security level search map.
 */
export type IdentityKeysPurposeMap = {
  [purpose: number]: {
    [securityLevel: number]: IdentityKeysSearchKind;
  };
};

/**
 * Which keys should be returned for a purpose/security level pairing.
 */
export type IdentityKeysSearchKind = 'current' | 'all';

/**
 * Query parameters for fetching identity public keys.
 */
export interface IdentityKeysQuery {
  /**
   * Identity identifier.
   */
  identityId: IdentifierLike

  /**
   * Requested key selection strategy.
   */
  request: IdentityKeysRequest;

  /**
   * Maximum number of keys to return after applying request filters.
   * @default undefined (no additional limit)
   */
  limit?: number;

  /**
   * Number of keys to skip from the beginning of the result set.
   * @default undefined
   */
  offset?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityKeysQuery")]
    pub type IdentityKeysQueryJs;
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityKeysQueryInput {
    identity_id: IdentifierWasm,
    request: IdentityKeysRequestInput,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum IdentityKeysRequestInput {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "specific")]
    Specific {
        #[serde(rename = "specificKeyIds")]
        specific_key_ids: Vec<u32>,
    },
    #[serde(rename = "search")]
    Search {
        #[serde(rename = "purposeMap")]
        purpose_map: BTreeMap<u32, BTreeMap<u32, IdentityKeysSearchKind>>,
    },
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum IdentityKeysSearchKind {
    Current,
    All,
}

struct IdentityKeysQueryParsed {
    identity_id: Identifier,
    request: IdentityKeysRequestInput,
    limit: Option<u32>,
    offset: Option<u32>,
}

fn parse_identity_keys_query(
    query: IdentityKeysQueryJs,
) -> Result<IdentityKeysQueryParsed, WasmSdkError> {
    let input: IdentityKeysQueryInput =
        deserialize_required_query(query, "Query object is required", "identity keys query")?;

    let identity_id: Identifier = input.identity_id.into();

    Ok(IdentityKeysQueryParsed {
        identity_id,
        request: input.request,
        limit: input.limit,
        offset: input.offset,
    })
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getIdentity")]
    pub async fn get_identity(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<Option<IdentityWasm>, WasmSdkError> {
        let id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        let identity = Identity::fetch_by_identifier(self.as_ref(), id).await?;

        Ok(identity.map(IdentityWasm::from))
    }

    #[wasm_bindgen(
        js_name = "getIdentityWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Identity | undefined>"
    )]
    pub async fn get_identity_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        let (identity, metadata, proof) =
            Identity::fetch_with_metadata_and_proof(self.as_ref(), id, None).await?;

        let data: JsValue = match identity {
            Some(identity) => IdentityWasm::from(identity).into(),
            None => JsValue::NULL,
        };

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(js_name = "getIdentityUnproved")]
    pub async fn get_identity_unproved(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<IdentityWasm, WasmSdkError> {
        use dash_sdk::platform::proto::get_identity_request::{
            GetIdentityRequestV0, Version as GetIdentityRequestVersion,
        };
        use dash_sdk::platform::proto::get_identity_response::{
            get_identity_response_v0, GetIdentityResponseV0, Version,
        };
        use dash_sdk::platform::proto::{GetIdentityRequest, GetIdentityResponse};
        use rs_dapi_client::{DapiRequest, RequestSettings};

        let id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        let request = GetIdentityRequest {
            version: Some(GetIdentityRequestVersion::V0(GetIdentityRequestV0 {
                id: id.to_vec(),
                prove: false, // Request without proof
            })),
        };

        let response: GetIdentityResponse = request
            .execute(self.as_ref(), RequestSettings::default())
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to fetch identity: {}", e)))?
            .into_inner();

        match response.version {
            Some(Version::V0(GetIdentityResponseV0 {
                result: Some(get_identity_response_v0::Result::Identity(identity_bytes)),
                ..
            })) => {
                use dash_sdk::dpp::serialization::PlatformDeserializable;
                let identity = Identity::deserialize_from_bytes(identity_bytes.as_slice())
                    .map_err(|e| {
                        WasmSdkError::serialization(format!(
                            "Failed to deserialize identity: {}",
                            e
                        ))
                    })?;
                Ok(identity.into())
            }
            _ => Err(WasmSdkError::not_found("Identity not found")),
        }
    }

    #[wasm_bindgen(
        js_name = "getIdentityKeys",
        unchecked_return_type = "Array<IdentityPublicKey>"
    )]
    pub async fn get_identity_keys(
        &self,
        query: IdentityKeysQueryJs,
    ) -> Result<Array, WasmSdkError> {
        let IdentityKeysQueryParsed {
            identity_id,
            request,
            limit,
            offset,
        } = parse_identity_keys_query(query)?;

        let keys_result: IdentityPublicKeys = match request {
            IdentityKeysRequestInput::All => {
                IdentityPublicKey::fetch_many(self.as_ref(), identity_id).await?
            }
            IdentityKeysRequestInput::Specific { specific_key_ids } => {
                if specific_key_ids.is_empty() {
                    return Err(WasmSdkError::invalid_argument(
                        "specificKeyIds must contain at least one entry",
                    ));
                }

                let query = IdentityKeysQuery::new(identity_id, specific_key_ids)
                    .with_limit(limit.unwrap_or(100))
                    .with_offset(offset.unwrap_or(0));

                IdentityPublicKey::fetch_many(self.as_ref(), query).await?
            }
            IdentityKeysRequestInput::Search { purpose_map } => {
                use dash_sdk::platform::proto::{
                    get_identity_keys_request::{GetIdentityKeysRequestV0, Version},
                    key_request_type::Request,
                    security_level_map::KeyKindRequestType as GrpcKeyKindRequestType,
                    GetIdentityKeysRequest, KeyRequestType, SearchKey, SecurityLevelMap,
                };
                use rs_dapi_client::{DapiRequest, RequestSettings};

                let purpose_map = purpose_map
                    .into_iter()
                    .map(|(purpose, levels)| {
                        let security_level_map = levels
                            .into_iter()
                            .map(|(level, kind)| {
                                let kind_value = match kind {
                                    IdentityKeysSearchKind::Current => {
                                        GrpcKeyKindRequestType::CurrentKeyOfKindRequest as i32
                                    }
                                    IdentityKeysSearchKind::All => {
                                        GrpcKeyKindRequestType::AllKeysOfKindRequest as i32
                                    }
                                };
                                (level, kind_value)
                            })
                            .collect::<HashMap<_, _>>();

                        (purpose, SecurityLevelMap { security_level_map })
                    })
                    .collect::<HashMap<_, _>>();

                let request = GetIdentityKeysRequest {
                    version: Some(Version::V0(GetIdentityKeysRequestV0 {
                        identity_id: identity_id.to_vec(),
                        prove: false,
                        limit: Some(limit.unwrap_or(100)),
                        offset,
                        request_type: Some(KeyRequestType {
                            request: Some(Request::SearchKey(SearchKey { purpose_map })),
                        }),
                    })),
                };

                let response = request
                    .execute(self.as_ref(), RequestSettings::default())
                    .await
                    .map_err(|e| {
                        WasmSdkError::generic(format!(
                            "Failed to fetch search identity keys: {}",
                            e
                        ))
                    })?;

                use dash_sdk::platform::proto::{
                    get_identity_keys_response::Version as ResponseVersion, GetIdentityKeysResponse,
                };
                use rs_dapi_client::IntoInner;

                let response: GetIdentityKeysResponse = response.into_inner();
                match response.version {
                    Some(ResponseVersion::V0(response_v0)) => {
                        if let Some(result) = response_v0.result {
                            match result {
                                dash_sdk::platform::proto::get_identity_keys_response::get_identity_keys_response_v0::Result::Keys(
                                    keys_response,
                                ) => {
                                    let mut key_map: IdentityPublicKeys = IndexMap::new();
                                    for key_bytes in keys_response.keys_bytes {
                                        use dash_sdk::dpp::serialization::PlatformDeserializable;
                                        let key = dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey::deserialize_from_bytes(
                                                key_bytes.as_slice(),
                                            )
                                            .map_err(|e| WasmSdkError::serialization(
                                                format!("Failed to deserialize identity public key: {}", e),
                                            ))?;
                                        key_map.insert(key.id(), Some(key));
                                    }
                                    key_map
                                }
                                _ => {
                                    return Err(
                                        WasmSdkError::generic("Unexpected response format"),
                                    );
                                }
                            }
                        } else {
                            return Err(WasmSdkError::not_found("No keys found in response"));
                        }
                    }
                    _ => return Err(WasmSdkError::generic("Unexpected response version")),
                }
            }
        };

        let array = Array::new();
        for (_key_id, key_opt) in keys_result {
            if let Some(key) = key_opt {
                array.push(&IdentityPublicKeyWasm::from(key).into());
            }
        }

        Ok(array)
    }

    #[wasm_bindgen(js_name = "getIdentityNonce")]
    pub async fn get_identity_nonce(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<Option<BigInt>, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityNonceFetcher;

        let id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        let nonce_result = IdentityNonceFetcher::fetch(self.as_ref(), id).await?;

        Ok(nonce_result.map(|fetcher| BigInt::from(fetcher.0)))
    }

    #[wasm_bindgen(
        js_name = "getIdentityNonceWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<bigint | undefined>"
    )]
    pub async fn get_identity_nonce_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityNonceFetcher;

        let id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        let (nonce_result, metadata, proof) =
            IdentityNonceFetcher::fetch_with_metadata_and_proof(self.as_ref(), id, None).await?;

        let data: JsValue = match nonce_result {
            Some(fetcher) => BigInt::from(fetcher.0).into(),
            None => JsValue::NULL,
        };

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(js_name = "getIdentityContractNonce")]
    pub async fn get_identity_contract_nonce(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> Result<Option<BigInt>, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityContractNonceFetcher;

        let identity_id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;
        let contract_id: Identifier = contract_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", err))
        })?;

        let nonce_result =
            IdentityContractNonceFetcher::fetch(self.as_ref(), (identity_id, contract_id)).await?;

        Ok(nonce_result.map(|fetcher| BigInt::from(fetcher.0)))
    }

    #[wasm_bindgen(
        js_name = "getIdentityContractNonceWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<bigint | undefined>"
    )]
    pub async fn get_identity_contract_nonce_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityContractNonceFetcher;

        let identity_id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;
        let contract_id: Identifier = contract_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", err))
        })?;

        let (nonce_result, metadata, proof) =
            IdentityContractNonceFetcher::fetch_with_metadata_and_proof(
                self.as_ref(),
                (identity_id, contract_id),
                None,
            )
            .await?;

        let data: JsValue = match nonce_result {
            Some(fetcher) => BigInt::from(fetcher.0).into(),
            None => JsValue::NULL,
        };

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(js_name = "getIdentityBalance")]
    pub async fn get_identity_balance(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<Option<BigInt>, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityBalance;

        let identity_id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        let balance_result = IdentityBalance::fetch(self.as_ref(), identity_id).await?;

        Ok(balance_result.map(BigInt::from))
    }

    #[wasm_bindgen(
        js_name = "getIdentitiesBalances",
        unchecked_return_type = "Map<string, bigint | undefined>"
    )]
    pub async fn get_identities_balances(
        &self,
        #[wasm_bindgen(js_name = "identityIds")] identity_ids: IdentifierLikeArrayJs,
    ) -> Result<Map, WasmSdkError> {
        use drive_proof_verifier::types::IdentityBalance;
        use wasm_dpp2::utils::try_to_vec;

        // Convert JS identifiers to native Identifiers
        let identifiers: Vec<Identifier> =
            try_to_vec::<IdentifierWasm, _, _>(identity_ids, "identityIds", "identifier")?;

        let balances_result: drive_proof_verifier::types::IdentityBalances =
            IdentityBalance::fetch_many(self.as_ref(), identifiers.clone()).await?;

        let results_map = Map::new();

        for identifier in identifiers {
            let key: JsValue = IdentifierWasm::from(identifier).to_base58().into();
            let value = match balances_result.get(&identifier) {
                Some(Some(balance)) => BigInt::from(*balance).into(),
                _ => JsValue::NULL,
            };
            results_map.set(&key, &value);
        }

        Ok(results_map)
    }

    #[wasm_bindgen(js_name = "getIdentityBalanceAndRevision")]
    pub async fn get_identity_balance_and_revision(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<Option<IdentityBalanceAndRevisionWasm>, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityBalanceAndRevision;

        let id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        let result = IdentityBalanceAndRevision::fetch(self.as_ref(), id).await?;

        Ok(
            result
                .map(|(balance, revision)| IdentityBalanceAndRevisionWasm::new(balance, revision)),
        )
    }

    #[wasm_bindgen(js_name = "getIdentityByPublicKeyHash")]
    pub async fn get_identity_by_public_key_hash(
        &self,
        #[wasm_bindgen(js_name = "publicKeyHash")] public_key_hash: PublicKeyHashLikeJs,
    ) -> Result<Option<IdentityWasm>, WasmSdkError> {
        use dash_sdk::platform::types::identity::PublicKeyHash;
        let hash_bytes: Vec<u8> = public_key_hash_from_js(public_key_hash)?;
        if hash_bytes.len() != 20 {
            return Err(WasmSdkError::invalid_argument(
                "Public key hash must be 20 bytes (40 hex characters)",
            ));
        }

        let mut hash_array = [0u8; 20];
        hash_array.copy_from_slice(&hash_bytes);

        let result = Identity::fetch(self.as_ref(), PublicKeyHash(hash_array)).await?;

        Ok(result.map(IdentityWasm::from))
    }

    #[wasm_bindgen(
        js_name = "getIdentitiesContractKeys",
        unchecked_return_type = "Array<IdentityContractKeys>"
    )]
    pub async fn get_identities_contract_keys(
        &self,
        query: IdentitiesContractKeysQueryJs,
    ) -> Result<Array, WasmSdkError> {
        use dash_sdk::platform::Fetch;

        let params = parse_identities_contract_keys_query(query)?;

        let query: IdentitiesContractKeysQuery = params.try_into()?;

        let keys_result: Option<IdentitiesContractKeys> =
            IdentitiesContractKeys::fetch(self.as_ref(), query).await?;

        let array = Array::new();
        if let Some(keys_map) = keys_result {
            for (identity_id, purposes_map) in keys_map {
                let identity_keys: Vec<IdentityPublicKeyWasm> = purposes_map
                    .into_iter()
                    .filter_map(|(_, key_opt)| key_opt.map(IdentityPublicKeyWasm::from))
                    .collect();

                if !identity_keys.is_empty() {
                    let response = IdentityContractKeysWasm::new(
                        IdentifierWasm::from(identity_id),
                        identity_keys,
                    );
                    array.push(&response.into());
                }
            }
        }

        Ok(array)
    }

    #[wasm_bindgen(
        js_name = "getIdentityByNonUniquePublicKeyHash",
        unchecked_return_type = "Array<Identity>"
    )]
    pub async fn get_identity_by_non_unique_public_key_hash(
        &self,
        #[wasm_bindgen(js_name = "publicKeyHash")] public_key_hash: PublicKeyHashLikeJs,
        #[wasm_bindgen(js_name = "startAfterId")] start_after_id: IdentifierLikeOrUndefinedJs,
    ) -> Result<Array, WasmSdkError> {
        let hash_bytes: Vec<u8> = public_key_hash_from_js(public_key_hash)?;
        if hash_bytes.len() != 20 {
            return Err(WasmSdkError::invalid_argument(
                "Public key hash must be 20 bytes (40 hex characters)",
            ));
        }

        let mut hash_array = [0u8; 20];
        hash_array.copy_from_slice(&hash_bytes);

        // Convert start_after if provided
        let start_id: Option<Identifier> = start_after_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid startAfter identity ID: {}", err))
        })?;

        use dash_sdk::platform::types::identity::NonUniquePublicKeyHashQuery;

        let query = NonUniquePublicKeyHashQuery {
            key_hash: hash_array,
            after: start_id.map(|id| *id.as_bytes()),
        };

        let identity = Identity::fetch(self.as_ref(), query).await?;

        let js_array = Array::new();
        if let Some(identity) = identity {
            js_array.push(&JsValue::from(IdentityWasm::from(identity)));
        }
        Ok(js_array)
    }

    #[wasm_bindgen(
        js_name = "getIdentityTokenBalances",
        unchecked_return_type = "Map<string, bigint>"
    )]
    pub async fn get_identity_token_balances(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "tokenIds")] token_ids: IdentifierLikeArrayJs,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::dpp::balances::credits::TokenAmount;
        use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;
        use wasm_dpp2::utils::try_to_vec;

        let identity_id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        // Convert token IDs to Identifiers
        let token_identifiers: Vec<Identifier> =
            try_to_vec::<IdentifierWasm, _, _>(token_ids, "tokenIds", "identifier")?;

        let query = IdentityTokenBalancesQuery {
            identity_id,
            token_ids: token_identifiers.clone(),
        };

        // Use FetchMany trait to fetch token balances
        let balances: drive_proof_verifier::types::identity_token_balance::IdentityTokenBalances =
            TokenAmount::fetch_many(self.as_ref(), query).await?;

        let balances_map = Map::new();
        for token_id in token_identifiers {
            if let Some(Some(balance)) = balances.get(&token_id) {
                let key: JsValue = IdentifierWasm::from(token_id).to_base58().into();
                let value = JsValue::from(BigInt::from(*balance));
                balances_map.set(&key, &value);
            }
        }

        Ok(balances_map)
    }

    // Proof info versions for identity queries

    #[wasm_bindgen(
        js_name = "getIdentityKeysWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Array<IdentityPublicKey>>"
    )]
    pub async fn get_identity_keys_with_proof_info(
        &self,
        query: IdentityKeysQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let IdentityKeysQueryParsed {
            identity_id,
            request,
            limit,
            offset,
        } = parse_identity_keys_query(query)?;

        let (keys_result, metadata, proof) = match request {
            IdentityKeysRequestInput::All => {
                IdentityPublicKey::fetch_many_with_metadata_and_proof(
                    self.as_ref(),
                    identity_id,
                    None,
                )
                .await?
            }
            IdentityKeysRequestInput::Specific { specific_key_ids } => {
                use dash_sdk::platform::FetchMany;

                if specific_key_ids.is_empty() {
                    return Err(WasmSdkError::invalid_argument(
                        "specificKeyIds must contain at least one entry",
                    ));
                }

                let query = IdentityKeysQuery::new(identity_id, specific_key_ids)
                    .with_limit(limit.unwrap_or(100))
                    .with_offset(offset.unwrap_or(0));

                IdentityPublicKey::fetch_many_with_metadata_and_proof(self.as_ref(), query, None)
                    .await?
            }
            IdentityKeysRequestInput::Search { .. } => {
                return Err(WasmSdkError::invalid_argument(
                    "Search key requests are not supported with proof",
                ));
            }
        };

        let keys_array = Array::new();
        for (_key_id, key_opt) in keys_result {
            if let Some(key) = key_opt {
                keys_array.push(&IdentityPublicKeyWasm::from(key).into());
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            keys_array, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getIdentityBalanceWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<bigint | undefined>"
    )]
    pub async fn get_identity_balance_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityBalance;

        let identity_id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        let (balance_result, metadata, proof) =
            IdentityBalance::fetch_with_metadata_and_proof(self.as_ref(), identity_id, None)
                .await?;

        let data: JsValue = match balance_result {
            Some(balance) => BigInt::from(balance).into(),
            None => JsValue::NULL,
        };

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getIdentitiesBalancesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, bigint | undefined>>"
    )]
    pub async fn get_identities_balances_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityIds")] identity_ids: IdentifierLikeArrayJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use drive_proof_verifier::types::IdentityBalance;
        use wasm_dpp2::utils::try_to_vec;

        // Convert JS identifiers to native Identifiers
        let identifiers: Vec<Identifier> =
            try_to_vec::<IdentifierWasm, _, _>(identity_ids, "identityIds", "identifier")?;

        let (balances_result, metadata, proof): (
            drive_proof_verifier::types::IdentityBalances,
            _,
            _,
        ) = IdentityBalance::fetch_many_with_metadata_and_proof(
            self.as_ref(),
            identifiers.clone(),
            None,
        )
        .await?;

        let balances_map = Map::new();
        for identifier in identifiers {
            let key: JsValue = IdentifierWasm::from(identifier).to_base58().into();
            let value = match balances_result.get(&identifier) {
                Some(Some(balance)) => BigInt::from(*balance).into(),
                _ => JsValue::NULL,
            };
            balances_map.set(&key, &value);
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            balances_map,
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getIdentityBalanceAndRevisionWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<IdentityBalanceAndRevision | undefined>"
    )]
    pub async fn get_identity_balance_and_revision_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityBalanceAndRevision;

        let id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        let (result, metadata, proof) =
            IdentityBalanceAndRevision::fetch_with_metadata_and_proof(self.as_ref(), id, None)
                .await?;

        let data: JsValue = match result {
            Some((balance, revision)) => {
                IdentityBalanceAndRevisionWasm::new(balance, revision).into()
            }
            None => JsValue::NULL,
        };

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getIdentityByPublicKeyHashWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Identity | undefined>"
    )]
    pub async fn get_identity_by_public_key_hash_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "publicKeyHash")] public_key_hash: PublicKeyHashLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::types::identity::PublicKeyHash;
        let hash_bytes: Vec<u8> = public_key_hash_from_js(public_key_hash)?;
        if hash_bytes.len() != 20 {
            return Err(WasmSdkError::invalid_argument(
                "Public key hash must be 20 bytes (40 hex characters)",
            ));
        }

        let mut hash_array = [0u8; 20];
        hash_array.copy_from_slice(&hash_bytes);

        let (result, metadata, proof) =
            Identity::fetch_with_metadata_and_proof(self.as_ref(), PublicKeyHash(hash_array), None)
                .await?;

        let data: JsValue = match result {
            Some(identity) => IdentityWasm::from(identity).into(),
            None => JsValue::NULL,
        };

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getIdentityByNonUniquePublicKeyHashWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Array<Identity>>"
    )]
    pub async fn get_identity_by_non_unique_public_key_hash_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "publicKeyHash")] public_key_hash: PublicKeyHashLikeJs,
        #[wasm_bindgen(js_name = "startAfterId")] start_after_id: IdentifierLikeOrUndefinedJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let hash_bytes: Vec<u8> = public_key_hash_from_js(public_key_hash)?;
        if hash_bytes.len() != 20 {
            return Err(WasmSdkError::invalid_argument(
                "Public key hash must be 20 bytes (40 hex characters)",
            ));
        }

        let mut hash_array = [0u8; 20];
        hash_array.copy_from_slice(&hash_bytes);

        // Convert start_after if provided
        let start_id: Option<Identifier> = start_after_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid startAfter identity ID: {}", err))
        })?;

        use dash_sdk::platform::types::identity::NonUniquePublicKeyHashQuery;

        let query = NonUniquePublicKeyHashQuery {
            key_hash: hash_array,
            after: start_id.map(|id| *id.as_bytes()),
        };

        // Fetch identity by non-unique public key hash with proof
        let (identity, metadata, proof) =
            Identity::fetch_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let identities_array = Array::new();
        if let Some(identity) = identity {
            identities_array.push(&JsValue::from(IdentityWasm::from(identity)));
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            identities_array,
            metadata,
            proof,
        ))
    }

    // TODO: This method returns proof only for first identity
    #[wasm_bindgen(
        js_name = "getIdentitiesContractKeysWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Array<IdentityContractKeys>>"
    )]
    pub async fn get_identities_contract_keys_with_proof_info(
        &self,
        query: IdentitiesContractKeysQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        let params = parse_identities_contract_keys_query(query)?;
        let query: IdentitiesContractKeysQuery = params.try_into()?;

        let (keys_result, metadata, proof): (Option<IdentitiesContractKeys>, _, _) =
            IdentitiesContractKeys::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        let responses_array = Array::new();
        if let Some(keys_map) = keys_result {
            for (identity_id, purposes_map) in keys_map {
                let identity_keys: Vec<IdentityPublicKeyWasm> = purposes_map
                    .into_iter()
                    .filter_map(|(_, key_opt)| key_opt.map(IdentityPublicKeyWasm::from))
                    .collect();

                if !identity_keys.is_empty() {
                    let response = IdentityContractKeysWasm::new(
                        IdentifierWasm::from(identity_id),
                        identity_keys,
                    );
                    responses_array.push(&response.into());
                }
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            responses_array,
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getIdentityTokenBalancesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, bigint>>"
    )]
    pub async fn get_identity_token_balances_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "tokenIds")] token_ids: IdentifierLikeArrayJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::dpp::balances::credits::TokenAmount;
        use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;
        use wasm_dpp2::utils::try_to_vec;

        let identity_id: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        // Convert token IDs to Identifiers
        let token_identifiers: Vec<Identifier> =
            try_to_vec::<IdentifierWasm, _, _>(token_ids, "tokenIds", "identifier")?;

        let query = IdentityTokenBalancesQuery {
            identity_id,
            token_ids: token_identifiers.clone(),
        };

        // Use FetchMany trait to fetch token balances with proof
        let (balances, metadata, proof): (
            dash_sdk::query_types::identity_token_balance::IdentityTokenBalances,
            _,
            _,
        ) = TokenAmount::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let balances_map = Map::new();
        for token_id in token_identifiers {
            if let Some(Some(balance)) = balances.get(&token_id) {
                let key: JsValue = IdentifierWasm::from(token_id).to_base58().into();
                let value = JsValue::from(BigInt::from(*balance));
                balances_map.set(&key, &value);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            JsValue::from(balances_map),
            metadata,
            proof,
        ))
    }
}

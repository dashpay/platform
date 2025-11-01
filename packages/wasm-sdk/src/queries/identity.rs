use crate::error::WasmSdkError;
use crate::queries::utils::{deserialize_required_query, identifier_from_base58};
use crate::queries::{ProofInfoWasm, ProofMetadataResponseWasm, ResponseMetadataWasm};
use crate::sdk::WasmSdk;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey;
use dash_sdk::platform::{Fetch, FetchMany, Identifier, Identity, IdentityKeysQuery};
use drive_proof_verifier::types::{IdentityPublicKeys, IndexMap};
use js_sys::{Array, BigInt, Map};
use rs_dapi_client::IntoInner;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::identity::IdentityWasm;

#[wasm_bindgen(js_name = "IdentityKeyInfo")]
#[derive(Clone)]
pub struct IdentityKeyInfoWasm {
    key_id: u32,
    key_type: String,
    public_key_data: String,
    purpose: String,
    security_level: String,
    read_only: bool,
    disabled: bool,
}

impl IdentityKeyInfoWasm {
    fn from_entry(key_id: u32, key: &IdentityPublicKey) -> Self {
        IdentityKeyInfoWasm {
            key_id,
            key_type: format!("{:?}", key.key_type()),
            public_key_data: hex::encode(key.data().as_slice()),
            purpose: format!("{:?}", key.purpose()),
            security_level: format!("{:?}", key.security_level()),
            read_only: key.read_only(),
            disabled: key.disabled_at().is_some(),
        }
    }
}

#[wasm_bindgen(js_class = IdentityKeyInfo)]
impl IdentityKeyInfoWasm {
    #[wasm_bindgen(getter = "keyId")]
    pub fn key_id(&self) -> u32 {
        self.key_id
    }

    #[wasm_bindgen(getter = "keyType")]
    pub fn key_type(&self) -> String {
        self.key_type.clone()
    }

    #[wasm_bindgen(getter = "publicKeyData")]
    pub fn public_key_data(&self) -> String {
        self.public_key_data.clone()
    }

    #[wasm_bindgen(getter = "purpose")]
    pub fn purpose(&self) -> String {
        self.purpose.clone()
    }

    #[wasm_bindgen(getter = "securityLevel")]
    pub fn security_level(&self) -> String {
        self.security_level.clone()
    }

    #[wasm_bindgen(getter = "readOnly")]
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    #[wasm_bindgen(getter = "disabled")]
    pub fn disabled(&self) -> bool {
        self.disabled
    }
}

#[wasm_bindgen(js_name = "IdentityContractKeys")]
#[derive(Clone)]
pub struct IdentityContractKeysWasm {
    identity_id: String,
    keys: Vec<IdentityKeyInfoWasm>,
}

impl IdentityContractKeysWasm {
    fn new(identity_id: String, keys: Vec<IdentityKeyInfoWasm>) -> Self {
        IdentityContractKeysWasm { identity_id, keys }
    }
}

#[wasm_bindgen(js_class = IdentityContractKeys)]
impl IdentityContractKeysWasm {
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> String {
        self.identity_id.clone()
    }

    #[wasm_bindgen(getter = "keys")]
    pub fn keys(&self) -> Array {
        let array = Array::new();
        for key in &self.keys {
            array.push(&JsValue::from(key.clone()));
        }
        array
    }
}

#[wasm_bindgen(js_name = "IdentityBalanceInfo")]
#[derive(Clone)]
pub struct IdentityBalanceWasm {
    balance: u64,
}

impl IdentityBalanceWasm {
    fn new(balance: u64) -> Self {
        IdentityBalanceWasm { balance }
    }
}

#[wasm_bindgen(js_class = IdentityBalanceInfo)]
impl IdentityBalanceWasm {
    #[wasm_bindgen(getter = "balance")]
    pub fn balance(&self) -> BigInt {
        BigInt::from(self.balance)
    }
}

#[wasm_bindgen(js_name = "IdentityBalanceEntry")]
#[derive(Clone)]
pub struct IdentityBalanceEntryWasm {
    identity_id: String,
    balance: u64,
}

impl IdentityBalanceEntryWasm {
    fn new(identity_id: String, balance: u64) -> Self {
        IdentityBalanceEntryWasm {
            identity_id,
            balance,
        }
    }
}

#[wasm_bindgen(js_class = IdentityBalanceEntry)]
impl IdentityBalanceEntryWasm {
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> String {
        self.identity_id.clone()
    }

    #[wasm_bindgen(getter = "balance")]
    pub fn balance(&self) -> BigInt {
        BigInt::from(self.balance)
    }
}

#[wasm_bindgen(js_name = "IdentityBalanceAndRevision")]
#[derive(Clone)]
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

#[wasm_bindgen(js_name = "IdentityNonce")]
#[derive(Clone)]
pub struct IdentityNonceWasm {
    nonce: u64,
}

impl IdentityNonceWasm {
    fn new(nonce: u64) -> Self {
        IdentityNonceWasm { nonce }
    }
}

#[wasm_bindgen(js_class = IdentityNonce)]
impl IdentityNonceWasm {
    #[wasm_bindgen(getter = "nonce")]
    pub fn nonce(&self) -> BigInt {
        BigInt::from(self.nonce)
    }
}

#[wasm_bindgen(js_name = "IdentityProofResponse")]
#[derive(Clone)]
pub struct IdentityProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub identity: Option<IdentityWasm>,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

#[wasm_bindgen(js_name = "IdentityKeysProofResponse")]
#[derive(Clone)]
pub struct IdentityKeysProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub keys: Array,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
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
   * Identity identifier (base58 string).
   */
  identityId: string;

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
    identity_id: String,
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

    let identity_id = identifier_from_base58(&input.identity_id, "identity ID")?;

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
        base58_id: &str,
    ) -> Result<Option<IdentityWasm>, WasmSdkError> {
        let id = Identifier::from_string(
            base58_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let identity = Identity::fetch_by_identifier(self.as_ref(), id).await?;

        Ok(identity.map(IdentityWasm::from))
    }

    #[wasm_bindgen(js_name = "getIdentityWithProofInfo")]
    pub async fn get_identity_with_proof_info(
        &self,
        base58_id: &str,
    ) -> Result<IdentityProofResponseWasm, WasmSdkError> {
        let id = Identifier::from_string(
            base58_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let (identity, metadata, proof) =
            Identity::fetch_with_metadata_and_proof(self.as_ref(), id, None).await?;

        match identity {
            Some(identity) => Ok(IdentityProofResponseWasm {
                identity: Some(IdentityWasm::from(identity)),
                metadata: metadata.into(),
                proof: proof.into(),
            }),
            None => Err(WasmSdkError::not_found("Identity not found")),
        }
    }

    #[wasm_bindgen(js_name = "getIdentityUnproved")]
    pub async fn get_identity_unproved(
        &self,
        base58_id: &str,
    ) -> Result<IdentityWasm, WasmSdkError> {
        use dash_sdk::platform::proto::get_identity_request::{
            GetIdentityRequestV0, Version as GetIdentityRequestVersion,
        };
        use dash_sdk::platform::proto::get_identity_response::{
            get_identity_response_v0, GetIdentityResponseV0, Version,
        };
        use dash_sdk::platform::proto::{GetIdentityRequest, GetIdentityResponse};
        use rs_dapi_client::{DapiRequest, RequestSettings};

        let id = Identifier::from_string(
            base58_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

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

    #[wasm_bindgen(js_name = "getIdentityKeys")]
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
                IdentityPublicKey::fetch_many(self.as_ref(), identity_id.clone()).await?
            }
            IdentityKeysRequestInput::Specific { specific_key_ids } => {
                if specific_key_ids.is_empty() {
                    return Err(WasmSdkError::invalid_argument(
                        "specificKeyIds must contain at least one entry",
                    ));
                }

                let request_limit = limit.unwrap_or(100);

                let query = IdentityKeysQuery::new(identity_id.clone(), specific_key_ids)
                    .with_limit(request_limit);

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
                        offset: None,
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
                                dash_sdk::platform::proto::get_identity_keys_response::get_identity_keys_response_v0::Result::Keys(keys_response) => {
                                    let mut key_map: IdentityPublicKeys = IndexMap::new();
                                    for key_bytes in keys_response.keys_bytes {
                                        use dash_sdk::dpp::serialization::PlatformDeserializable;
                                        let key = dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey::deserialize_from_bytes(key_bytes.as_slice())
                                            .map_err(|e| WasmSdkError::serialization(format!("Failed to deserialize identity public key: {}", e)))?;
                                        key_map.insert(key.id(), Some(key));
                                    }
                                    key_map
                                }
                                _ => return Err(WasmSdkError::generic("Unexpected response format")),
                            }
                        } else {
                            return Err(WasmSdkError::not_found("No keys found in response"));
                        }
                    }
                    _ => return Err(WasmSdkError::generic("Unexpected response version")),
                }
            }
        };

        let mut keys: Vec<IdentityKeyInfoWasm> = Vec::new();

        let start = offset.unwrap_or(0) as usize;
        let end = if let Some(lim) = limit {
            start + lim as usize
        } else {
            usize::MAX
        };

        for (idx, (key_id, key_opt)) in keys_result.into_iter().enumerate() {
            if idx < start {
                continue;
            }
            if idx >= end {
                break;
            }

            if let Some(key) = key_opt {
                keys.push(IdentityKeyInfoWasm::from_entry(key_id, &key));
            }
        }

        let array = Array::new();
        for key in keys {
            array.push(&JsValue::from(key));
        }

        Ok(array)
    }

    #[wasm_bindgen(js_name = "getIdentityNonce")]
    pub async fn get_identity_nonce(
        &self,
        identity_id: &str,
    ) -> Result<IdentityNonceWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityNonceFetcher;

        if identity_id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Identity ID is required"));
        }

        let id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let nonce_result = IdentityNonceFetcher::fetch(self.as_ref(), id).await?;

        let nonce = nonce_result
            .map(|fetcher| fetcher.0)
            .ok_or_else(|| WasmSdkError::not_found("Identity nonce not found"))?;

        Ok(IdentityNonceWasm::new(nonce))
    }

    #[wasm_bindgen(js_name = "getIdentityNonceWithProofInfo")]
    pub async fn get_identity_nonce_with_proof_info(
        &self,
        identity_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityNonceFetcher;

        if identity_id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Identity ID is required"));
        }

        let id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let (nonce_result, metadata, proof) =
            IdentityNonceFetcher::fetch_with_metadata_and_proof(self.as_ref(), id, None).await?;

        let nonce = nonce_result
            .map(|fetcher| fetcher.0)
            .ok_or_else(|| WasmSdkError::not_found("Identity nonce not found"))?;

        let data = IdentityNonceWasm::new(nonce);
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            JsValue::from(data),
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(js_name = "getIdentityContractNonce")]
    pub async fn get_identity_contract_nonce(
        &self,
        identity_id: &str,
        contract_id: &str,
    ) -> Result<IdentityNonceWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityContractNonceFetcher;

        if identity_id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Identity ID is required"));
        }

        if contract_id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Contract ID is required"));
        }

        let identity_id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let contract_id = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        let nonce_result =
            IdentityContractNonceFetcher::fetch(self.as_ref(), (identity_id, contract_id)).await?;

        let nonce = nonce_result
            .map(|fetcher| fetcher.0)
            .ok_or_else(|| WasmSdkError::not_found("Identity contract nonce not found"))?;

        Ok(IdentityNonceWasm::new(nonce))
    }

    #[wasm_bindgen(js_name = "getIdentityContractNonceWithProofInfo")]
    pub async fn get_identity_contract_nonce_with_proof_info(
        &self,
        identity_id: &str,
        contract_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityContractNonceFetcher;

        if identity_id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Identity ID is required"));
        }

        if contract_id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Contract ID is required"));
        }

        let identity_id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let contract_id = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        let (nonce_result, metadata, proof) =
            IdentityContractNonceFetcher::fetch_with_metadata_and_proof(
                self.as_ref(),
                (identity_id, contract_id),
                None,
            )
            .await?;

        let nonce = nonce_result
            .map(|fetcher| fetcher.0)
            .ok_or_else(|| WasmSdkError::not_found("Identity contract nonce not found"))?;

        let data = IdentityNonceWasm::new(nonce);
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            JsValue::from(data),
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(js_name = "getIdentityBalance")]
    pub async fn get_identity_balance(
        &self,
        id: &str,
    ) -> Result<IdentityBalanceWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityBalance;

        if id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Identity ID is required"));
        }

        let identity_id = Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let balance_result = IdentityBalance::fetch(self.as_ref(), identity_id).await?;

        balance_result
            .map(IdentityBalanceWasm::new)
            .ok_or_else(|| WasmSdkError::not_found("Identity balance not found"))
    }

    #[wasm_bindgen(js_name = "getIdentitiesBalances")]
    pub async fn get_identities_balances(
        &self,
        identity_ids: Vec<String>,
    ) -> Result<Array, WasmSdkError> {
        use drive_proof_verifier::types::IdentityBalance;

        // Convert string IDs to Identifiers
        let identifiers: Vec<Identifier> = identity_ids
            .into_iter()
            .map(|id| {
                Identifier::from_string(
                    &id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let balances_result: drive_proof_verifier::types::IdentityBalances =
            IdentityBalance::fetch_many(self.as_ref(), identifiers.clone()).await?;

        let results_array = Array::new();

        for id in identifiers {
            if let Some(Some(balance)) = balances_result.get(&id) {
                let identity_id =
                    id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58);
                results_array.push(&JsValue::from(IdentityBalanceEntryWasm::new(
                    identity_id,
                    *balance,
                )));
            }
        }

        Ok(results_array)
    }

    #[wasm_bindgen(js_name = "getIdentityBalanceAndRevision")]
    pub async fn get_identity_balance_and_revision(
        &self,
        identity_id: &str,
    ) -> Result<IdentityBalanceAndRevisionWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityBalanceAndRevision;

        if identity_id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Identity ID is required"));
        }

        let id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let result = IdentityBalanceAndRevision::fetch(self.as_ref(), id).await?;

        result
            .map(|(balance, revision)| IdentityBalanceAndRevisionWasm::new(balance, revision))
            .ok_or_else(|| WasmSdkError::not_found("Identity balance and revision not found"))
    }

    #[wasm_bindgen(js_name = "getIdentityByPublicKeyHash")]
    pub async fn get_identity_by_public_key_hash(
        &self,
        public_key_hash: &str,
    ) -> Result<IdentityWasm, WasmSdkError> {
        use dash_sdk::platform::types::identity::PublicKeyHash;

        // Parse the hex-encoded public key hash
        let hash_bytes = hex::decode(public_key_hash).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid public key hash hex: {}", e))
        })?;

        if hash_bytes.len() != 20 {
            return Err(WasmSdkError::invalid_argument(
                "Public key hash must be 20 bytes (40 hex characters)",
            ));
        }

        let mut hash_array = [0u8; 20];
        hash_array.copy_from_slice(&hash_bytes);

        let result = Identity::fetch(self.as_ref(), PublicKeyHash(hash_array)).await?;

        result
            .ok_or_else(|| WasmSdkError::not_found("Identity not found for public key hash"))
            .map(Into::into)
    }

    #[wasm_bindgen(js_name = "getIdentitiesContractKeys")]
    pub async fn get_identities_contract_keys(
        &self,
        identities_ids: Vec<String>,
        contract_id: &str,
        purposes: Option<Vec<u32>>,
    ) -> Result<Array, WasmSdkError> {
        use dash_sdk::dpp::identity::Purpose;

        // Convert string IDs to Identifiers
        let _identity_ids: Vec<Identifier> = identities_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Contract ID is not used in the individual key queries, but we validate it
        let _contract_identifier = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Convert purposes if provided
        let purposes_opt = purposes.map(|p| {
            p.into_iter()
                .filter_map(|purpose_int| match purpose_int {
                    0 => Some(Purpose::AUTHENTICATION as u32),
                    1 => Some(Purpose::ENCRYPTION as u32),
                    2 => Some(Purpose::DECRYPTION as u32),
                    3 => Some(Purpose::TRANSFER as u32),
                    4 => Some(Purpose::SYSTEM as u32),
                    5 => Some(Purpose::VOTING as u32),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });

        // For now, we'll implement this by fetching keys for each identity individually
        // The SDK doesn't fully expose the batch query yet
        let mut responses: Vec<IdentityContractKeysWasm> = Vec::new();

        for identity_id_str in identities_ids {
            let identity_id = Identifier::from_string(
                &identity_id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| {
                WasmSdkError::invalid_argument(format!(
                    "Invalid identity ID '{}': {}",
                    identity_id_str, e
                ))
            })?;

            // Get keys for this identity using the regular identity keys query
            let keys_result = IdentityPublicKey::fetch_many(self.as_ref(), identity_id).await?;

            let mut identity_keys = Vec::new();

            // Filter keys by purpose if specified
            for (key_id, key_opt) in keys_result {
                if let Some(key) = key_opt {
                    // Check if this key matches the requested purposes
                    if let Some(ref purposes) = purposes_opt {
                        if !purposes.contains(&(key.purpose() as u32)) {
                            continue;
                        }
                    }

                    identity_keys.push(IdentityKeyInfoWasm::from_entry(key_id, &key));
                }
            }

            if !identity_keys.is_empty() {
                responses.push(IdentityContractKeysWasm::new(
                    identity_id_str,
                    identity_keys,
                ));
            }
        }

        let array = Array::new();
        for response in responses {
            array.push(&JsValue::from(response));
        }

        Ok(array)
    }

    #[wasm_bindgen(js_name = "getIdentityByNonUniquePublicKeyHash")]
    pub async fn get_identity_by_non_unique_public_key_hash(
        &self,
        public_key_hash: &str,
        start_after: Option<String>,
    ) -> Result<Array, WasmSdkError> {
        // Parse the hex-encoded public key hash
        let hash_bytes = hex::decode(public_key_hash).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid public key hash hex: {}", e))
        })?;

        if hash_bytes.len() != 20 {
            return Err(WasmSdkError::invalid_argument(
                "Public key hash must be 20 bytes (40 hex characters)",
            ));
        }

        let mut hash_array = [0u8; 20];
        hash_array.copy_from_slice(&hash_bytes);

        // Convert start_after if provided
        let start_id = if let Some(start) = start_after {
            Some(
                Identifier::from_string(
                    &start,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
                .map_err(|e| {
                    WasmSdkError::invalid_argument(format!(
                        "Invalid start_after identity ID: {}",
                        e
                    ))
                })?,
            )
        } else {
            None
        };

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

    #[wasm_bindgen(js_name = "getIdentityTokenBalances")]
    pub async fn get_identity_token_balances(
        &self,
        identity_id: &str,
        token_ids: Vec<String>,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::dpp::balances::credits::TokenAmount;
        use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;

        let identity_id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Convert token IDs to Identifiers
        let token_identifiers: Vec<Identifier> = token_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

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
                let key = JsValue::from(IdentifierWasm::from(token_id));
                let value = JsValue::from(BigInt::from(*balance));
                balances_map.set(&key, &value);
            }
        }

        Ok(balances_map)
    }

    // Proof info versions for identity queries

    #[wasm_bindgen(js_name = "getIdentityKeysWithProofInfo")]
    pub async fn get_identity_keys_with_proof_info(
        &self,
        query: IdentityKeysQueryJs,
    ) -> Result<IdentityKeysProofResponseWasm, WasmSdkError> {
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
                    identity_id.clone(),
                    None,
                )
                .await?
            }
            IdentityKeysRequestInput::Specific { specific_key_ids } => {
                use dash_sdk::platform::proto::{
                    get_identity_keys_request::{GetIdentityKeysRequestV0, Version},
                    key_request_type::Request,
                    GetIdentityKeysRequest, KeyRequestType, SpecificKeys,
                };
                use rs_dapi_client::{DapiRequest, RequestSettings};

                if specific_key_ids.is_empty() {
                    return Err(WasmSdkError::invalid_argument(
                        "specificKeyIds must contain at least one entry",
                    ));
                }

                let request = GetIdentityKeysRequest {
                    version: Some(Version::V0(GetIdentityKeysRequestV0 {
                        identity_id: identity_id.to_vec(),
                        prove: true,
                        limit,
                        offset,
                        request_type: Some(KeyRequestType {
                            request: Some(Request::SpecificKeys(SpecificKeys {
                                key_ids: specific_key_ids,
                            })),
                        }),
                    })),
                };

                let response = request
                    .execute(self.as_ref(), RequestSettings::default())
                    .await
                    .map_err(|e| {
                        WasmSdkError::generic(format!(
                            "Failed to fetch specific identity keys: {}",
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
                                dash_sdk::platform::proto::get_identity_keys_response::get_identity_keys_response_v0::Result::Keys(keys_response) => {
                                    let mut key_map = IndexMap::new();
                                    for key_bytes in keys_response.keys_bytes {
                                        use dash_sdk::dpp::serialization::PlatformDeserializable;
                                        let key = dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey::deserialize_from_bytes(key_bytes.as_slice())
                                            .map_err(|e| WasmSdkError::serialization(format!("Failed to deserialize identity public key: {}", e)))?;
                                        key_map.insert(key.id(), Some(key));
                                    }
                                    let metadata = dash_sdk::platform::proto::ResponseMetadata {
                                        height: 0,
                                        core_chain_locked_height: 0,
                                        epoch: 0,
                                        time_ms: 0,
                                        protocol_version: 0,
                                        chain_id: "".to_string(),
                                    };
                                    let proof = dash_sdk::platform::proto::Proof {
                                        grovedb_proof: vec![],
                                        quorum_hash: vec![],
                                        signature: vec![],
                                        round: 0,
                                        block_id_hash: vec![],
                                        quorum_type: 0,
                                    };
                                    (key_map, metadata, proof)
                                }
                                _ => return Err(WasmSdkError::generic("Unexpected response format")),
                            }
                        } else {
                            return Err(WasmSdkError::not_found("No keys found in response"));
                        }
                    }
                    _ => return Err(WasmSdkError::generic("Unexpected response version")),
                }
            }
            IdentityKeysRequestInput::Search { .. } => {
                return Err(WasmSdkError::invalid_argument(
                    "Search key requests are not supported with proof",
                ))
            }
        };

        let mut keys: Vec<IdentityKeyInfoWasm> = Vec::new();

        let start = offset.unwrap_or(0) as usize;
        let end = if let Some(lim) = limit {
            start + lim as usize
        } else {
            usize::MAX
        };

        for (idx, (key_id, key_opt)) in keys_result.into_iter().enumerate() {
            if idx < start {
                continue;
            }
            if idx >= end {
                break;
            }

            if let Some(key) = key_opt {
                keys.push(IdentityKeyInfoWasm::from_entry(key_id, &key));
            }
        }

        let keys_array = Array::new();
        for key in keys {
            keys_array.push(&JsValue::from(key));
        }

        Ok(IdentityKeysProofResponseWasm {
            keys: keys_array,
            metadata: metadata.into(),
            proof: proof.into(),
        })
    }

    #[wasm_bindgen(js_name = "getIdentityBalanceWithProofInfo")]
    pub async fn get_identity_balance_with_proof_info(
        &self,
        id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityBalance;

        if id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Identity ID is required"));
        }

        let identity_id = Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let (balance_result, metadata, proof) =
            IdentityBalance::fetch_with_metadata_and_proof(self.as_ref(), identity_id, None)
                .await?;

        balance_result
            .map(|balance| {
                ProofMetadataResponseWasm::from_sdk_parts(
                    JsValue::from(IdentityBalanceWasm::new(balance)),
                    metadata,
                    proof,
                )
            })
            .ok_or_else(|| WasmSdkError::not_found("Identity balance not found"))
    }

    #[wasm_bindgen(js_name = "getIdentitiesBalancesWithProofInfo")]
    pub async fn get_identities_balances_with_proof_info(
        &self,
        identity_ids: Vec<String>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use drive_proof_verifier::types::IdentityBalance;

        // Convert string IDs to Identifiers
        let identifiers: Vec<Identifier> = identity_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

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

        let balances_array = Array::new();
        for id in identifiers {
            if let Some(Some(balance)) = balances_result.get(&id) {
                balances_array.push(&JsValue::from(IdentityBalanceEntryWasm::new(
                    id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                    *balance,
                )));
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            balances_array,
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(js_name = "getIdentityBalanceAndRevisionWithProofInfo")]
    pub async fn get_identity_balance_and_revision_with_proof_info(
        &self,
        identity_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::IdentityBalanceAndRevision;

        if identity_id.is_empty() {
            return Err(WasmSdkError::invalid_argument("Identity ID is required"));
        }

        let id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let (result, metadata, proof) =
            IdentityBalanceAndRevision::fetch_with_metadata_and_proof(self.as_ref(), id, None)
                .await?;

        result
            .map(|(balance, revision)| {
                ProofMetadataResponseWasm::from_sdk_parts(
                    JsValue::from(IdentityBalanceAndRevisionWasm::new(balance, revision)),
                    metadata,
                    proof,
                )
            })
            .ok_or_else(|| WasmSdkError::not_found("Identity balance and revision not found"))
    }

    #[wasm_bindgen(js_name = "getIdentityByPublicKeyHashWithProofInfo")]
    pub async fn get_identity_by_public_key_hash_with_proof_info(
        &self,
        public_key_hash: &str,
    ) -> Result<IdentityProofResponseWasm, WasmSdkError> {
        use dash_sdk::platform::types::identity::PublicKeyHash;

        // Parse the hex-encoded public key hash
        let hash_bytes = hex::decode(public_key_hash).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid public key hash hex: {}", e))
        })?;

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

        match result {
            Some(identity) => Ok(IdentityProofResponseWasm {
                identity: Some(IdentityWasm::from(identity)),
                metadata: metadata.into(),
                proof: proof.into(),
            }),
            None => Err(WasmSdkError::not_found(
                "Identity not found for public key hash",
            )),
        }
    }

    #[wasm_bindgen(js_name = "getIdentityByNonUniquePublicKeyHashWithProofInfo")]
    pub async fn get_identity_by_non_unique_public_key_hash_with_proof_info(
        &self,
        public_key_hash: &str,
        start_after: Option<String>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse the hex-encoded public key hash
        let hash_bytes = hex::decode(public_key_hash).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid public key hash hex: {}", e))
        })?;

        if hash_bytes.len() != 20 {
            return Err(WasmSdkError::invalid_argument(
                "Public key hash must be 20 bytes (40 hex characters)",
            ));
        }

        let mut hash_array = [0u8; 20];
        hash_array.copy_from_slice(&hash_bytes);

        // Convert start_after if provided
        let start_id = if let Some(start) = start_after {
            Some(
                Identifier::from_string(
                    &start,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
                .map_err(|e| {
                    WasmSdkError::invalid_argument(format!(
                        "Invalid start_after identity ID: {}",
                        e
                    ))
                })?,
            )
        } else {
            None
        };

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

    #[wasm_bindgen(js_name = "getIdentitiesContractKeysWithProofInfo")]
    pub async fn get_identities_contract_keys_with_proof_info(
        &self,
        identities_ids: Vec<String>,
        contract_id: &str,
        purposes: Option<Vec<u32>>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::dpp::identity::Purpose;

        // Convert string IDs to Identifiers
        let _identity_ids: Vec<Identifier> = identities_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Contract ID is not used in the individual key queries, but we validate it
        let _contract_identifier = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Convert purposes if provided
        let purposes_opt = purposes.map(|p| {
            p.into_iter()
                .filter_map(|purpose_int| match purpose_int {
                    0 => Some(Purpose::AUTHENTICATION as u32),
                    1 => Some(Purpose::ENCRYPTION as u32),
                    2 => Some(Purpose::DECRYPTION as u32),
                    3 => Some(Purpose::TRANSFER as u32),
                    4 => Some(Purpose::SYSTEM as u32),
                    5 => Some(Purpose::VOTING as u32),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });

        // For now, we'll implement this by fetching keys for each identity individually with proof
        // The SDK doesn't fully expose the batch query with proof yet
        let mut all_responses: Vec<IdentityContractKeysWasm> = Vec::new();
        let mut combined_metadata: Option<dash_sdk::platform::proto::ResponseMetadata> = None;
        let mut combined_proof: Option<dash_sdk::platform::proto::Proof> = None;

        for identity_id_str in identities_ids {
            let identity_id = Identifier::from_string(
                &identity_id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| {
                WasmSdkError::invalid_argument(format!(
                    "Invalid identity ID '{}': {}",
                    identity_id_str, e
                ))
            })?;

            // Get keys for this identity using the regular identity keys query with proof
            let (keys_result, metadata, proof) =
                IdentityPublicKey::fetch_many_with_metadata_and_proof(
                    self.as_ref(),
                    identity_id,
                    None,
                )
                .await?;

            // Store first metadata and proof
            if combined_metadata.is_none() {
                combined_metadata = Some(metadata);
                combined_proof = Some(proof);
            }

            let mut identity_keys = Vec::new();

            // Filter keys by purpose if specified
            for (key_id, key_opt) in keys_result {
                if let Some(key) = key_opt {
                    // Check if this key matches the requested purposes
                    if let Some(ref purposes) = purposes_opt {
                        if !purposes.contains(&(key.purpose() as u32)) {
                            continue;
                        }
                    }

                    identity_keys.push(IdentityKeyInfoWasm::from_entry(key_id, &key));
                }
            }

            if !identity_keys.is_empty() {
                all_responses.push(IdentityContractKeysWasm::new(
                    identity_id_str,
                    identity_keys,
                ));
            }
        }

        let responses_array = Array::new();
        for response in all_responses {
            responses_array.push(&JsValue::from(response));
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            responses_array,
            combined_metadata.unwrap_or_default(),
            combined_proof.unwrap_or_default(),
        ))
    }

    #[wasm_bindgen(js_name = "getIdentityTokenBalancesWithProofInfo")]
    pub async fn get_identity_token_balances_with_proof_info(
        &self,
        identity_id: &str,
        token_ids: Vec<String>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::dpp::balances::credits::TokenAmount;
        use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;

        let identity_id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Convert token IDs to Identifiers
        let token_identifiers: Vec<Identifier> = token_ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid token ID: {}", e)))?;

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
                let key = JsValue::from(IdentifierWasm::from(token_id));
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

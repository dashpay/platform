use crate::dpp::IdentityWasm;
use crate::sdk::WasmSdk;
use crate::queries::{ProofMetadataResponse, ResponseMetadata, ProofInfo};
use dash_sdk::platform::{Fetch, FetchMany, Identifier, Identity};
use dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use js_sys::Array;
use rs_dapi_client::IntoInner;
use drive_proof_verifier::types::{IdentityPublicKeys, IndexMap};

// Proof info functions are now included below

#[wasm_bindgen]
pub async fn identity_fetch(sdk: &WasmSdk, base58_id: &str) -> Result<IdentityWasm, JsError> {
    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    Identity::fetch_by_identifier(sdk, id)
        .await?
        .ok_or_else(|| JsError::new("Identity not found"))
        .map(Into::into)
}

#[wasm_bindgen]
pub async fn identity_fetch_with_proof_info(sdk: &WasmSdk, base58_id: &str) -> Result<JsValue, JsError> {
    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let (identity, metadata, proof) = Identity::fetch_with_metadata_and_proof(sdk, id, None)
        .await?;

    match identity {
        Some(identity) => {
            // Convert identity to JSON value first
            let identity_json = IdentityWasm::from(identity).to_json()
                .map_err(|e| JsError::new(&format!("Failed to convert identity to JSON: {:?}", e)))?;
            let identity_value: serde_json::Value = serde_wasm_bindgen::from_value(identity_json)?;

            let response = ProofMetadataResponse {
                data: identity_value,
                metadata: metadata.into(),
                proof: proof.into(),
            };

            // Use json_compatible serializer
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response.serialize(&serializer)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
        }
        None => Err(JsError::new("Identity not found")),
    }
}

#[wasm_bindgen]
pub async fn identity_fetch_unproved(sdk: &WasmSdk, base58_id: &str) -> Result<IdentityWasm, JsError> {
    use dash_sdk::platform::proto::get_identity_request::{GetIdentityRequestV0, Version as GetIdentityRequestVersion};
    use dash_sdk::platform::proto::get_identity_response::{get_identity_response_v0, GetIdentityResponseV0, Version};
    use dash_sdk::platform::proto::{GetIdentityRequest, GetIdentityResponse};
    use rs_dapi_client::{DapiRequest, RequestSettings};

    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let request = GetIdentityRequest {
        version: Some(GetIdentityRequestVersion::V0(GetIdentityRequestV0 {
            id: id.to_vec(),
            prove: false, // Request without proof
        })),
    };

    let response: GetIdentityResponse = request
        .execute(sdk.as_ref(), RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity: {}", e)))?
        .into_inner();

    match response.version {
        Some(Version::V0(GetIdentityResponseV0 {
            result: Some(get_identity_response_v0::Result::Identity(identity_bytes)),
            ..
        })) => {
            use dash_sdk::dpp::serialization::PlatformDeserializable;
            let identity = Identity::deserialize_from_bytes(
                identity_bytes.as_slice()
            )?;
            Ok(identity.into())
        }
        _ => Err(JsError::new("Identity not found")),
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdentityKeyResponse {
    key_id: u32,
    key_type: String,
    public_key_data: String,
    purpose: String,
    security_level: String,
    read_only: bool,
    disabled: bool,
}

#[wasm_bindgen]
pub async fn get_identity_keys(
    sdk: &WasmSdk,
    identity_id: &str,
    key_request_type: &str,
    specific_key_ids: Option<Vec<u32>>,
    search_purpose_map: Option<String>, // JSON string for SearchKey purpose map
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<JsValue, JsError> {

    // DapiRequestExecutor not needed anymore

    if identity_id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Handle different key request types
    let keys_result = match key_request_type {
        "all" => {
            // Use existing all keys implementation
            IdentityPublicKey::fetch_many(sdk.as_ref(), id)
                .await
                .map_err(|e| JsError::new(&format!("Failed to fetch identity keys: {}", e)))?
        }
        "specific" => {
            // Use direct gRPC request for specific keys
            use dash_sdk::platform::proto::{
                GetIdentityKeysRequest, get_identity_keys_request::{GetIdentityKeysRequestV0, Version},
                KeyRequestType, key_request_type::Request, SpecificKeys
            };
            use rs_dapi_client::{DapiRequest, RequestSettings};

            let key_ids = specific_key_ids
                .ok_or_else(|| JsError::new("specific_key_ids is required for 'specific' key request type"))?;

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: id.to_vec(),
                    prove: false,
                    limit: Some(limit.unwrap_or(100).into()), // Always provide a limit when prove=false
                    offset: None, // Offsets not supported when prove=false
                    request_type: Some(KeyRequestType {
                        request: Some(Request::SpecificKeys(SpecificKeys {
                            key_ids,
                        })),
                    }),
                })),
            };

            let response = request
                .execute(sdk.as_ref(), RequestSettings::default())
                .await
                .map_err(|e| JsError::new(&format!("Failed to fetch specific identity keys: {}", e)))?;

            // Process the response to extract keys
            use dash_sdk::platform::proto::{GetIdentityKeysResponse, get_identity_keys_response::Version as ResponseVersion};
            use rs_dapi_client::IntoInner;

            let response: GetIdentityKeysResponse = response.into_inner();
            match response.version {
                Some(ResponseVersion::V0(response_v0)) => {
                    if let Some(result) = response_v0.result {
                        match result {
                            dash_sdk::platform::proto::get_identity_keys_response::get_identity_keys_response_v0::Result::Keys(keys_response) => {
                                // Convert keys to the expected format
                                let mut key_map: IdentityPublicKeys = IndexMap::new();
                                for key_bytes in keys_response.keys_bytes {
                                    use dash_sdk::dpp::serialization::PlatformDeserializable;
                                    let key = dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey::deserialize_from_bytes(key_bytes.as_slice())?;
                                    key_map.insert(key.id(), Some(key));
                                }
                                key_map
                            }
                            _ => return Err(JsError::new("Unexpected response format")),
                        }
                    } else {
                        IndexMap::new() // Return empty map if no keys found
                    }
                }
                _ => return Err(JsError::new("Unexpected response version")),
            }
        }
        "search" => {
            // Use direct gRPC request for search keys
            use dash_sdk::platform::proto::{
                GetIdentityKeysRequest, get_identity_keys_request::{GetIdentityKeysRequestV0, Version},
                KeyRequestType, key_request_type::Request, SearchKey, SecurityLevelMap,
                security_level_map::KeyKindRequestType as GrpcKeyKindRequestType
            };
            use rs_dapi_client::{DapiRequest, RequestSettings};
            use std::collections::HashMap;

            let purpose_map_str = search_purpose_map
                .ok_or_else(|| JsError::new("search_purpose_map is required for 'search' key request type"))?;

            // Parse the JSON purpose map
            let purpose_map_json: serde_json::Value = serde_json::from_str(&purpose_map_str)
                .map_err(|e| JsError::new(&format!("Invalid JSON in search_purpose_map: {}", e)))?;

            // Convert JSON to gRPC structure
            let mut purpose_map = HashMap::new();

            if let serde_json::Value::Object(map) = purpose_map_json {
                for (purpose_str, security_levels) in map {
                    let purpose = purpose_str.parse::<u32>()
                        .map_err(|_| JsError::new(&format!("Invalid purpose value: {}", purpose_str)))?;

                    let mut security_level_map = HashMap::new();

                    if let serde_json::Value::Object(levels) = security_levels {
                        for (level_str, kind_str) in levels {
                            let level = level_str.parse::<u32>()
                                .map_err(|_| JsError::new(&format!("Invalid security level: {}", level_str)))?;

                            let kind = match kind_str.as_str().unwrap_or("") {
                                "current" | "0" => GrpcKeyKindRequestType::CurrentKeyOfKindRequest as i32,
                                "all" | "1" => GrpcKeyKindRequestType::AllKeysOfKindRequest as i32,
                                _ => return Err(JsError::new(&format!("Invalid key kind: {}", kind_str))),
                            };

                            security_level_map.insert(level, kind);
                        }
                    }

                    purpose_map.insert(purpose, SecurityLevelMap {
                        security_level_map,
                    });
                }
            } else {
                return Err(JsError::new("search_purpose_map must be a JSON object"));
            }

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: id.to_vec(),
                    prove: false,
                    limit: Some(limit.unwrap_or(100).into()), // Always provide a limit when prove=false
                    offset: None, // Offsets not supported when prove=false
                    request_type: Some(KeyRequestType {
                        request: Some(Request::SearchKey(SearchKey {
                            purpose_map,
                        })),
                    }),
                })),
            };

            let response = request
                .execute(sdk.as_ref(), RequestSettings::default())
                .await
                .map_err(|e| JsError::new(&format!("Failed to fetch search identity keys: {}", e)))?;

            // Process the response to extract keys
            use dash_sdk::platform::proto::{GetIdentityKeysResponse, get_identity_keys_response::Version as ResponseVersion};
            use rs_dapi_client::IntoInner;

            let response: GetIdentityKeysResponse = response.into_inner();
            match response.version {
                Some(ResponseVersion::V0(response_v0)) => {
                    if let Some(result) = response_v0.result {
                        match result {
                            dash_sdk::platform::proto::get_identity_keys_response::get_identity_keys_response_v0::Result::Keys(keys_response) => {
                                // Convert keys to the expected format
                                let mut key_map: IdentityPublicKeys = IndexMap::new();
                                for key_bytes in keys_response.keys_bytes {
                                    use dash_sdk::dpp::serialization::PlatformDeserializable;
                                    let key = dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey::deserialize_from_bytes(key_bytes.as_slice())?;
                                    key_map.insert(key.id(), Some(key));
                                }
                                key_map
                            }
                            _ => return Err(JsError::new("Unexpected response format")),
                        }
                    } else {
                        return Err(JsError::new("No keys found in response"));
                    }
                }
                _ => return Err(JsError::new("Unexpected response version")),
            }
        }
        _ => {
            return Err(JsError::new("Invalid key_request_type. Use 'all', 'specific', or 'search'"));
        }
    };

    // Convert keys to response format
    let mut keys: Vec<IdentityKeyResponse> = Vec::new();

    // Apply offset and limit if provided
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
            keys.push(IdentityKeyResponse {
                key_id: key_id,
                key_type: format!("{:?}", key.key_type()),
                public_key_data: hex::encode(key.data().as_slice()),
                purpose: format!("{:?}", key.purpose()),
                security_level: format!("{:?}", key.security_level()),
                read_only: key.read_only(),
                disabled: key.disabled_at().is_some(),
            });
        }
    }

    serde_wasm_bindgen::to_value(&keys)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identity_nonce(sdk: &WasmSdk, identity_id: &str) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityNonceFetcher;
    use dash_sdk::platform::Fetch;

    if identity_id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let nonce_result = IdentityNonceFetcher::fetch(sdk.as_ref(), id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity nonce: {}", e)))?;

    let nonce = nonce_result
        .map(|fetcher| fetcher.0)
        .ok_or_else(|| JsError::new("Identity nonce not found"))?;

    // Return as a JSON object with nonce as string to avoid BigInt serialization issues
    #[derive(Serialize)]
    struct NonceResponse {
        nonce: String,
    }

    let response = NonceResponse {
        nonce: nonce.to_string(),
    };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identity_nonce_with_proof_info(sdk: &WasmSdk, identity_id: &str) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityNonceFetcher;
    use dash_sdk::platform::Fetch;

    if identity_id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let (nonce_result, metadata, proof) = IdentityNonceFetcher::fetch_with_metadata_and_proof(sdk.as_ref(), id, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity nonce with proof: {}", e)))?;

    let nonce = nonce_result
        .map(|fetcher| fetcher.0)
        .ok_or_else(|| JsError::new("Identity nonce not found"))?;

    let data = serde_json::json!({
        "nonce": nonce.to_string()
    });

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
pub async fn get_identity_contract_nonce(
    sdk: &WasmSdk,
    identity_id: &str,
    contract_id: &str,
) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityContractNonceFetcher;
    use dash_sdk::platform::Fetch;

    if identity_id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    if contract_id.is_empty() {
        return Err(JsError::new("Contract ID is required"));
    }

    let identity_id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let contract_id = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let nonce_result = IdentityContractNonceFetcher::fetch(sdk.as_ref(), (identity_id, contract_id))
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity contract nonce: {}", e)))?;

    let nonce = nonce_result
        .map(|fetcher| fetcher.0)
        .ok_or_else(|| JsError::new("Identity contract nonce not found"))?;

    // Return as a JSON object with nonce as string to avoid BigInt serialization issues
    #[derive(Serialize)]
    struct NonceResponse {
        nonce: String,
    }

    let response = NonceResponse {
        nonce: nonce.to_string(),
    };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identity_contract_nonce_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
    contract_id: &str,
) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityContractNonceFetcher;
    use dash_sdk::platform::Fetch;

    if identity_id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    if contract_id.is_empty() {
        return Err(JsError::new("Contract ID is required"));
    }

    let identity_id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let contract_id = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let (nonce_result, metadata, proof) = IdentityContractNonceFetcher::fetch_with_metadata_and_proof(
        sdk.as_ref(),
        (identity_id, contract_id),
        None
    )
    .await
    .map_err(|e| JsError::new(&format!("Failed to fetch identity contract nonce with proof: {}", e)))?;

    let nonce = nonce_result
        .map(|fetcher| fetcher.0)
        .ok_or_else(|| JsError::new("Identity contract nonce not found"))?;

    let data = serde_json::json!({
        "nonce": nonce.to_string()
    });

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
pub async fn get_identity_balance(sdk: &WasmSdk, id: &str) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityBalance;
    use dash_sdk::platform::Fetch;

    if id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    let identity_id = Identifier::from_string(
        id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let balance_result = IdentityBalance::fetch(sdk.as_ref(), identity_id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity balance: {}", e)))?;

    if let Some(balance) = balance_result {
        // Return as object with balance as string to handle large numbers
        #[derive(Serialize)]
        struct BalanceResponse {
            balance: String,
        }

        let response = BalanceResponse {
            balance: balance.to_string(),
        };

        // Use json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        response.serialize(&serializer)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        Err(JsError::new("Identity balance not found"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdentityBalanceResponse {
    identity_id: String,
    balance: String,  // String to handle large numbers
}

#[wasm_bindgen]
pub async fn get_identities_balances(sdk: &WasmSdk, identity_ids: Vec<String>) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityBalance;


    // Convert string IDs to Identifiers
    let identifiers: Vec<Identifier> = identity_ids
        .into_iter()
        .map(|id| Identifier::from_string(
            &id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect::<Result<Vec<_>, _>>()?;

    let balances_result: drive_proof_verifier::types::IdentityBalances = IdentityBalance::fetch_many(sdk.as_ref(), identifiers.clone())
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identities balances: {}", e)))?;

    // Convert to response format
    let responses: Vec<IdentityBalanceResponse> = identifiers
        .into_iter()
        .filter_map(|id| {
            balances_result.get(&id).and_then(|balance_opt| {
                balance_opt.map(|balance| {
                    IdentityBalanceResponse {
                        identity_id: id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
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
pub(crate) struct IdentityBalanceAndRevisionResponse {
    balance: String,  // String to handle large numbers
    revision: u64,
}

#[wasm_bindgen]
pub async fn get_identity_balance_and_revision(sdk: &WasmSdk, identity_id: &str) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityBalanceAndRevision;
    use dash_sdk::platform::Fetch;

    if identity_id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let result = IdentityBalanceAndRevision::fetch(sdk.as_ref(), id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity balance and revision: {}", e)))?;

    if let Some(balance_and_revision) = result {
        let response = IdentityBalanceAndRevisionResponse {
            balance: balance_and_revision.0.to_string(),
            revision: balance_and_revision.1,
        };

        // Use json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        response.serialize(&serializer)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        Err(JsError::new("Identity balance and revision not found"))
    }
}

#[wasm_bindgen]
pub async fn get_identity_by_public_key_hash(sdk: &WasmSdk, public_key_hash: &str) -> Result<IdentityWasm, JsError> {
    use dash_sdk::platform::types::identity::PublicKeyHash;

    // Parse the hex-encoded public key hash
    let hash_bytes = hex::decode(public_key_hash)
        .map_err(|e| JsError::new(&format!("Invalid public key hash hex: {}", e)))?;

    if hash_bytes.len() != 20 {
        return Err(JsError::new("Public key hash must be 20 bytes (40 hex characters)"));
    }

    let mut hash_array = [0u8; 20];
    hash_array.copy_from_slice(&hash_bytes);

    let result = Identity::fetch(sdk.as_ref(), PublicKeyHash(hash_array))
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity by public key hash: {}", e)))?;

    result
        .ok_or_else(|| JsError::new("Identity not found for public key hash"))
        .map(Into::into)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdentityContractKeyResponse {
    identity_id: String,
    purpose: u32,
    key_id: u32,
    key_type: String,
    public_key_data: String,
    security_level: String,
    read_only: bool,
    disabled: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdentityContractKeysResponse {
    identity_id: String,
    keys: Vec<IdentityKeyResponse>,
}

#[wasm_bindgen]
pub async fn get_identities_contract_keys(
    sdk: &WasmSdk,
    identities_ids: Vec<String>,
    contract_id: &str,
    purposes: Option<Vec<u32>>,
) -> Result<JsValue, JsError> {
    use dash_sdk::dpp::identity::Purpose;

    // Convert string IDs to Identifiers
    let _identity_ids: Vec<Identifier> = identities_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect::<Result<Vec<_>, _>>()?;

    // Contract ID is not used in the individual key queries, but we validate it
    let _contract_identifier = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

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
    let mut responses: Vec<IdentityContractKeysResponse> = Vec::new();

    for identity_id_str in identities_ids {
        let identity_id = Identifier::from_string(
            &identity_id_str,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?;

        // Get keys for this identity using the regular identity keys query
        let keys_result = IdentityPublicKey::fetch_many(sdk.as_ref(), identity_id)
            .await
            .map_err(|e| JsError::new(&format!("Failed to fetch keys for identity {}: {}", identity_id_str, e)))?;

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

                let key_response = IdentityKeyResponse {
                    key_id: key_id,
                    key_type: format!("{:?}", key.key_type()),
                    public_key_data: hex::encode(key.data().as_slice()),
                    purpose: format!("{:?}", key.purpose()),
                    security_level: format!("{:?}", key.security_level()),
                    read_only: key.read_only(),
                    disabled: key.disabled_at().is_some(),
                };
                identity_keys.push(key_response);
            }
        }

        if !identity_keys.is_empty() {
            responses.push(IdentityContractKeysResponse {
                identity_id: identity_id_str,
                keys: identity_keys,
            });
        }
    }

    serde_wasm_bindgen::to_value(&responses)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identity_by_non_unique_public_key_hash(
    sdk: &WasmSdk,
    public_key_hash: &str,
    start_after: Option<String>,
) -> Result<JsValue, JsError> {


    // Parse the hex-encoded public key hash
    let hash_bytes = hex::decode(public_key_hash)
        .map_err(|e| JsError::new(&format!("Invalid public key hash hex: {}", e)))?;

    if hash_bytes.len() != 20 {
        return Err(JsError::new("Public key hash must be 20 bytes (40 hex characters)"));
    }

    let mut hash_array = [0u8; 20];
    hash_array.copy_from_slice(&hash_bytes);

    // Convert start_after if provided
    let start_id = if let Some(start) = start_after {
        Some(Identifier::from_string(
            &start,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?)
    } else {
        None
    };

    use dash_sdk::platform::types::identity::NonUniquePublicKeyHashQuery;

    let query = NonUniquePublicKeyHashQuery {
        key_hash: hash_array,
        after: start_id.map(|id| *id.as_bytes()),
    };

    // Fetch identity by non-unique public key hash
    let identity = Identity::fetch(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identities by non-unique public key hash: {}", e)))?;

    // Return array with single identity if found
    let results = if let Some(id) = identity {
        vec![id]
    } else {
        vec![]
    };

    // Convert results to IdentityWasm
    let identities: Vec<IdentityWasm> = results
        .into_iter()
        .map(Into::into)
        .collect();

    // Create JS array directly
    let js_array = Array::new();
    for identity in identities {
        let json = identity.to_json().map_err(|e| JsError::new(&format!("Failed to convert identity to JSON: {:?}", e)))?;
        js_array.push(&json);
    }
    Ok(js_array.into())
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenBalanceResponse {
    token_id: String,
    balance: String,  // String to handle large numbers
}

#[wasm_bindgen]
pub async fn get_identity_token_balances(
    sdk: &WasmSdk,
    identity_id: &str,
    token_ids: Vec<String>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;
    use dash_sdk::dpp::balances::credits::TokenAmount;

    let identity_id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Convert token IDs to Identifiers
    let token_identifiers: Vec<Identifier> = token_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect::<Result<Vec<_>, _>>()?;

    let query = IdentityTokenBalancesQuery {
        identity_id,
        token_ids: token_identifiers.clone(),
    };



    // Use FetchMany trait to fetch token balances
    let balances: drive_proof_verifier::types::identity_token_balance::IdentityTokenBalances = TokenAmount::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity token balances: {}", e)))?;

    // Convert to response format
    let responses: Vec<TokenBalanceResponse> = token_identifiers
        .into_iter()
        .zip(token_ids.into_iter())
        .filter_map(|(token_id, token_id_str)| {
            balances.get(&token_id).and_then(|balance_opt| {
                balance_opt.map(|balance| TokenBalanceResponse {
                    token_id: token_id_str,
                    balance: balance.to_string(),
                })
            })
        })
        .collect();

    serde_wasm_bindgen::to_value(&responses)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

// Proof info versions for identity queries

#[wasm_bindgen]
pub async fn get_identity_keys_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
    key_request_type: &str,
    specific_key_ids: Option<Vec<u32>>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<JsValue, JsError> {
    if identity_id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Handle different key request types
    let (keys_result, metadata, proof) = match key_request_type {
        "all" => {
            // Use existing all keys implementation with proof
            IdentityPublicKey::fetch_many_with_metadata_and_proof(sdk.as_ref(), id, None)
                .await
                .map_err(|e| JsError::new(&format!("Failed to fetch identity keys with proof: {}", e)))?
        }
        "specific" => {
            // For now, specific keys with proof is not implemented
            // Fall back to the non-proof version temporarily
            let key_ids = specific_key_ids
                .ok_or_else(|| JsError::new("specific_key_ids is required for 'specific' key request type"))?;

            // Use direct gRPC request for specific keys
            use dash_sdk::platform::proto::{
                GetIdentityKeysRequest, get_identity_keys_request::{GetIdentityKeysRequestV0, Version},
                KeyRequestType, key_request_type::Request, SpecificKeys
            };
            use rs_dapi_client::{DapiRequest, RequestSettings};

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: id.to_vec(),
                    prove: true,
                    limit: limit.map(|l| l.into()),
                    offset: offset.map(|o| o.into()),
                    request_type: Some(KeyRequestType {
                        request: Some(Request::SpecificKeys(SpecificKeys {
                            key_ids,
                        })),
                    }),
                })),
            };

            let response = request
                .execute(sdk.as_ref(), RequestSettings::default())
                .await
                .map_err(|e| JsError::new(&format!("Failed to fetch specific identity keys: {}", e)))?;

            // Process the response to extract keys
            use dash_sdk::platform::proto::{GetIdentityKeysResponse, get_identity_keys_response::Version as ResponseVersion};
            use rs_dapi_client::IntoInner;

            let response: GetIdentityKeysResponse = response.into_inner();
            match response.version {
                Some(ResponseVersion::V0(response_v0)) => {
                    if let Some(result) = response_v0.result {
                        match result {
                            dash_sdk::platform::proto::get_identity_keys_response::get_identity_keys_response_v0::Result::Keys(keys_response) => {
                                // Convert keys to the expected format
                                let mut key_map = IndexMap::new();
                                for key_bytes in keys_response.keys_bytes {
                                    use dash_sdk::dpp::serialization::PlatformDeserializable;
                                    let key = dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey::deserialize_from_bytes(key_bytes.as_slice())?;
                                    key_map.insert(key.id(), Some(key));
                                }
                                // Create dummy metadata and proof for consistency
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
                            _ => return Err(JsError::new("Unexpected response format")),
                        }
                    } else {
                        return Err(JsError::new("No keys found in response"));
                    }
                }
                _ => return Err(JsError::new("Unexpected response version")),
            }
        }
        _ => {
            return Err(JsError::new("Invalid key_request_type. Use 'all', 'specific', or 'search'"));
        }
    };

    // Convert keys to response format
    let mut keys: Vec<IdentityKeyResponse> = Vec::new();

    // Apply offset and limit if provided
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
            keys.push(IdentityKeyResponse {
                key_id: key_id,
                key_type: format!("{:?}", key.key_type()),
                public_key_data: hex::encode(key.data().as_slice()),
                purpose: format!("{:?}", key.purpose()),
                security_level: format!("{:?}", key.security_level()),
                read_only: key.read_only(),
                disabled: key.disabled_at().is_some(),
            });
        }
    }

    let response = ProofMetadataResponse {
        data: keys,
        metadata: metadata.into(),
        proof: proof.into(),
    };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identity_balance_with_proof_info(sdk: &WasmSdk, id: &str) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityBalance;
    use dash_sdk::platform::Fetch;

    if id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    let identity_id = Identifier::from_string(
        id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let (balance_result, metadata, proof) = IdentityBalance::fetch_with_metadata_and_proof(sdk.as_ref(), identity_id, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity balance with proof: {}", e)))?;

    if let Some(balance) = balance_result {
        #[derive(Serialize)]
        struct BalanceResponse {
            balance: String,
        }

        let data = BalanceResponse {
            balance: balance.to_string(),
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
    } else {
        Err(JsError::new("Identity balance not found"))
    }
}

#[wasm_bindgen]
pub async fn get_identities_balances_with_proof_info(sdk: &WasmSdk, identity_ids: Vec<String>) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityBalance;

    // Convert string IDs to Identifiers
    let identifiers: Vec<Identifier> = identity_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect::<Result<Vec<_>, _>>()?;

    let (balances_result, metadata, proof): (drive_proof_verifier::types::IdentityBalances, _, _) = IdentityBalance::fetch_many_with_metadata_and_proof(sdk.as_ref(), identifiers.clone(), None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identities balances with proof: {}", e)))?;

    // Convert to response format
    let responses: Vec<IdentityBalanceResponse> = identifiers
        .into_iter()
        .filter_map(|id| {
            balances_result.get(&id).and_then(|balance_opt| {
                balance_opt.map(|balance| {
                    IdentityBalanceResponse {
                        identity_id: id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
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
pub async fn get_identity_balance_and_revision_with_proof_info(sdk: &WasmSdk, identity_id: &str) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityBalanceAndRevision;
    use dash_sdk::platform::Fetch;

    if identity_id.is_empty() {
        return Err(JsError::new("Identity ID is required"));
    }

    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let (result, metadata, proof) = IdentityBalanceAndRevision::fetch_with_metadata_and_proof(sdk.as_ref(), id, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity balance and revision with proof: {}", e)))?;

    if let Some(balance_and_revision) = result {
        let data = IdentityBalanceAndRevisionResponse {
            balance: balance_and_revision.0.to_string(),
            revision: balance_and_revision.1,
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
    } else {
        Err(JsError::new("Identity balance and revision not found"))
    }
}

#[wasm_bindgen]
pub async fn get_identity_by_public_key_hash_with_proof_info(sdk: &WasmSdk, public_key_hash: &str) -> Result<JsValue, JsError> {
    use dash_sdk::platform::types::identity::PublicKeyHash;

    // Parse the hex-encoded public key hash
    let hash_bytes = hex::decode(public_key_hash)
        .map_err(|e| JsError::new(&format!("Invalid public key hash hex: {}", e)))?;

    if hash_bytes.len() != 20 {
        return Err(JsError::new("Public key hash must be 20 bytes (40 hex characters)"));
    }

    let mut hash_array = [0u8; 20];
    hash_array.copy_from_slice(&hash_bytes);

    let (result, metadata, proof) = Identity::fetch_with_metadata_and_proof(sdk.as_ref(), PublicKeyHash(hash_array), None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity by public key hash with proof: {}", e)))?;

    match result {
        Some(identity) => {
            let identity_json = IdentityWasm::from(identity).to_json()
                .map_err(|e| JsError::new(&format!("Failed to convert identity to JSON: {:?}", e)))?;
            let identity_value: serde_json::Value = serde_wasm_bindgen::from_value(identity_json)?;

            let response = ProofMetadataResponse {
                data: identity_value,
                metadata: metadata.into(),
                proof: proof.into(),
            };

            // Use json_compatible serializer
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response.serialize(&serializer)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
        }
        None => Err(JsError::new("Identity not found for public key hash")),
    }
}

#[wasm_bindgen]
pub async fn get_identity_by_non_unique_public_key_hash_with_proof_info(
    sdk: &WasmSdk,
    public_key_hash: &str,
    start_after: Option<String>,
) -> Result<JsValue, JsError> {
    // Parse the hex-encoded public key hash
    let hash_bytes = hex::decode(public_key_hash)
        .map_err(|e| JsError::new(&format!("Invalid public key hash hex: {}", e)))?;

    if hash_bytes.len() != 20 {
        return Err(JsError::new("Public key hash must be 20 bytes (40 hex characters)"));
    }

    let mut hash_array = [0u8; 20];
    hash_array.copy_from_slice(&hash_bytes);

    // Convert start_after if provided
    let start_id = if let Some(start) = start_after {
        Some(Identifier::from_string(
            &start,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?)
    } else {
        None
    };

    use dash_sdk::platform::types::identity::NonUniquePublicKeyHashQuery;

    let query = NonUniquePublicKeyHashQuery {
        key_hash: hash_array,
        after: start_id.map(|id| *id.as_bytes()),
    };

    // Fetch identity by non-unique public key hash with proof
    let (identity, metadata, proof) = Identity::fetch_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identities by non-unique public key hash with proof: {}", e)))?;

    // Return array with single identity if found
    let results = if let Some(id) = identity {
        vec![id]
    } else {
        vec![]
    };

    // Convert results to JSON
    let identities_json: Vec<serde_json::Value> = results
        .into_iter()
        .map(|identity| {
            let identity_wasm: IdentityWasm = identity.into();
            let json = identity_wasm.to_json()
                .map_err(|_| serde_wasm_bindgen::Error::new("Failed to convert identity to JSON"))?;
            serde_wasm_bindgen::from_value(json)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let response = ProofMetadataResponse {
        data: identities_json,
        metadata: metadata.into(),
        proof: proof.into(),
    };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identities_contract_keys_with_proof_info(
    sdk: &WasmSdk,
    identities_ids: Vec<String>,
    contract_id: &str,
    purposes: Option<Vec<u32>>,
) -> Result<JsValue, JsError> {
    use dash_sdk::dpp::identity::Purpose;

    // Convert string IDs to Identifiers
    let _identity_ids: Vec<Identifier> = identities_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect::<Result<Vec<_>, _>>()?;

    // Contract ID is not used in the individual key queries, but we validate it
    let _contract_identifier = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

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
    let mut all_responses: Vec<IdentityContractKeysResponse> = Vec::new();
    let mut combined_metadata: Option<ResponseMetadata> = None;
    let mut combined_proof: Option<ProofInfo> = None;

    for identity_id_str in identities_ids {
        let identity_id = Identifier::from_string(
            &identity_id_str,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?;

        // Get keys for this identity using the regular identity keys query with proof
        let (keys_result, metadata, proof) = IdentityPublicKey::fetch_many_with_metadata_and_proof(sdk.as_ref(), identity_id, None)
            .await
            .map_err(|e| JsError::new(&format!("Failed to fetch keys for identity {} with proof: {}", identity_id_str, e)))?;

        // Store first metadata and proof
        if combined_metadata.is_none() {
            combined_metadata = Some(metadata.into());
            combined_proof = Some(proof.into());
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

                let key_response = IdentityKeyResponse {
                    key_id: key_id,
                    key_type: format!("{:?}", key.key_type()),
                    public_key_data: hex::encode(key.data().as_slice()),
                    purpose: format!("{:?}", key.purpose()),
                    security_level: format!("{:?}", key.security_level()),
                    read_only: key.read_only(),
                    disabled: key.disabled_at().is_some(),
                };
                identity_keys.push(key_response);
            }
        }

        if !identity_keys.is_empty() {
            all_responses.push(IdentityContractKeysResponse {
                identity_id: identity_id_str,
                keys: identity_keys,
            });
        }
    }

    let response = ProofMetadataResponse {
        data: all_responses,
        metadata: combined_metadata.unwrap_or_else(|| ResponseMetadata {
            height: 0,
            core_chain_locked_height: 0,
            epoch: 0,
            time_ms: 0,
            protocol_version: 0,
            chain_id: String::new(),
        }),
        proof: combined_proof.unwrap_or_else(|| ProofInfo {
            grovedb_proof: String::new(),
            quorum_hash: String::new(),
            signature: String::new(),
            round: 0,
            block_id_hash: String::new(),
            quorum_type: 0,
        }),
    };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identity_token_balances_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
    token_ids: Vec<String>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;
    use dash_sdk::dpp::balances::credits::TokenAmount;

    let identity_id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Convert token IDs to Identifiers
    let token_identifiers: Vec<Identifier> = token_ids
        .iter()
        .map(|id| Identifier::from_string(
            id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        ))
        .collect::<Result<Vec<_>, _>>()?;

    let query = IdentityTokenBalancesQuery {
        identity_id,
        token_ids: token_identifiers.clone(),
    };

    // Use FetchMany trait to fetch token balances with proof
    let (balances, metadata, proof): (dash_sdk::query_types::identity_token_balance::IdentityTokenBalances, _, _) = TokenAmount::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity token balances with proof: {}", e)))?;

    // Convert to response format
    let responses: Vec<TokenBalanceResponse> = token_identifiers
        .into_iter()
        .zip(token_ids.into_iter())
        .filter_map(|(token_id, token_id_str)| {
            balances.get(&token_id).and_then(|balance_opt| {
                balance_opt.map(|balance| TokenBalanceResponse {
                    token_id: token_id_str,
                    balance: balance.to_string(),
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
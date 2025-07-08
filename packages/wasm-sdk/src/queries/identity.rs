use crate::dpp::IdentityWasm;
use crate::sdk::WasmSdk;
use dash_sdk::platform::{Fetch, FetchMany, Identifier, Identity};
use dash_sdk::dpp::identity::identity_public_key::IdentityPublicKey;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use js_sys::Array;

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IdentityKeyResponse {
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
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<JsValue, JsError> {
    
    // DapiRequestExecutor not needed anymore
    
    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Fetch all keys for now - TODO: implement specific key request once available in SDK
    if key_request_type != "all" {
        return Err(JsError::new("Currently only 'all' key request type is supported"));
    }
    
    // Use FetchMany to get identity keys
    let keys_result = IdentityPublicKey::fetch_many(sdk.as_ref(), id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity keys: {}", e)))?;
    
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
    use dash_sdk::dpp::prelude::IdentityNonce;
    use dash_sdk::platform::Fetch;
    
    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    let nonce_result = IdentityNonce::fetch(sdk.as_ref(), id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch identity nonce: {}", e)))?;
    
    let nonce = nonce_result.ok_or_else(|| JsError::new("Identity nonce not found"))?;
    
    // Return as a JSON object with nonce as string to avoid BigInt serialization issues
    #[derive(Serialize)]
    struct NonceResponse {
        nonce: String,
    }
    
    let response = NonceResponse {
        nonce: nonce.to_string(),
    };
    
    serde_wasm_bindgen::to_value(&response)
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
    
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identity_balance(sdk: &WasmSdk, id: &str) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityBalance;
    use dash_sdk::platform::Fetch;
    
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
        
        serde_wasm_bindgen::to_value(&response)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        Err(JsError::new("Identity balance not found"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IdentityBalanceResponse {
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
struct IdentityBalanceAndRevisionResponse {
    balance: String,  // String to handle large numbers
    revision: u64,
}

#[wasm_bindgen]
pub async fn get_identity_balance_and_revision(sdk: &WasmSdk, identity_id: &str) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::IdentityBalanceAndRevision;
    use dash_sdk::platform::Fetch;
    
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
        
        serde_wasm_bindgen::to_value(&response)
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
struct IdentityContractKeyResponse {
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
struct IdentityContractKeysResponse {
    identity_id: String,
    keys: Vec<IdentityKeyResponse>,
}

#[wasm_bindgen]
pub async fn get_identities_contract_keys(
    sdk: &WasmSdk,
    identities_ids: Vec<String>,
    contract_id: &str,
    document_type_name: Option<String>,
    purposes: Option<Vec<u32>>,
) -> Result<JsValue, JsError> {
    use dash_sdk::dpp::identity::Purpose;
    
    // Convert string IDs to Identifiers
    let identity_ids: Vec<Identifier> = identities_ids
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
struct TokenBalanceResponse {
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
use drive::verify::RootHash;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;
use js_sys::{Uint8Array, Array, Object, Reflect};
use std::collections::BTreeMap;

#[wasm_bindgen]
pub struct VerifyTokenPreProgrammedDistributionsResult {
    root_hash: Vec<u8>,
    distributions: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenPreProgrammedDistributionsResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn distributions(&self) -> JsValue {
        self.distributions.clone()
    }
}

// Vec variant - returns array of tuples [timestamp, recipientDistributions]
// where recipientDistributions is an array of tuples [identityId, amount]
#[wasm_bindgen(js_name = "verifyTokenPreProgrammedDistributionsVec")]
pub fn verify_token_pre_programmed_distributions_vec(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    start_at_timestamp: Option<u64>,
    start_at_identity_id: Option<Uint8Array>,
    limit: Option<u16>,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenPreProgrammedDistributionsResult, JsValue> {
    let proof_vec = proof.to_vec();
    
    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;
    
    // Build start_at parameter
    let start_at = match (start_at_timestamp, start_at_identity_id) {
        (Some(timestamp), Some(id_uint8)) => {
            let id_vec = id_uint8.to_vec();
            let id_bytes: [u8; 32] = id_vec.try_into()
                .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;
            Some(drive::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt {
                at_time_ms: timestamp,
                identity_id: Identifier::from(id_bytes),
            })
        },
        (Some(timestamp), None) => {
            Some(drive::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt {
                at_time_ms: timestamp,
                identity_id: Identifier::default(),
            })
        },
        _ => None,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    type DistributionVec = Vec<(Identifier, TokenAmount)>;
    let (root_hash, distributions_vec): (RootHash, Vec<(TimestampMillis, DistributionVec)>) = 
        drive::drive::Drive::verify_token_pre_programmed_distributions(
            &proof_vec,
            token_id_bytes,
            start_at,
            limit,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array
    let js_array = Array::new();
    for (timestamp, recipients) in distributions_vec {
        let tuple_array = Array::new();
        
        // Add timestamp
        tuple_array.push(&JsValue::from_f64(timestamp as f64));
        
        // Add recipient distributions as array of tuples
        let recipients_array = Array::new();
        for (identity_id, amount) in recipients {
            let recipient_tuple = Array::new();
            recipient_tuple.push(&Uint8Array::from(identity_id.as_slice()));
            recipient_tuple.push(&JsValue::from_f64(amount as f64));
            recipients_array.push(&recipient_tuple);
        }
        tuple_array.push(&recipients_array);
        
        js_array.push(&tuple_array);
    }

    Ok(VerifyTokenPreProgrammedDistributionsResult {
        root_hash: root_hash.to_vec(),
        distributions: js_array.into(),
    })
}

// BTreeMap variant - returns object with timestamp as key, and each value is an object with identity ID (hex) as key
#[wasm_bindgen(js_name = "verifyTokenPreProgrammedDistributionsMap")]
pub fn verify_token_pre_programmed_distributions_map(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    start_at_timestamp: Option<u64>,
    start_at_identity_id: Option<Uint8Array>,
    limit: Option<u16>,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenPreProgrammedDistributionsResult, JsValue> {
    let proof_vec = proof.to_vec();
    
    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;
    
    // Build start_at parameter
    let start_at = match (start_at_timestamp, start_at_identity_id) {
        (Some(timestamp), Some(id_uint8)) => {
            let id_vec = id_uint8.to_vec();
            let id_bytes: [u8; 32] = id_vec.try_into()
                .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;
            Some(drive::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt {
                at_time_ms: timestamp,
                identity_id: Identifier::from(id_bytes),
            })
        },
        (Some(timestamp), None) => {
            Some(drive::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt {
                at_time_ms: timestamp,
                identity_id: Identifier::default(),
            })
        },
        _ => None,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    type DistributionMap = BTreeMap<Identifier, TokenAmount>;
    let (root_hash, distributions_map): (RootHash, BTreeMap<TimestampMillis, DistributionMap>) = 
        drive::drive::Drive::verify_token_pre_programmed_distributions(
            &proof_vec,
            token_id_bytes,
            start_at,
            limit,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object
    let js_obj = Object::new();
    for (timestamp, recipients) in distributions_map {
        let recipients_obj = Object::new();
        
        for (identity_id, amount) in recipients {
            let hex_key = hex::encode(identity_id.as_slice());
            Reflect::set(&recipients_obj, &JsValue::from_str(&hex_key), &JsValue::from_f64(amount as f64))
                .map_err(|_| JsValue::from_str("Failed to set recipient amount"))?;
        }
        
        Reflect::set(&js_obj, &JsValue::from_str(&timestamp.to_string()), &recipients_obj)
            .map_err(|_| JsValue::from_str("Failed to set distribution timestamp"))?;
    }

    Ok(VerifyTokenPreProgrammedDistributionsResult {
        root_hash: root_hash.to_vec(),
        distributions: js_obj.into(),
    })
}
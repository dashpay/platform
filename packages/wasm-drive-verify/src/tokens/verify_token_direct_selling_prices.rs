use drive::verify::RootHash;
use dpp::balances::credits::Credits;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;
use js_sys::{Uint8Array, Array, Object, Reflect};
use std::collections::BTreeMap;

#[wasm_bindgen]
pub struct VerifyTokenDirectSellingPricesResult {
    root_hash: Vec<u8>,
    prices: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenDirectSellingPricesResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn prices(&self) -> JsValue {
        self.prices.clone()
    }
}

// Vec variant - returns array of tuples [tokenId, price]
#[wasm_bindgen(js_name = "verifyTokenDirectSellingPricesVec")]
pub fn verify_token_direct_selling_prices_vec(
    proof: &Uint8Array,
    token_ids: &JsValue,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenDirectSellingPricesResult, JsValue> {
    let proof_vec = proof.to_vec();
    
    // Parse token IDs from JS array
    let ids_array: Array = token_ids.clone().dyn_into()
        .map_err(|_| JsValue::from_str("token_ids must be an array"))?;
    
    let mut token_ids_vec = Vec::new();
    for i in 0..ids_array.length() {
        let id_array = ids_array.get(i);
        let id_uint8: Uint8Array = id_array.dyn_into()
            .map_err(|_| JsValue::from_str("Each token ID must be a Uint8Array"))?;
        
        let id_vec = id_uint8.to_vec();
        let id_bytes: [u8; 32] = id_vec.try_into()
            .map_err(|_| JsValue::from_str("Invalid token ID length. Expected 32 bytes."))?;
        
        token_ids_vec.push(id_bytes);
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, prices_vec): (RootHash, Vec<([u8; 32], Option<Credits>)>) = 
        drive::drive::Drive::verify_token_direct_selling_prices(
            &proof_vec,
            &token_ids_vec,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (id, price_option) in prices_vec {
        let tuple_array = Array::new();
        
        // Add token ID as Uint8Array
        let id_uint8 = Uint8Array::from(&id[..]);
        tuple_array.push(&id_uint8);
        
        // Add price
        match price_option {
            Some(price) => tuple_array.push(&JsValue::from_f64(price as f64)),
            None => tuple_array.push(&JsValue::NULL),
        }
        
        js_array.push(&tuple_array);
    }

    Ok(VerifyTokenDirectSellingPricesResult {
        root_hash: root_hash.to_vec(),
        prices: js_array.into(),
    })
}

// BTreeMap variant - returns object with token ID (hex) as key
#[wasm_bindgen(js_name = "verifyTokenDirectSellingPricesMap")]
pub fn verify_token_direct_selling_prices_map(
    proof: &Uint8Array,
    token_ids: &JsValue,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenDirectSellingPricesResult, JsValue> {
    let proof_vec = proof.to_vec();
    
    // Parse token IDs from JS array
    let ids_array: Array = token_ids.clone().dyn_into()
        .map_err(|_| JsValue::from_str("token_ids must be an array"))?;
    
    let mut token_ids_vec = Vec::new();
    for i in 0..ids_array.length() {
        let id_array = ids_array.get(i);
        let id_uint8: Uint8Array = id_array.dyn_into()
            .map_err(|_| JsValue::from_str("Each token ID must be a Uint8Array"))?;
        
        let id_vec = id_uint8.to_vec();
        let id_bytes: [u8; 32] = id_vec.try_into()
            .map_err(|_| JsValue::from_str("Invalid token ID length. Expected 32 bytes."))?;
        
        token_ids_vec.push(id_bytes);
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, prices_map): (RootHash, BTreeMap<[u8; 32], Option<Credits>>) = 
        drive::drive::Drive::verify_token_direct_selling_prices(
            &proof_vec,
            &token_ids_vec,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with hex keys
    let js_obj = Object::new();
    for (id, price_option) in prices_map {
        let hex_key = hex::encode(&id);
        
        let price_js = match price_option {
            Some(price) => JsValue::from_f64(price as f64),
            None => JsValue::NULL,
        };
        
        Reflect::set(&js_obj, &JsValue::from_str(&hex_key), &price_js)
            .map_err(|_| JsValue::from_str("Failed to set price in result object"))?;
    }

    Ok(VerifyTokenDirectSellingPricesResult {
        root_hash: root_hash.to_vec(),
        prices: js_obj.into(),
    })
}
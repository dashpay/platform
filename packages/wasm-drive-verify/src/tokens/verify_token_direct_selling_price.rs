use crate::utils::getters::VecU8ToUint8Array;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenDirectSellingPriceResult {
    root_hash: Vec<u8>,
    price: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenDirectSellingPriceResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn price(&self) -> JsValue {
        self.price.clone()
    }
}

#[wasm_bindgen(js_name = "verifyTokenDirectSellingPrice")]
pub fn verify_token_direct_selling_price(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenDirectSellingPriceResult, JsValue> {
    let proof_vec = proof.to_vec();

    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, price_option) = Drive::verify_token_direct_selling_price(
        &proof_vec,
        token_id_bytes,
        verify_subset_of_proof,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let price_js = match price_option {
        Some(pricing_schedule) => {
            // Convert TokenPricingSchedule to JS value
            match pricing_schedule {
                TokenPricingSchedule::SinglePrice(credits) => {
                    let price_obj = Object::new();
                    Reflect::set(
                        &price_obj,
                        &JsValue::from_str("type"),
                        &JsValue::from_str("single"),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set type"))?;
                    Reflect::set(
                        &price_obj,
                        &JsValue::from_str("price"),
                        &JsValue::from_f64(credits as f64),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set price"))?;
                    price_obj.into()
                }
                TokenPricingSchedule::SetPrices(prices_map) => {
                    let price_obj = Object::new();
                    Reflect::set(
                        &price_obj,
                        &JsValue::from_str("type"),
                        &JsValue::from_str("set"),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set type"))?;

                    let prices_array = Array::new();
                    for (amount, credits) in prices_map {
                        let entry = Array::new();
                        entry.push(&JsValue::from_f64(amount as f64));
                        entry.push(&JsValue::from_f64(credits as f64));
                        prices_array.push(&entry);
                    }
                    Reflect::set(&price_obj, &JsValue::from_str("prices"), &prices_array)
                        .map_err(|_| JsValue::from_str("Failed to set prices"))?;
                    price_obj.into()
                }
            }
        }
        None => JsValue::NULL,
    };

    Ok(VerifyTokenDirectSellingPriceResult {
        root_hash: root_hash.to_vec(),
        price: price_js,
    })
}

use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::verify::RootHash;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenTotalSupplyAndAggregatedIdentityBalanceResult {
    root_hash: Vec<u8>,
    total_supply_and_balance: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenTotalSupplyAndAggregatedIdentityBalanceResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn total_supply_and_balance(&self) -> JsValue {
        self.total_supply_and_balance.clone()
    }
}

#[wasm_bindgen(js_name = "verifyTokenTotalSupplyAndAggregatedIdentityBalance")]
pub fn verify_token_total_supply_and_aggregated_identity_balance(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenTotalSupplyAndAggregatedIdentityBalanceResult, JsValue> {
    let proof_vec = proof.to_vec();

    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, supply_and_balance_option) =
        Drive::verify_token_total_supply_and_aggregated_identity_balance(
            &proof_vec,
            token_id_bytes,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let result_js = match supply_and_balance_option {
        Some((total_supply, aggregated_balance)) => {
            let obj = Object::new();

            Reflect::set(
                &obj,
                &JsValue::from_str("totalSupply"),
                &JsValue::from_f64(total_supply as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set totalSupply"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("aggregatedIdentityBalance"),
                &JsValue::from_f64(aggregated_balance as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set aggregatedIdentityBalance"))?;

            obj.into()
        }
        None => JsValue::NULL,
    };

    Ok(VerifyTokenTotalSupplyAndAggregatedIdentityBalanceResult {
        root_hash: root_hash.to_vec(),
        total_supply_and_balance: result_js,
    })
}

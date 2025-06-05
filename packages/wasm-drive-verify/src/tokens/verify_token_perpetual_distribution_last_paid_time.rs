use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenPerpetualDistributionLastPaidTimeResult {
    root_hash: Vec<u8>,
    last_paid_time: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenPerpetualDistributionLastPaidTimeResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn last_paid_time(&self) -> JsValue {
        self.last_paid_time.clone()
    }
}

// TODO: This function needs a more complex API to handle RewardDistributionType
// which has variants like BlockBasedDistribution, TimeBasedDistribution, etc.
// For now, commenting out until we can properly handle the complex distribution type
/*
#[wasm_bindgen(js_name = "verifyTokenPerpetualDistributionLastPaidTime")]
pub fn verify_token_perpetual_distribution_last_paid_time(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    identity_id: &Uint8Array,
    distribution_type: &str,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenPerpetualDistributionLastPaidTimeResult, JsValue> {
    let proof_vec = proof.to_vec();

    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

    let distribution_type_enum = match distribution_type {
        "evenly" => RewardDistributionType::Evenly,
        "proRataByHoldingAmount" => RewardDistributionType::ProRataByHoldingAmount,
        _ => return Err(JsValue::from_str("Invalid distribution type. Must be 'evenly' or 'proRataByHoldingAmount'")),
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, last_paid_time_option) =
        Drive::verify_token_perpetual_distribution_last_paid_time(
            &proof_vec,
            token_id_bytes,
            identity_id_bytes,
            &distribution_type_enum,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let last_paid_time_js = match last_paid_time_option {
        Some(moment) => {
            match moment {
                RewardDistributionMoment::TimestampMillis(timestamp) => {
                    let obj = Object::new();
                    Reflect::set(&obj, &JsValue::from_str("type"), &JsValue::from_str("timestamp"))
                        .map_err(|_| JsValue::from_str("Failed to set type"))?;
                    Reflect::set(&obj, &JsValue::from_str("value"), &JsValue::from_f64(timestamp as f64))
                        .map_err(|_| JsValue::from_str("Failed to set value"))?;
                    obj.into()
                }
                RewardDistributionMoment::Block(block_height) => {
                    let obj = Object::new();
                    Reflect::set(&obj, &JsValue::from_str("type"), &JsValue::from_str("block"))
                        .map_err(|_| JsValue::from_str("Failed to set type"))?;
                    Reflect::set(&obj, &JsValue::from_str("value"), &JsValue::from_f64(block_height as f64))
                        .map_err(|_| JsValue::from_str("Failed to set value"))?;
                    obj.into()
                }
            }
        }
        None => JsValue::NULL,
    };

    Ok(VerifyTokenPerpetualDistributionLastPaidTimeResult {
        root_hash: root_hash.to_vec(),
        last_paid_time: last_paid_time_js,
    })
}
*/

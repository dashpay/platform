use dpp::version::PlatformVersion;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::utils::getters::VecU8ToUint8Array;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use drive::drive::Drive;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenPerpetualDistributionLastPaidTimeResult {
    root_hash: Vec<u8>,
    last_paid_time: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenPerpetualDistributionLastPaidTimeResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn last_paid_time(&self) -> JsValue {
        self.last_paid_time.clone()
    }
}

#[wasm_bindgen(js_name = "verifyTokenPerpetualDistributionLastPaidTime")]
pub fn verify_token_perpetual_distribution_last_paid_time(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    identity_id: &Uint8Array,
    distribution_type_js: &JsValue,
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

    // Parse the distribution type from JavaScript object
    let distribution_type = parse_reward_distribution_type(distribution_type_js)?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, last_paid_time_option) =
        Drive::verify_token_perpetual_distribution_last_paid_time(
            &proof_vec,
            token_id_bytes,
            identity_id_bytes,
            &distribution_type,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let last_paid_time_js = match last_paid_time_option {
        Some(moment) => serialize_reward_distribution_moment(&moment)?,
        None => JsValue::NULL,
    };

    Ok(VerifyTokenPerpetualDistributionLastPaidTimeResult {
        root_hash: root_hash.to_vec(),
        last_paid_time: last_paid_time_js,
    })
}

// Helper function to parse RewardDistributionType from JavaScript object
fn parse_reward_distribution_type(js_obj: &JsValue) -> Result<RewardDistributionType, JsValue> {
    let obj = js_obj
        .dyn_ref::<Object>()
        .ok_or_else(|| JsValue::from_str("Distribution type must be an object"))?;

    let type_str = Reflect::get(obj, &JsValue::from_str("type"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("Distribution type must have a 'type' field"))?;

    match type_str.as_str() {
        "block" => {
            let interval = Reflect::get(obj, &JsValue::from_str("interval"))?
                .as_f64()
                .ok_or_else(|| {
                    JsValue::from_str("Block distribution must have an 'interval' field")
                })? as u64;

            let function_obj = Reflect::get(obj, &JsValue::from_str("function"))?;
            let function = parse_distribution_function(&function_obj)?;

            Ok(RewardDistributionType::BlockBasedDistribution { interval, function })
        }
        "time" => {
            let interval = Reflect::get(obj, &JsValue::from_str("interval"))?
                .as_f64()
                .ok_or_else(|| {
                    JsValue::from_str("Time distribution must have an 'interval' field")
                })? as u64;

            let function_obj = Reflect::get(obj, &JsValue::from_str("function"))?;
            let function = parse_distribution_function(&function_obj)?;

            Ok(RewardDistributionType::TimeBasedDistribution { interval, function })
        }
        "epoch" => {
            let interval = Reflect::get(obj, &JsValue::from_str("interval"))?
                .as_f64()
                .ok_or_else(|| {
                    JsValue::from_str("Epoch distribution must have an 'interval' field")
                })? as u16;

            let function_obj = Reflect::get(obj, &JsValue::from_str("function"))?;
            let function = parse_distribution_function(&function_obj)?;

            Ok(RewardDistributionType::EpochBasedDistribution { interval, function })
        }
        _ => Err(JsValue::from_str(&format!(
            "Unknown distribution type: {}",
            type_str
        ))),
    }
}

// Helper function to parse DistributionFunction from JavaScript object
fn parse_distribution_function(js_obj: &JsValue) -> Result<DistributionFunction, JsValue> {
    let obj = js_obj
        .dyn_ref::<Object>()
        .ok_or_else(|| JsValue::from_str("Distribution function must be an object"))?;

    let type_str = Reflect::get(obj, &JsValue::from_str("type"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("Distribution function must have a 'type' field"))?;

    match type_str.as_str() {
        "fixed" => {
            let amount = Reflect::get(obj, &JsValue::from_str("amount"))?
                .as_f64()
                .ok_or_else(|| JsValue::from_str("Fixed function must have an 'amount' field"))?
                as u64;

            Ok(DistributionFunction::FixedAmount { amount })
        }
        "random" => {
            let min = Reflect::get(obj, &JsValue::from_str("min"))?
                .as_f64()
                .ok_or_else(|| JsValue::from_str("Random function must have a 'min' field"))?
                as u64;

            let max = Reflect::get(obj, &JsValue::from_str("max"))?
                .as_f64()
                .ok_or_else(|| JsValue::from_str("Random function must have a 'max' field"))?
                as u64;

            Ok(DistributionFunction::Random { min, max })
        }
        // For now, we'll support only the simplest distribution functions
        // Complex functions like Linear, Polynomial, etc. can be added later
        _ => Err(JsValue::from_str(&format!(
            "Distribution function '{}' not yet supported in WASM bindings",
            type_str
        ))),
    }
}

// Helper function to serialize RewardDistributionMoment to JavaScript object
fn serialize_reward_distribution_moment(
    moment: &RewardDistributionMoment,
) -> Result<JsValue, JsValue> {
    let obj = Object::new();

    match moment {
        RewardDistributionMoment::BlockBasedMoment(block_height) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("block"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("value"),
                &JsValue::from_f64(*block_height as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set value"))?;
        }
        RewardDistributionMoment::TimeBasedMoment(timestamp) => {
            Reflect::set(&obj, &JsValue::from_str("type"), &JsValue::from_str("time"))
                .map_err(|_| JsValue::from_str("Failed to set type"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("value"),
                &JsValue::from_f64(*timestamp as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set value"))?;
        }
        RewardDistributionMoment::EpochBasedMoment(epoch) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("epoch"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            Reflect::set(&obj, &JsValue::from_str("value"), &JsValue::from(*epoch))
                .map_err(|_| JsValue::from_str("Failed to set value"))?;
        }
    }

    Ok(obj.into())
}

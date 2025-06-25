use crate::utils::error::{format_error, format_result_error, ErrorCategory};
use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::logging::{debug, error, PerfLogger};
use crate::utils::platform_version::get_platform_version_with_validation;
use crate::utils::serialization::identity_to_js_value;
use drive::drive::Drive;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyFullIdentityByIdentityIdResult {
    root_hash: Vec<u8>,
    identity: JsValue,
}

#[wasm_bindgen]
impl VerifyFullIdentityByIdentityIdResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn identity(&self) -> JsValue {
        self.identity.clone()
    }
}

#[wasm_bindgen(js_name = "verifyFullIdentityByIdentityId")]
pub fn verify_full_identity_by_identity_id(
    proof: &Uint8Array,
    is_proof_subset: bool,
    identity_id: &Uint8Array,
    platform_version_number: u32,
) -> Result<VerifyFullIdentityByIdentityIdResult, JsValue> {
    let _perf = PerfLogger::new("identity", "verify_full_identity_by_identity_id");

    debug(
        "identity",
        format!(
            "Verifying identity with proof size: {} bytes",
            proof.length()
        ),
    );

    let proof_vec = proof.to_vec();

    let identity_id_bytes: [u8; 32] = identity_id.to_vec().try_into().map_err(|_e| {
        error("identity", "Invalid identity_id length");
        format_error(ErrorCategory::InvalidInput, "identity_id must be 32 bytes")
    })?;

    let platform_version = get_platform_version_with_validation(platform_version_number)?;

    let (root_hash, identity_option) = Drive::verify_full_identity_by_identity_id(
        &proof_vec,
        is_proof_subset,
        identity_id_bytes,
        platform_version,
    )
    .map_err(|e| {
        error("identity", format!("Verification failed: {:?}", e));
        format_result_error(ErrorCategory::VerificationError, e)
    })?;

    let identity_js = match identity_option {
        Some(identity) => {
            debug("identity", "Identity found and verified successfully");
            identity_to_js_value(identity)?
        }
        None => {
            debug("identity", "No identity found for given ID");
            JsValue::NULL
        }
    };

    Ok(VerifyFullIdentityByIdentityIdResult {
        root_hash: root_hash.to_vec(),
        identity: identity_js,
    })
}

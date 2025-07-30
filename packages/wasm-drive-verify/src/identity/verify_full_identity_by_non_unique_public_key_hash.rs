use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::serialization::identity_to_js_value;
use dpp::version::PlatformVersion;
use drive::drive::identity::identity_and_non_unique_public_key_hash_double_proof::IdentityAndNonUniquePublicKeyHashDoubleProof;
use drive::drive::Drive;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyFullIdentityByNonUniquePublicKeyHashResult {
    root_hash: Vec<u8>,
    identity: JsValue,
}

#[wasm_bindgen]
impl VerifyFullIdentityByNonUniquePublicKeyHashResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn identity(&self) -> JsValue {
        self.identity.clone()
    }
}

#[wasm_bindgen(js_name = "verifyFullIdentityByNonUniquePublicKeyHash")]
pub fn verify_full_identity_by_non_unique_public_key_hash(
    identity_proof: Option<Uint8Array>,
    identity_id_public_key_hash_proof: &Uint8Array,
    public_key_hash: &Uint8Array,
    after: Option<Uint8Array>,
    platform_version_number: u32,
) -> Result<VerifyFullIdentityByNonUniquePublicKeyHashResult, JsValue> {
    let identity_proof_vec = identity_proof.map(|proof| proof.to_vec());
    let identity_id_public_key_hash_proof_vec = identity_id_public_key_hash_proof.to_vec();

    let public_key_hash_bytes: [u8; 20] = public_key_hash
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid public_key_hash length. Expected 20 bytes."))?;

    let after_bytes = if let Some(after_array) = after {
        let after_vec = after_array.to_vec();
        Some(
            after_vec
                .try_into()
                .map_err(|_| JsValue::from_str("Invalid after length. Expected 32 bytes."))?,
        )
    } else {
        None
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let proof = IdentityAndNonUniquePublicKeyHashDoubleProof {
        identity_proof: identity_proof_vec,
        identity_id_public_key_hash_proof: identity_id_public_key_hash_proof_vec,
    };

    let (root_hash, identity_option) = Drive::verify_full_identity_by_non_unique_public_key_hash(
        &proof,
        public_key_hash_bytes,
        after_bytes,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let identity_js = match identity_option {
        Some(identity) => identity_to_js_value(identity)?,
        None => JsValue::NULL,
    };

    Ok(VerifyFullIdentityByNonUniquePublicKeyHashResult {
        root_hash: root_hash.to_vec(),
        identity: identity_js,
    })
}

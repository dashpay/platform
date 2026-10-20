use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::platform_version::get_platform_version_with_validation;
use crate::utils::proof::supported_grovedb_proof;
use crate::utils::serialization::identity_to_js_value;
use dpp::prelude::Identity;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::error::Error;
use drive::verify::RootHash;
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
    proof: &Uint8Array,
    public_key_hash: &Uint8Array,
    after: Option<Uint8Array>,
    query_platform_version_number: u32,
    platform_version_number: u32,
) -> Result<VerifyFullIdentityByNonUniquePublicKeyHashResult, JsValue> {
    let proof = proof.to_vec();

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

    let query_platform_version =
        get_platform_version_with_validation(query_platform_version_number).map_err(|error| {
            JsValue::from_str(&format!("Invalid producer platform version: {:?}", error))
        })?;
    let platform_version =
        get_platform_version_with_validation(platform_version_number).map_err(|error| {
            JsValue::from_str(&format!(
                "Invalid verification platform version: {:?}",
                error
            ))
        })?;

    supported_grovedb_proof(&proof)?;

    let (root_hash, identity_option) = verify_with_versions(
        &proof,
        public_key_hash_bytes,
        after_bytes,
        query_platform_version,
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

fn verify_with_versions(
    proof: &[u8],
    public_key_hash: [u8; 20],
    after: Option<[u8; 32]>,
    query_platform_version: &PlatformVersion,
    platform_version: &PlatformVersion,
) -> Result<(RootHash, Option<Identity>), Error> {
    Drive::verify_full_identity_by_non_unique_public_key_hash(
        proof,
        public_key_hash,
        after,
        query_platform_version,
        platform_version,
    )
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::{verify_full_identity_by_non_unique_public_key_hash, verify_with_versions};
    use dpp::version::PlatformVersion;
    use drive::error::{drive::DriveError, Error};
    use js_sys::Uint8Array;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn should_verify_first_next_and_absence_from_one_proof_blob() {
        let vectors: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../tests/fixtures/nonunique_single_proof.json"
        ))
        .expect("deterministic verified vectors");
        for version in [
            12,
            13,
            dpp::version::PlatformVersion::latest().protocol_version,
        ] {
            for vector in &vectors {
                let bytes =
                    hex::decode(vector["proof"].as_str().expect("proof")).expect("proof hex");
                let hash = hex::decode(vector["hash"].as_str().expect("hash")).expect("hash hex");
                let after = vector["after"].as_str().map(|after| {
                    Uint8Array::from(hex::decode(after).expect("cursor hex").as_slice())
                });
                let result = verify_full_identity_by_non_unique_public_key_hash(
                    &Uint8Array::from(bytes.as_slice()),
                    &Uint8Array::from(hash.as_slice()),
                    after,
                    version,
                    version,
                )
                .expect("one blob reaches complete verifier");
                assert_eq!(
                    result.root_hash().to_vec(),
                    hex::decode(vector["root"].as_str().expect("root")).expect("root hex")
                );
                if let Some(id) = vector["identity_id"].as_str() {
                    let identity: serde_json::Value =
                        serde_wasm_bindgen::from_value(result.identity()).expect("identity JSON");
                    assert_eq!(
                        identity["id"],
                        bs58::encode(hex::decode(id).expect("identity ID")).into_string()
                    );
                    assert_eq!(identity["balance"], vector["balance"]);
                    assert_eq!(identity["revision"], vector["revision"]);
                    assert_eq!(
                        identity["public_keys"]
                            .as_array()
                            .expect("all public keys")
                            .len() as u64,
                        vector["key_count"].as_u64().expect("key count")
                    );
                } else {
                    assert!(result.identity().is_null());
                }
            }
        }
    }

    #[wasm_bindgen_test]
    fn should_reject_bad_complete_proof_and_request_lengths() {
        let vector: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../tests/fixtures/nonunique_single_proof.json"
        ))
        .expect("vectors");
        let hash = hex::decode(vector[0]["hash"].as_str().expect("hash")).expect("hash bytes");
        let hash = Uint8Array::from(hash.as_slice());
        let version = dpp::version::PlatformVersion::latest().protocol_version;
        for bytes in [vec![], vec![0], vec![1]] {
            assert!(verify_full_identity_by_non_unique_public_key_hash(
                &Uint8Array::from(bytes.as_slice()),
                &hash,
                None,
                version,
                version
            )
            .is_err());
        }
        let proof = hex::decode(vector[0]["proof"].as_str().expect("proof")).expect("proof bytes");
        let proof = Uint8Array::from(proof.as_slice());
        assert!(verify_full_identity_by_non_unique_public_key_hash(
            &proof,
            &Uint8Array::from(&[0u8; 19][..]),
            None,
            version,
            version
        )
        .is_err());
        assert!(verify_full_identity_by_non_unique_public_key_hash(
            &proof,
            &hash,
            Some(Uint8Array::from(&[0u8; 31][..])),
            version,
            version
        )
        .is_err());
    }
    #[wasm_bindgen_test]
    fn should_forward_producer_and_verifier_tables_independently() {
        let vectors: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../tests/fixtures/nonunique_single_proof.json"
        ))
        .expect("verified vectors");
        let proof = hex::decode(vectors[0]["proof"].as_str().expect("proof")).expect("proof hex");
        let hash: [u8; 20] = hex::decode(vectors[0]["hash"].as_str().expect("hash"))
            .expect("hash hex")
            .try_into()
            .expect("20-byte hash");
        let latest = PlatformVersion::latest();
        let mut unknown_verifier = latest.clone();
        unknown_verifier
            .drive
            .methods
            .verify
            .identity
            .verify_full_identity_by_non_unique_public_key_hash = 1;
        let (root, identity) = verify_with_versions(&proof, hash, None, &unknown_verifier, latest)
            .expect("producer uses query table, not verifier generation");
        assert_eq!(
            root.to_vec(),
            hex::decode(vectors[0]["root"].as_str().expect("root")).expect("root hex")
        );
        assert!(identity.is_some());
        let error = verify_with_versions(&proof, hash, None, latest, &unknown_verifier)
            .expect_err("security table selects verifier generation");
        assert!(matches!(
            error,
            Error::Drive(DriveError::UnknownVersionMismatch { received: 1, .. })
        ));
    }

    #[wasm_bindgen_test]
    fn should_reject_invalid_versions_in_each_slot_and_verify_mixed_versions() {
        let vectors: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../tests/fixtures/nonunique_single_proof.json"
        ))
        .expect("verified vectors");
        let proof = Uint8Array::from(
            hex::decode(vectors[0]["proof"].as_str().expect("proof"))
                .expect("proof hex")
                .as_slice(),
        );
        let hash = Uint8Array::from(
            hex::decode(vectors[0]["hash"].as_str().expect("hash"))
                .expect("hash hex")
                .as_slice(),
        );
        let latest = PlatformVersion::latest().protocol_version;
        for invalid in [0, latest + 1, 0x10000000, 0x10000001, u32::MAX] {
            let error = verify_full_identity_by_non_unique_public_key_hash(
                &proof, &hash, None, invalid, latest,
            )
            .err()
            .expect("bad producer version");
            assert!(error
                .as_string()
                .expect("error string")
                .contains("Invalid producer platform version"));
            let error = verify_full_identity_by_non_unique_public_key_hash(
                &proof, &hash, None, 13, invalid,
            )
            .err()
            .expect("bad verifier version");
            assert!(error
                .as_string()
                .expect("error string")
                .contains("Invalid verification platform version"));
        }
        let result =
            verify_full_identity_by_non_unique_public_key_hash(&proof, &hash, None, 13, latest)
                .expect("mixed versions");
        assert_eq!(
            result.root_hash().to_vec(),
            hex::decode(vectors[0]["root"].as_str().expect("root")).expect("root hex")
        );
    }
}

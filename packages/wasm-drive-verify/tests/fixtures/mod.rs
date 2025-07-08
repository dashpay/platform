//! Test fixture loading utilities

use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::include_str;

/// Proof fixture data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ProofFixture {
    pub timestamp: String,
    pub network: String,
    #[serde(rename = "platformVersion")]
    pub platform_version: u32,
    pub proofs: HashMap<String, ProofData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProofData {
    pub description: String,
    pub proof: String, // base64 encoded
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(rename = "expectedResult")]
    pub expected_result: ExpectedResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpectedResult {
    #[serde(rename = "hasRootHash")]
    pub has_root_hash: bool,
    #[serde(rename = "hasIdentity", default)]
    pub has_identity: Option<bool>,
    #[serde(rename = "hasBalance", default)]
    pub has_balance: Option<bool>,
    #[serde(rename = "hasDocuments", default)]
    pub has_documents: Option<bool>,
    #[serde(rename = "hasContract", default)]
    pub has_contract: Option<bool>,
    #[serde(rename = "minDocuments", default)]
    pub min_documents: Option<usize>,
}

/// Load example proof fixtures
pub fn load_example_fixtures() -> ProofFixture {
    const FIXTURE_DATA: &str = include_str!("testnet_proofs/example_proofs.json");
    serde_json::from_str(FIXTURE_DATA).expect("Failed to parse fixture data")
}

/// Convert base64 proof string to Uint8Array
pub fn proof_string_to_uint8array(proof_base64: &str) -> Uint8Array {
    use base64::{engine::general_purpose, Engine as _};
    let bytes = general_purpose::STANDARD
        .decode(proof_base64)
        .expect("Invalid base64 in fixture");
    Uint8Array::from(&bytes[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_fixtures() {
        let fixtures = load_example_fixtures();
        assert_eq!(fixtures.network, "testnet");
        assert!(fixtures.proofs.contains_key("identityById"));
    }
}

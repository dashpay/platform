use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::proof::supported_grovedb_proof;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Array, Uint8Array};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyContractsByRangeResult {
    root_hash: Vec<u8>,
    contracts: JsValue,
}

#[wasm_bindgen]
impl VerifyContractsByRangeResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    /// The page rows in ascending contract id order, as an array of
    /// `[contractId: Uint8Array, contract | null]` pairs. Contracts are `null` on an
    /// ids-only page.
    #[wasm_bindgen(getter)]
    pub fn contracts(&self) -> JsValue {
        self.contracts.clone()
    }
}

/// Verifies a `getDataContractsByRange` proof against the page parameters of the request.
///
/// `start_after` and `start_at` are optional 32-byte contract ids and mutually exclusive;
/// `limit` and `ids_only` must be the values the request carried (an omitted request limit
/// is 100).
#[wasm_bindgen(js_name = "verifyContractsByRange")]
pub fn verify_contracts_by_range(
    proof: &Uint8Array,
    start_after: Option<Uint8Array>,
    start_at: Option<Uint8Array>,
    limit: u16,
    ids_only: bool,
    platform_version_number: u32,
) -> Result<VerifyContractsByRangeResult, JsValue> {
    let proof_vec = proof.to_vec();

    let start_at = match (start_after, start_at) {
        (Some(_), Some(_)) => {
            return Err(JsValue::from_str(
                "start_after and start_at are mutually exclusive",
            ))
        }
        (Some(cursor), None) => Some((contract_id_bytes(&cursor)?, false)),
        (None, Some(cursor)) => Some((contract_id_bytes(&cursor)?, true)),
        (None, None) => None,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, page) = Drive::verify_contracts_by_range(
        supported_grovedb_proof(&proof_vec, platform_version)?,
        start_at,
        limit,
        ids_only,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let rows = Array::new();
    for (contract_id, contract) in page {
        let contract_js = match contract {
            Some(contract) => to_value(&contract).map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize contract: {:?}", e))
            })?,
            None => JsValue::NULL,
        };
        let row = Array::new();
        row.push(&contract_id.to_vec().to_uint8array().into());
        row.push(&contract_js);
        rows.push(&row);
    }

    Ok(VerifyContractsByRangeResult {
        root_hash: root_hash.to_vec(),
        contracts: rows.into(),
    })
}

fn contract_id_bytes(cursor: &Uint8Array) -> Result<[u8; 32], JsValue> {
    cursor
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract id length. Expected 32 bytes."))
}

//! # Asset Lock Module
//!
//! This module provides functionality for handling asset lock proofs in identity creation

use dpp::dashcore::consensus::{deserialize, Encodable};
use dpp::dashcore::{InstantLock, OutPoint, Transaction};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockProof as DppAssetLockProof, InstantAssetLockProof,
};
use dpp::prelude::Identifier;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

/// Asset lock proof wrapper for WASM
#[wasm_bindgen]
pub struct AssetLockProof {
    inner: DppAssetLockProof,
}

#[wasm_bindgen]
impl AssetLockProof {
    /// Create an instant asset lock proof
    #[wasm_bindgen(js_name = createInstant)]
    pub fn create_instant(
        transaction_bytes: Vec<u8>,
        output_index: u32,
        instant_lock_bytes: Vec<u8>,
    ) -> Result<AssetLockProof, JsError> {
        if transaction_bytes.is_empty() {
            return Err(JsError::new("Transaction cannot be empty"));
        }
        if instant_lock_bytes.is_empty() {
            return Err(JsError::new("Instant lock cannot be empty"));
        }

        // Deserialize transaction and instant lock
        let transaction: Transaction = deserialize(&transaction_bytes)
            .map_err(|e| JsError::new(&format!("Failed to deserialize transaction: {}", e)))?;

        let instant_lock: InstantLock = deserialize(&instant_lock_bytes)
            .map_err(|e| JsError::new(&format!("Failed to deserialize instant lock: {}", e)))?;

        let instant_proof = InstantAssetLockProof::new(instant_lock, transaction, output_index);

        Ok(AssetLockProof {
            inner: DppAssetLockProof::Instant(instant_proof),
        })
    }

    /// Create a chain asset lock proof
    #[wasm_bindgen(js_name = createChain)]
    pub fn create_chain(
        core_chain_locked_height: u32,
        out_point_bytes: Vec<u8>,
    ) -> Result<AssetLockProof, JsError> {
        if out_point_bytes.len() != 36 {
            return Err(JsError::new("OutPoint must be exactly 36 bytes"));
        }

        let mut out_point_array = [0u8; 36];
        out_point_array.copy_from_slice(&out_point_bytes);

        let chain_proof = ChainAssetLockProof::new(core_chain_locked_height, out_point_array);

        Ok(AssetLockProof {
            inner: DppAssetLockProof::Chain(chain_proof),
        })
    }

    /// Get the proof type
    #[wasm_bindgen(getter, js_name = proofType)]
    pub fn proof_type(&self) -> String {
        match &self.inner {
            DppAssetLockProof::Instant(_) => "instant".to_string(),
            DppAssetLockProof::Chain(_) => "chain".to_string(),
        }
    }

    /// Get the transaction (only for instant proofs)
    #[wasm_bindgen(getter)]
    pub fn transaction(&self) -> Result<Vec<u8>, JsError> {
        match &self.inner {
            DppAssetLockProof::Instant(proof) => {
                // Serialize transaction to bytes using dashcore consensus encoding
                let mut buf = Vec::new();
                proof.transaction.consensus_encode(&mut buf).map_err(|e| {
                    JsError::new(&format!("Failed to serialize transaction: {}", e))
                })?;
                Ok(buf)
            }
            DppAssetLockProof::Chain(_) => {
                Err(JsError::new("Chain proofs don't contain transactions"))
            }
        }
    }

    /// Get the output index
    #[wasm_bindgen(getter, js_name = outputIndex)]
    pub fn output_index(&self) -> u32 {
        self.inner.output_index()
    }

    /// Get the instant lock (if present)
    #[wasm_bindgen(getter, js_name = instantLock)]
    pub fn instant_lock(&self) -> Result<Option<Vec<u8>>, JsError> {
        match &self.inner {
            DppAssetLockProof::Instant(proof) => {
                // Serialize instant lock to bytes using dashcore consensus encoding
                let mut buf = Vec::new();
                proof.instant_lock.consensus_encode(&mut buf).map_err(|e| {
                    JsError::new(&format!("Failed to serialize instant lock: {}", e))
                })?;
                Ok(Some(buf))
            }
            DppAssetLockProof::Chain(_) => Ok(None),
        }
    }

    /// Get the core chain locked height (only for chain proofs)
    #[wasm_bindgen(getter, js_name = coreChainLockedHeight)]
    pub fn core_chain_locked_height(&self) -> Option<u32> {
        match &self.inner {
            DppAssetLockProof::Chain(proof) => Some(proof.core_chain_locked_height),
            DppAssetLockProof::Instant(_) => None,
        }
    }

    /// Get the outpoint (as bytes)
    #[wasm_bindgen(getter, js_name = outPoint)]
    pub fn out_point(&self) -> Option<Vec<u8>> {
        self.inner.out_point().map(|op| {
            let bytes: [u8; 36] = op.into();
            bytes.to_vec()
        })
    }

    /// Serialize to bytes using bincode
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Result<Vec<u8>, JsError> {
        bincode::encode_to_vec(&self.inner, bincode::config::standard())
            .map_err(|e| JsError::new(&format!("Failed to serialize asset lock proof: {}", e)))
    }

    /// Deserialize from bytes using bincode
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: &[u8]) -> Result<AssetLockProof, JsError> {
        let (inner, _): (DppAssetLockProof, _) =
            bincode::decode_from_slice(bytes, bincode::config::standard()).map_err(|e| {
                JsError::new(&format!("Failed to deserialize asset lock proof: {}", e))
            })?;

        Ok(AssetLockProof { inner })
    }

    /// Serialize to JSON-compatible object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsError> {
        let value = self
            .inner
            .to_raw_object()
            .map_err(|e| JsError::new(&format!("Failed to convert to object: {}", e)))?;

        serde_wasm_bindgen::to_value(&value)
            .map_err(|e| JsError::new(&format!("Failed to serialize to JSON: {}", e)))
    }

    /// Deserialize from JSON-compatible object
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(json: JsValue) -> Result<AssetLockProof, JsError> {
        let value: platform_value::Value = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsError::new(&format!("Failed to deserialize JSON: {}", e)))?;

        let inner = DppAssetLockProof::try_from(value)
            .map_err(|e| JsError::new(&format!("Failed to convert from value: {}", e)))?;

        Ok(AssetLockProof { inner })
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();

        Reflect::set(&obj, &"type".into(), &self.proof_type().into())
            .map_err(|_| JsError::new("Failed to set type"))?;

        match &self.inner {
            DppAssetLockProof::Instant(proof) => {
                // Serialize transaction
                // Serialize transaction to bytes using dashcore consensus encoding
                let mut tx_bytes = Vec::new();
                proof
                    .transaction
                    .consensus_encode(&mut tx_bytes)
                    .map_err(|e| {
                        JsError::new(&format!("Failed to serialize transaction: {}", e))
                    })?;
                let tx_array = Uint8Array::from(&tx_bytes[..]);
                Reflect::set(&obj, &"transaction".into(), &tx_array.into())
                    .map_err(|_| JsError::new("Failed to set transaction"))?;

                // Serialize instant lock
                // Serialize instant lock to bytes using dashcore consensus encoding
                let mut lock_bytes = Vec::new();
                proof
                    .instant_lock
                    .consensus_encode(&mut lock_bytes)
                    .map_err(|e| {
                        JsError::new(&format!("Failed to serialize instant lock: {}", e))
                    })?;
                let lock_array = Uint8Array::from(&lock_bytes[..]);
                Reflect::set(&obj, &"instantLock".into(), &lock_array.into())
                    .map_err(|_| JsError::new("Failed to set instant lock"))?;

                Reflect::set(&obj, &"outputIndex".into(), &proof.output_index.into())
                    .map_err(|_| JsError::new("Failed to set output index"))?;
            }
            DppAssetLockProof::Chain(proof) => {
                Reflect::set(
                    &obj,
                    &"coreChainLockedHeight".into(),
                    &proof.core_chain_locked_height.into(),
                )
                .map_err(|_| JsError::new("Failed to set core chain locked height"))?;

                let out_point_bytes: [u8; 36] = proof.out_point.into();
                let out_point_array = Uint8Array::from(&out_point_bytes[..]);
                Reflect::set(&obj, &"outPoint".into(), &out_point_array.into())
                    .map_err(|_| JsError::new("Failed to set out point"))?;
            }
        }

        Ok(obj.into())
    }

    /// Get identity identifier created from this proof
    #[wasm_bindgen(js_name = getIdentityId)]
    pub fn get_identity_id(&self) -> Result<String, JsError> {
        let identifier = self
            .inner
            .create_identifier()
            .map_err(|e| JsError::new(&format!("Failed to create identifier: {}", e)))?;

        Ok(identifier.to_string(platform_value::string_encoding::Encoding::Base58))
    }
}

/// Validate an asset lock proof
#[wasm_bindgen(js_name = validateAssetLockProof)]
pub fn validate_asset_lock_proof(
    proof: &AssetLockProof,
    identity_id: Option<String>,
) -> Result<bool, JsError> {
    // If identity ID provided, verify it matches the proof
    if let Some(id_str) = identity_id {
        let expected_identifier =
            Identifier::from_string(&id_str, platform_value::string_encoding::Encoding::Base58)
                .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

        let proof_identifier = proof
            .inner
            .create_identifier()
            .map_err(|e| JsError::new(&format!("Failed to create identifier: {}", e)))?;

        if expected_identifier != proof_identifier {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Calculate the credits from an asset lock proof
#[wasm_bindgen(js_name = calculateCreditsFromProof)]
pub fn calculate_credits_from_proof(
    proof: &AssetLockProof,
    duffs_per_credit: Option<u64>,
) -> Result<u64, JsError> {
    // Default: 1000 duffs per credit
    let rate = duffs_per_credit.unwrap_or(1000);

    match &proof.inner {
        DppAssetLockProof::Instant(instant_proof) => {
            let output = instant_proof
                .output()
                .ok_or_else(|| JsError::new("No output found at given index"))?;
            Ok(output.value / rate)
        }
        DppAssetLockProof::Chain(_) => {
            // Chain proofs don't contain the transaction, so we can't calculate value
            Err(JsError::new(
                "Cannot calculate credits from chain proof without transaction",
            ))
        }
    }
}

/// Create an OutPoint from transaction ID and output index
#[wasm_bindgen(js_name = createOutPoint)]
pub fn create_out_point(tx_id: &str, output_index: u32) -> Result<Vec<u8>, JsError> {
    use std::str::FromStr;

    let txid = dpp::dashcore::Txid::from_str(tx_id)
        .map_err(|e| JsError::new(&format!("Invalid transaction ID: {}", e)))?;

    let out_point = OutPoint::new(txid, output_index);
    let bytes: [u8; 36] = out_point.into();
    Ok(bytes.to_vec())
}

/// Helper to create an instant asset lock proof from component parts
#[wasm_bindgen(js_name = createInstantProofFromParts)]
pub fn create_instant_proof_from_parts(
    transaction: JsValue,
    output_index: u32,
    instant_lock: JsValue,
) -> Result<AssetLockProof, JsError> {
    // Handle transaction input - could be string or Uint8Array
    let tx_bytes = if let Some(tx_str) = transaction.as_string() {
        hex::decode(&tx_str)
            .map_err(|e| JsError::new(&format!("Invalid transaction hex: {}", e)))?
    } else if let Some(array) = transaction.dyn_ref::<Uint8Array>() {
        array.to_vec()
    } else {
        return Err(JsError::new(
            "Transaction must be a hex string or Uint8Array",
        ));
    };

    // Handle instant lock input - could be string or Uint8Array
    let lock_bytes = if let Some(lock_str) = instant_lock.as_string() {
        hex::decode(&lock_str)
            .map_err(|e| JsError::new(&format!("Invalid instant lock hex: {}", e)))?
    } else if let Some(array) = instant_lock.dyn_ref::<Uint8Array>() {
        array.to_vec()
    } else {
        return Err(JsError::new(
            "Instant lock must be a hex string or Uint8Array",
        ));
    };

    AssetLockProof::create_instant(tx_bytes, output_index, lock_bytes)
}

/// Helper to create a chain asset lock proof from component parts
#[wasm_bindgen(js_name = createChainProofFromParts)]
pub fn create_chain_proof_from_parts(
    core_chain_locked_height: u32,
    tx_id: &str,
    output_index: u32,
) -> Result<AssetLockProof, JsError> {
    let out_point_bytes = create_out_point(tx_id, output_index)?;
    AssetLockProof::create_chain(core_chain_locked_height, out_point_bytes)
}

/// Get a reference to the inner DPP asset lock proof (for internal use)
impl AssetLockProof {
    pub(crate) fn inner(&self) -> &DppAssetLockProof {
        &self.inner
    }
}

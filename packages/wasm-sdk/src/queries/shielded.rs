use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use js_sys::{Array, BigInt, Uint8Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

// ── Wrapper types ──────────────────────────────────────────────────────

#[dpp_json_convertible_derive::json_safe_fields(crate = "dash_sdk::dpp")]
#[wasm_bindgen(js_name = "ShieldedEncryptedNote")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShieldedEncryptedNoteWasm {
    cmx: Vec<u8>,
    nullifier: Vec<u8>,
    cv_net: Vec<u8>,
    encrypted_note: Vec<u8>,
}

#[wasm_bindgen(js_class = ShieldedEncryptedNote)]
impl ShieldedEncryptedNoteWasm {
    #[wasm_bindgen(getter)]
    pub fn cmx(&self) -> Uint8Array {
        Uint8Array::from(self.cmx.as_slice())
    }

    #[wasm_bindgen(getter)]
    pub fn nullifier(&self) -> Uint8Array {
        Uint8Array::from(self.nullifier.as_slice())
    }

    #[wasm_bindgen(getter = cvNet)]
    pub fn cv_net(&self) -> Uint8Array {
        Uint8Array::from(self.cv_net.as_slice())
    }

    #[wasm_bindgen(getter = encryptedNote)]
    pub fn encrypted_note(&self) -> Uint8Array {
        Uint8Array::from(self.encrypted_note.as_slice())
    }
}
impl_wasm_serde_conversions!(ShieldedEncryptedNoteWasm, ShieldedEncryptedNote);

#[dpp_json_convertible_derive::json_safe_fields(crate = "dash_sdk::dpp")]
#[wasm_bindgen(js_name = "ShieldedNullifierStatus")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShieldedNullifierStatusWasm {
    nullifier: Vec<u8>,
    is_spent: bool,
}

#[wasm_bindgen(js_class = ShieldedNullifierStatus)]
impl ShieldedNullifierStatusWasm {
    #[wasm_bindgen(getter)]
    pub fn nullifier(&self) -> Uint8Array {
        Uint8Array::from(self.nullifier.as_slice())
    }

    #[wasm_bindgen(getter = isSpent)]
    pub fn is_spent(&self) -> bool {
        self.is_spent
    }
}
impl_wasm_serde_conversions!(ShieldedNullifierStatusWasm, ShieldedNullifierStatus);

// ── Query methods ──────────────────────────────────────────────────────

#[wasm_bindgen]
impl WasmSdk {
    /// Returns the total shielded pool balance as a BigInt, or undefined if not available.
    #[wasm_bindgen(js_name = "getShieldedPoolState")]
    pub async fn get_shielded_pool_state(&self) -> Result<Option<u64>, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{NoParamQuery, ShieldedPoolState};

        let result = ShieldedPoolState::fetch(self.as_ref(), NoParamQuery {}).await?;
        Ok(result.map(|s| s.0))
    }

    /// Fetches encrypted notes from the shielded pool, paginated.
    #[wasm_bindgen(
        js_name = "getShieldedEncryptedNotes",
        unchecked_return_type = "ShieldedEncryptedNote[]"
    )]
    pub async fn get_shielded_encrypted_notes(
        &self,
        #[wasm_bindgen(js_name = "startIndex")] start_index: u64,
        count: u32,
    ) -> Result<Array, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery};

        let query = ShieldedEncryptedNotesQuery { start_index, count };
        let result = ShieldedEncryptedNotes::fetch(self.as_ref(), query).await?;

        let array = Array::new();
        if let Some(notes) = result {
            for note in notes.notes {
                array.push(&JsValue::from(ShieldedEncryptedNoteWasm {
                    cmx: note.cmx,
                    nullifier: note.nullifier,
                    cv_net: note.cv_net,
                    encrypted_note: note.encrypted_note,
                }));
            }
        }
        Ok(array)
    }

    /// Returns all valid anchors for building Orchard spend proofs.
    #[wasm_bindgen(js_name = "getShieldedAnchors", unchecked_return_type = "Uint8Array[]")]
    pub async fn get_shielded_anchors(&self) -> Result<Array, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{NoParamQuery, ShieldedAnchors};

        let result = ShieldedAnchors::fetch(self.as_ref(), NoParamQuery {}).await?;

        let array = Array::new();
        if let Some(anchors) = result {
            for anchor in anchors.0 {
                array.push(&Uint8Array::from(anchor.as_slice()).into());
            }
        }
        Ok(array)
    }

    /// Returns the most recent shielded anchor (32 bytes), or undefined if none exists.
    #[wasm_bindgen(js_name = "getMostRecentShieldedAnchor")]
    pub async fn get_most_recent_shielded_anchor(
        &self,
    ) -> Result<Option<Uint8Array>, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{MostRecentShieldedAnchor, NoParamQuery};

        let result = MostRecentShieldedAnchor::fetch(self.as_ref(), NoParamQuery {}).await?;
        Ok(result.map(|anchor| Uint8Array::from(anchor.0.as_slice())))
    }

    /// Checks the spent/unspent status of one or more nullifiers.
    #[wasm_bindgen(
        js_name = "getShieldedNullifiers",
        unchecked_return_type = "ShieldedNullifierStatus[]"
    )]
    pub async fn get_shielded_nullifiers(
        &self,
        #[wasm_bindgen(unchecked_param_type = "Uint8Array[]")] nullifiers: js_sys::Array,
    ) -> Result<Array, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{ShieldedNullifierStatuses, ShieldedNullifiersQuery};

        let nullifier_arrays: Vec<[u8; 32]> = nullifiers
            .iter()
            .enumerate()
            .map(|(i, v)| {
                wasm_dpp2::utils::try_to_fixed_bytes::<32>(v, &format!("nullifiers[{}]", i))
                    .map_err(WasmSdkError::from)
            })
            .collect::<Result<_, _>>()?;

        let query = ShieldedNullifiersQuery(nullifier_arrays);
        let result = ShieldedNullifierStatuses::fetch(self.as_ref(), query).await?;

        let array = Array::new();
        if let Some(statuses) = result {
            for status in statuses.0 {
                array.push(&JsValue::from(ShieldedNullifierStatusWasm {
                    nullifier: status.nullifier.to_vec(),
                    is_spent: status.is_spent,
                }));
            }
        }
        Ok(array)
    }

    // ── Proof variants ─────────────────────────────────────────────────

    #[wasm_bindgen(
        js_name = "getShieldedPoolStateWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<bigint | null>"
    )]
    pub async fn get_shielded_pool_state_with_proof_info(
        &self,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{NoParamQuery, ShieldedPoolState};

        let (result, metadata, proof) =
            ShieldedPoolState::fetch_with_metadata_and_proof(self.as_ref(), NoParamQuery {}, None)
                .await?;

        let data = result
            .map(|s| JsValue::from(BigInt::from(s.0)))
            .unwrap_or(JsValue::NULL);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getShieldedEncryptedNotesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ShieldedEncryptedNote[]>"
    )]
    pub async fn get_shielded_encrypted_notes_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "startIndex")] start_index: u64,
        count: u32,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery};

        let query = ShieldedEncryptedNotesQuery { start_index, count };
        let (result, metadata, proof) =
            ShieldedEncryptedNotes::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        let array = Array::new();
        if let Some(notes) = result {
            for note in notes.notes {
                array.push(&JsValue::from(ShieldedEncryptedNoteWasm {
                    cmx: note.cmx,
                    nullifier: note.nullifier,
                    cv_net: note.cv_net,
                    encrypted_note: note.encrypted_note,
                }));
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            array, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getShieldedAnchorsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Uint8Array[]>"
    )]
    pub async fn get_shielded_anchors_with_proof_info(
        &self,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{NoParamQuery, ShieldedAnchors};

        let (result, metadata, proof) =
            ShieldedAnchors::fetch_with_metadata_and_proof(self.as_ref(), NoParamQuery {}, None)
                .await?;

        let array = Array::new();
        if let Some(anchors) = result {
            for anchor in anchors.0 {
                array.push(&Uint8Array::from(anchor.as_slice()).into());
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            array, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getMostRecentShieldedAnchorWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Uint8Array | null>"
    )]
    pub async fn get_most_recent_shielded_anchor_with_proof_info(
        &self,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{MostRecentShieldedAnchor, NoParamQuery};

        let (result, metadata, proof) = MostRecentShieldedAnchor::fetch_with_metadata_and_proof(
            self.as_ref(),
            NoParamQuery {},
            None,
        )
        .await?;

        let data = result
            .map(|a| JsValue::from(Uint8Array::from(a.0.as_slice())))
            .unwrap_or(JsValue::NULL);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getShieldedNullifiersWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ShieldedNullifierStatus[]>"
    )]
    pub async fn get_shielded_nullifiers_with_proof_info(
        &self,
        #[wasm_bindgen(unchecked_param_type = "Uint8Array[]")] nullifiers: js_sys::Array,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{ShieldedNullifierStatuses, ShieldedNullifiersQuery};

        let nullifier_arrays: Vec<[u8; 32]> = nullifiers
            .iter()
            .enumerate()
            .map(|(i, v)| {
                wasm_dpp2::utils::try_to_fixed_bytes::<32>(v, &format!("nullifiers[{}]", i))
                    .map_err(WasmSdkError::from)
            })
            .collect::<Result<_, _>>()?;

        let query = ShieldedNullifiersQuery(nullifier_arrays);
        let (result, metadata, proof) =
            ShieldedNullifierStatuses::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        let array = Array::new();
        if let Some(statuses) = result {
            for status in statuses.0 {
                array.push(&JsValue::from(ShieldedNullifierStatusWasm {
                    nullifier: status.nullifier.to_vec(),
                    is_spent: status.is_spent,
                }));
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            array, metadata, proof,
        ))
    }
}

use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::utils::WithJsError;

use dpp::serialization::PlatformSerializable;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::state_transition::{StateTransition, StateTransitionLike};

#[wasm_bindgen(js_name = ShieldedTransferTransition)]
#[derive(Clone)]
pub struct ShieldedTransferTransitionWasm(ShieldedTransferTransition);

impl From<ShieldedTransferTransition> for ShieldedTransferTransitionWasm {
    fn from(v: ShieldedTransferTransition) -> Self {
        ShieldedTransferTransitionWasm(v)
    }
}

impl From<ShieldedTransferTransitionWasm> for ShieldedTransferTransition {
    fn from(v: ShieldedTransferTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = ShieldedTransferTransition)]
impl ShieldedTransferTransitionWasm {
    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> u8 {
        self.0.state_transition_type() as u8
    }

    /// Returns the serialized Orchard actions as a JS array.
    #[wasm_bindgen(js_name = getActions)]
    pub fn get_actions(&self) -> Result<JsValue, JsValue> {
        let inner = match &self.0 {
            ShieldedTransferTransition::V0(v0) => &v0.actions,
        };
        serde_wasm_bindgen::to_value(inner).map_err(|e| JsValue::from(e.to_string()))
    }

    /// Returns the bundle flags byte.
    #[wasm_bindgen(js_name = getFlags)]
    pub fn get_flags(&self) -> u8 {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.flags,
        }
    }

    /// Returns the net value balance.
    #[wasm_bindgen(js_name = getValueBalance)]
    pub fn get_value_balance(&self) -> i64 {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.value_balance,
        }
    }

    /// Returns the anchor (32-byte Merkle root) as a Buffer.
    #[wasm_bindgen(js_name = getAnchor)]
    pub fn get_anchor(&self) -> Buffer {
        let anchor = match &self.0 {
            ShieldedTransferTransition::V0(v0) => &v0.anchor,
        };
        Buffer::from_bytes(anchor)
    }

    /// Returns the Halo2 proof bytes as a Buffer.
    #[wasm_bindgen(js_name = getProof)]
    pub fn get_proof(&self) -> Buffer {
        let proof = match &self.0 {
            ShieldedTransferTransition::V0(v0) => &v0.proof,
        };
        Buffer::from_bytes(proof)
    }

    /// Returns the RedPallas binding signature (64 bytes) as a Buffer.
    #[wasm_bindgen(js_name = getBindingSignature)]
    pub fn get_binding_signature(&self) -> Buffer {
        let sig = match &self.0 {
            ShieldedTransferTransition::V0(v0) => &v0.binding_signature,
        };
        Buffer::from_bytes(sig)
    }

    /// Returns the user fee increase multiplier.
    #[wasm_bindgen(js_name = getUserFeeIncrease)]
    pub fn get_user_fee_increase(&self) -> u16 {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.user_fee_increase,
        }
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.0).map_err(|e| JsValue::from(e.to_string()))
    }

    #[wasm_bindgen(js_name = toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = PlatformSerializable::serialize_to_bytes(&StateTransition::ShieldedTransfer(
            self.0.clone(),
        ))
        .with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = serde_json::to_value(&self.0).map_err(|e| JsValue::from(e.to_string()))?;
        serde_wasm_bindgen::to_value(&json).map_err(|e| JsValue::from(e.to_string()))
    }

    #[wasm_bindgen(js_name = getModifiedDataIds)]
    pub fn modified_data_ids(&self) -> Vec<JsValue> {
        self.0
            .modified_data_ids()
            .into_iter()
            .map(|id| {
                let wrapper = crate::identifier::IdentifierWrapper::from(id);
                wrapper.into()
            })
            .collect()
    }

    #[wasm_bindgen(js_name = isDataContractStateTransition)]
    pub fn is_data_contract_state_transition(&self) -> bool {
        self.0.is_data_contract_state_transition()
    }

    #[wasm_bindgen(js_name = isDocumentStateTransition)]
    pub fn is_document_state_transition(&self) -> bool {
        self.0.is_document_state_transition()
    }

    #[wasm_bindgen(js_name = isIdentityStateTransition)]
    pub fn is_identity_state_transition(&self) -> bool {
        self.0.is_identity_state_transition()
    }

    #[wasm_bindgen(js_name = isVotingStateTransition)]
    pub fn is_voting_state_transition(&self) -> bool {
        self.0.is_voting_state_transition()
    }
}

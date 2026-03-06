use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::utils::WithJsError;

use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::state_transition::{StateTransition, StateTransitionLike};

#[wasm_bindgen(js_name = ShieldedWithdrawalTransition)]
#[derive(Clone)]
pub struct ShieldedWithdrawalTransitionWasm(ShieldedWithdrawalTransition);

impl From<ShieldedWithdrawalTransition> for ShieldedWithdrawalTransitionWasm {
    fn from(v: ShieldedWithdrawalTransition) -> Self {
        ShieldedWithdrawalTransitionWasm(v)
    }
}

impl From<ShieldedWithdrawalTransitionWasm> for ShieldedWithdrawalTransition {
    fn from(v: ShieldedWithdrawalTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = ShieldedWithdrawalTransition)]
impl ShieldedWithdrawalTransitionWasm {
    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> u8 {
        self.0.state_transition_type() as u8
    }

    /// Returns the serialized Orchard actions as a JS array.
    #[wasm_bindgen(js_name = getActions)]
    pub fn get_actions(&self) -> Result<JsValue, JsValue> {
        let inner = match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => &v0.actions,
        };
        serde_wasm_bindgen::to_value(inner).map_err(|e| JsValue::from(e.to_string()))
    }

    /// Returns the net value balance.
    #[wasm_bindgen(js_name = getUnshieldingAmount)]
    pub fn get_unshielding_amount(&self) -> u64 {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.unshielding_amount,
        }
    }

    /// Returns the anchor (32-byte Merkle root) as a Buffer.
    #[wasm_bindgen(js_name = getAnchor)]
    pub fn get_anchor(&self) -> Buffer {
        let anchor = match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => &v0.anchor,
        };
        Buffer::from_bytes(anchor)
    }

    /// Returns the Halo2 proof bytes as a Buffer.
    #[wasm_bindgen(js_name = getProof)]
    pub fn get_proof(&self) -> Buffer {
        let proof = match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => &v0.proof,
        };
        Buffer::from_bytes(proof)
    }

    /// Returns the RedPallas binding signature (64 bytes) as a Buffer.
    #[wasm_bindgen(js_name = getBindingSignature)]
    pub fn get_binding_signature(&self) -> Buffer {
        let sig = match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => &v0.binding_signature,
        };
        Buffer::from_bytes(sig)
    }

    /// Returns the core fee per byte.
    #[wasm_bindgen(js_name = getCoreFeePerByte)]
    pub fn get_core_fee_per_byte(&self) -> u32 {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.core_fee_per_byte,
        }
    }

    /// Returns the pooling strategy as a u8.
    #[wasm_bindgen(js_name = getPooling)]
    pub fn get_pooling(&self) -> u8 {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.pooling as u8,
        }
    }

    /// Returns the output script (core address) as a Buffer.
    #[wasm_bindgen(js_name = getOutputScript)]
    pub fn get_output_script(&self) -> Buffer {
        let script = match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => &v0.output_script,
        };
        Buffer::from_bytes(script.as_bytes())
    }

    /// Always returns 0 — the fee is cryptographically locked by the Orchard binding signature.
    #[wasm_bindgen(js_name = getUserFeeIncrease)]
    pub fn get_user_fee_increase(&self) -> u16 {
        0
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.0).map_err(|e| JsValue::from(e.to_string()))
    }

    #[wasm_bindgen(js_name = toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = PlatformSerializable::serialize_to_bytes(&StateTransition::ShieldedWithdrawal(
            self.0.clone(),
        ))
        .with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name = fromBuffer)]
    pub fn from_buffer(buffer: Vec<u8>) -> Result<ShieldedWithdrawalTransitionWasm, JsValue> {
        let state_transition: StateTransition =
            PlatformDeserializable::deserialize_from_bytes(&buffer).with_js_error()?;
        match state_transition {
            StateTransition::ShieldedWithdrawal(st) => Ok(st.into()),
            _ => Err(JsValue::from_str("Invalid state transition type")),
        }
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

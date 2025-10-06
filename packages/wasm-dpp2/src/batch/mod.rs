use crate::batch::batched_transition::BatchedTransitionWasm;
use crate::batch::document_transition::DocumentTransitionWasm;
use crate::identifier::IdentifierWasm;
use crate::state_transition::StateTransitionWasm;
use crate::utils::{IntoWasm, WithJsError};
use dpp::fee::Credits;
use dpp::identity::KeyID;
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::{
    BatchTransition, BatchTransitionV0, BatchTransitionV1,
};
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned, StateTransitionLike};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

pub mod batched_transition;
pub mod document_base_transition;
pub mod document_transition;
pub mod document_transitions;
pub mod generators;
pub mod prefunded_voting_balance;
pub mod token_base_transition;
pub mod token_payment_info;
pub mod token_pricing_schedule;
pub mod token_transition;
pub mod token_transitions;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=BatchTransition)]
pub struct BatchTransitionWasm(BatchTransition);

impl From<BatchTransition> for BatchTransitionWasm {
    fn from(batch: BatchTransition) -> Self {
        BatchTransitionWasm(batch)
    }
}

impl From<BatchTransitionWasm> for BatchTransition {
    fn from(batch: BatchTransitionWasm) -> Self {
        batch.0
    }
}

fn convert_array_to_vec_batched(js_batched_transitions: &js_sys::Array) -> Vec<BatchedTransition> {
    js_batched_transitions
        .clone()
        .iter()
        .map(|js_batched_transition| {
            let batched_transition: BatchedTransitionWasm = js_batched_transition
                .to_wasm::<BatchedTransitionWasm>("BatchedTransition")
                .unwrap()
                .clone();
            BatchedTransition::from(batched_transition)
        })
        .collect()
}

#[wasm_bindgen(js_class = BatchTransition)]
impl BatchTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "BatchTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "BatchTransition".to_string()
    }

    #[wasm_bindgen(js_name = "fromV1BatchedTransitions")]
    pub fn from_v1_batched_transitions(
        js_batched_transitions: &js_sys::Array,
        owner_id: &JsValue,
        user_fee_increase: UserFeeIncrease,
        signature_public_key_id: Option<u32>,
        signature: Option<Vec<u8>>,
    ) -> Result<BatchTransitionWasm, JsValue> {
        let transitions = convert_array_to_vec_batched(&js_batched_transitions);

        Ok(BatchTransitionWasm(BatchTransition::V1(
            BatchTransitionV1 {
                owner_id: IdentifierWasm::try_from(owner_id)?.into(),
                transitions,
                user_fee_increase,
                signature_public_key_id: signature_public_key_id.unwrap_or(0u32),
                signature: BinaryData::from(signature.unwrap_or(Vec::new())),
            },
        )))
    }

    #[wasm_bindgen(js_name = "fromV0Transitions")]
    pub fn from_v0_transitions(
        document_transitions: &js_sys::Array,
        js_owner_id: &JsValue,
        user_fee_increase: Option<UserFeeIncrease>,
        signature_public_key_id: Option<KeyID>,
        signature: Option<Vec<u8>>,
    ) -> Result<BatchTransitionWasm, JsValue> {
        let owner_id = IdentifierWasm::try_from(js_owner_id)?;

        let transitions: Vec<DocumentTransition> = document_transitions
            .clone()
            .iter()
            .map(|js_document_transition| {
                let document_transition: DocumentTransitionWasm = js_document_transition
                    .to_wasm::<DocumentTransitionWasm>("DocumentTransition")
                    .unwrap()
                    .clone();

                DocumentTransition::from(document_transition.clone().clone())
            })
            .collect();

        Ok(BatchTransitionWasm(BatchTransition::V0(
            BatchTransitionV0 {
                owner_id: owner_id.into(),
                transitions,
                user_fee_increase: user_fee_increase.unwrap_or(0),
                signature_public_key_id: signature_public_key_id.unwrap_or(0),
                signature: BinaryData::from(signature.unwrap_or(Vec::new())),
            },
        )))
    }

    #[wasm_bindgen(getter = "transitions")]
    pub fn get_transitions(&self) -> Vec<BatchedTransitionWasm> {
        self.0
            .transitions_iter()
            .map(|transition| BatchedTransitionWasm::from(transition.to_owned_transition()))
            .collect()
    }

    #[wasm_bindgen(setter = "transitions")]
    pub fn set_transitions(&mut self, js_batched_transitions: &js_sys::Array) {
        let transitions = convert_array_to_vec_batched(&js_batched_transitions);

        self.0.set_transitions(transitions)
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn get_signature_public_key_id(&self) -> KeyID {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter = "allPurchasesAmount")]
    pub fn get_all_purchases_amount(&self) -> Result<Option<Credits>, JsValue> {
        self.0.all_document_purchases_amount().with_js_error()
    }

    #[wasm_bindgen(getter = "ownerId")]
    pub fn get_owner_id(&self) -> IdentifierWasm {
        self.0.owner_id().into()
    }

    #[wasm_bindgen(getter = "modifiedDataIds")]
    pub fn get_modified_data_ids(&self) -> Vec<IdentifierWasm> {
        self.0
            .modified_data_ids()
            .iter()
            .map(|id| id.clone().into())
            .collect()
    }

    #[wasm_bindgen(getter = "allConflictingIndexCollateralVotingFunds")]
    pub fn get_all_conflicting_index_collateral_voting_funds(
        &self,
    ) -> Result<Option<Credits>, JsValue> {
        self.0
            .all_conflicting_index_collateral_voting_funds()
            .with_js_error()
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, js_signature: Vec<u8>) {
        self.0.set_signature(BinaryData::from(js_signature))
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.0.set_signature_public_key_id(key_id)
    }

    #[wasm_bindgen(js_name = "setIdentityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        self.0.set_identity_contract_nonce(nonce)
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        let st = StateTransition::from(self.0.clone());

        StateTransitionWasm::from(st)
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        state_transition: &StateTransitionWasm,
    ) -> Result<BatchTransitionWasm, JsValue> {
        let rs_transition: StateTransition = StateTransition::from(state_transition.clone());

        match rs_transition {
            StateTransition::Batch(batch) => Ok(BatchTransitionWasm(batch)),
            _ => Err(JsValue::from("invalid state document_transition content")),
        }
    }

    #[wasm_bindgen(js_name = "bytes")]
    pub fn to_bytes(&self) -> Result<Vec<u8>, JsValue> {
        let bytes = self.0.serialize_to_bytes().with_js_error()?;

        Ok(bytes)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> Result<String, JsValue> {
        Ok(encode(
            self.0.serialize_to_bytes().with_js_error()?.as_slice(),
            Hex,
        ))
    }

    #[wasm_bindgen(js_name = "base64")]
    pub fn to_base64(&self) -> Result<String, JsValue> {
        Ok(encode(
            self.0.serialize_to_bytes().with_js_error()?.as_slice(),
            Base64,
        ))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<BatchTransitionWasm, JsValue> {
        let rs_batch = BatchTransition::deserialize_from_bytes(bytes.as_slice()).with_js_error()?;

        Ok(BatchTransitionWasm::from(rs_batch))
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> Result<BatchTransitionWasm, JsValue> {
        BatchTransitionWasm::from_bytes(decode(base64.as_str(), Base64).map_err(JsError::from)?)
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> Result<BatchTransitionWasm, JsValue> {
        BatchTransitionWasm::from_bytes(decode(hex.as_str(), Hex).map_err(JsError::from)?)
    }
}

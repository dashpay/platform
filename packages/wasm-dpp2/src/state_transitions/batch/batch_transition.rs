use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::state_transitions::StateTransitionWasm;
use crate::state_transitions::batch::batched_transition::BatchedTransitionWasm;
use crate::utils::IntoWasm;
use dpp::fee::Credits;
use dpp::identity::KeyID;
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
use dpp::state_transition::{
    StateTransition, StateTransitionIdentitySigned, StateTransitionLike, StateTransitionOwned,
    StateTransitionSingleSigned,
};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
/**
 * BatchTransition serialized as a plain object.
 */
export interface BatchTransitionObject {
    $formatVersion: string;
    ownerId: Uint8Array;
    transitions: BatchedTransitionObject[];
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature: Uint8Array;
}

/**
 * BatchTransition serialized as JSON.
 */
export interface BatchTransitionJSON {
    $formatVersion: string;
    ownerId: string;
    transitions: BatchedTransitionJSON[];
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "BatchTransitionObject")]
    pub type BatchTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "BatchTransitionJSON")]
    pub type BatchTransitionJSONJs;
}

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

fn convert_array_to_vec_batched(
    batched_transitions: &js_sys::Array,
) -> WasmDppResult<Vec<BatchedTransition>> {
    let mut transitions = Vec::with_capacity(batched_transitions.length() as usize);

    for batched_transition in batched_transitions.iter() {
        let transition: BatchedTransitionWasm = batched_transition
            .to_wasm::<BatchedTransitionWasm>("BatchedTransition")?
            .clone();

        transitions.push(BatchedTransition::from(transition));
    }

    Ok(transitions)
}

#[wasm_bindgen(js_class = BatchTransition)]
impl BatchTransitionWasm {
    #[wasm_bindgen(js_name = "fromBatchedTransitions")]
    pub fn from_batched_transitions(
        batched_transitions: &js_sys::Array,
        owner_id: IdentifierLikeJs,
        user_fee_increase: UserFeeIncrease,
    ) -> WasmDppResult<BatchTransitionWasm> {
        let transitions = convert_array_to_vec_batched(batched_transitions)?;

        Ok(BatchTransitionWasm(BatchTransition::V1(
            BatchTransitionV1 {
                owner_id: owner_id.try_into()?,
                transitions,
                user_fee_increase,
                signature_public_key_id: 0u32,
                signature: BinaryData::default(),
            },
        )))
    }

    #[wasm_bindgen(getter = "transitions")]
    pub fn get_batched_transitions(&self) -> Vec<BatchedTransitionWasm> {
        self.0
            .transitions_iter()
            .map(|transition| BatchedTransitionWasm::from(transition.to_owned_transition()))
            .collect()
    }

    #[wasm_bindgen(setter = "transitions")]
    pub fn set_transitions(&mut self, batched_transitions: &js_sys::Array) -> WasmDppResult<()> {
        let transitions = convert_array_to_vec_batched(batched_transitions)?;

        self.0.set_transitions(transitions);
        Ok(())
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
    pub fn get_all_purchases_amount(&self) -> WasmDppResult<Option<Credits>> {
        self.0.all_document_purchases_amount().map_err(Into::into)
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
            .map(|id| (*id).into())
            .collect()
    }

    #[wasm_bindgen(getter = "allConflictingIndexCollateralVotingFunds")]
    pub fn get_all_conflicting_index_collateral_voting_funds(
        &self,
    ) -> WasmDppResult<Option<Credits>> {
        self.0
            .all_conflicting_index_collateral_voting_funds()
            .map_err(Into::into)
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature(BinaryData::from(signature))
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
    ) -> WasmDppResult<BatchTransitionWasm> {
        let rs_transition: StateTransition = StateTransition::from(state_transition.clone());

        match rs_transition {
            StateTransition::Batch(batch) => Ok(BatchTransitionWasm(batch)),
            _ => Err(WasmDppError::invalid_argument(
                "invalid state document_transition content",
            )),
        }
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        self.0.serialize_to_bytes().map_err(Into::into)
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<BatchTransitionObjectJs> {
        serialization::to_object(&self.0).map(Into::into)
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<BatchTransitionJSONJs> {
        serialization::to_json(&self.0).map(Into::into)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<BatchTransitionWasm> {
        let rs_batch = BatchTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(BatchTransitionWasm::from(rs_batch))
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(object: BatchTransitionObjectJs) -> WasmDppResult<BatchTransitionWasm> {
        serialization::from_object(object.into()).map(BatchTransitionWasm)
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(object: BatchTransitionJSONJs) -> WasmDppResult<BatchTransitionWasm> {
        serialization::from_json(object.into()).map(BatchTransitionWasm)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<BatchTransitionWasm> {
        BatchTransitionWasm::from_bytes(
            decode(base64.as_str(), Base64)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
        )
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<BatchTransitionWasm> {
        BatchTransitionWasm::from_bytes(
            decode(hex.as_str(), Hex)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
        )
    }
}

impl_wasm_type_info!(BatchTransitionWasm, BatchTransition);

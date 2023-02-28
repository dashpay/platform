use std::{convert::TryFrom, ops::Deref};

use dpp::{
    identity::KeyType,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext, StateTransition,
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
    },
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::{buffer::Buffer, errors::from_dpp_err, identifier::IdentifierWrapper, with_js_error};

pub mod errors;
pub mod state_transition_factory;

#[wasm_bindgen(js_name=StateTransitionExecutionContext)]
pub struct StateTransitionExecutionContextWasm(StateTransitionExecutionContext);

impl From<StateTransitionExecutionContext> for StateTransitionExecutionContextWasm {
    fn from(rs: StateTransitionExecutionContext) -> Self {
        StateTransitionExecutionContextWasm(rs)
    }
}

impl From<StateTransitionExecutionContextWasm> for StateTransitionExecutionContext {
    fn from(wa: StateTransitionExecutionContextWasm) -> Self {
        wa.0
    }
}

impl<'a> From<&'a StateTransitionExecutionContextWasm> for StateTransitionExecutionContext {
    fn from(wa: &StateTransitionExecutionContextWasm) -> Self {
        wa.0.clone()
    }
}

impl<'a> From<&'a StateTransitionExecutionContextWasm> for &'a StateTransitionExecutionContext {
    fn from(wa: &'a StateTransitionExecutionContextWasm) -> Self {
        &wa.0
    }
}

impl<'a> From<&'a StateTransitionExecutionContext> for StateTransitionExecutionContextWasm {
    fn from(rs: &'a StateTransitionExecutionContext) -> Self {
        Self(rs.clone())
    }
}

impl Default for StateTransitionExecutionContextWasm {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class=StateTransitionExecutionContext)]
impl StateTransitionExecutionContextWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> StateTransitionExecutionContextWasm {
        StateTransitionExecutionContext::default().into()
    }

    #[wasm_bindgen(js_name=enableDryRun)]
    pub fn enable_dry_run(&self) {
        self.0.enable_dry_run();
    }

    #[wasm_bindgen(js_name=disableDryRun)]
    pub fn disable_dry_run(&self) {
        self.0.disable_dry_run();
    }
}

macro_rules! either_st {
    ($value:expr, $pattern:pat => $result:expr) => {
        match $value {
            StateTransition::DataContractCreate($pattern) => $result,
            StateTransition::DataContractUpdate($pattern) => $result,
            StateTransition::DocumentsBatch($pattern) => $result,
            StateTransition::IdentityCreate($pattern) => $result,
            StateTransition::IdentityTopUp($pattern) => $result,
            StateTransition::IdentityCreditWithdrawal($pattern) => $result,
            StateTransition::IdentityUpdate($pattern) => $result,
        }
    };
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AbstractStateTransitionOptions {
    skip_signature: Option<bool>,
    skip_identifiers_conversion: Option<bool>,
}

#[wasm_bindgen(js_name = AbstractStateTransition)]
pub struct StateTransitionWasm(StateTransition);

#[wasm_bindgen(js_class = AbstractStateTransition)]
impl StateTransitionWasm {
    #[wasm_bindgen(js_name = getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        either_st!(&self.0, st => st.get_protocol_version())
    }

    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> u8 {
        either_st!(&self.0, st => st.get_type().into())
    }

    #[wasm_bindgen(js_name = getSignature)]
    pub fn get_signature(&self) -> Buffer {
        either_st!(&self.0, st => Buffer::from_bytes(st.get_signature()))
    }

    #[wasm_bindgen(js_name = setSignature)]
    pub fn set_signature(&self, signature: Vec<u8>) {
        either_st!(&self.0, st => st.set_signature(signature))
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options: AbstractStateTransitionOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        either_st!(&self.0, st => st.to_object(options.skip_signature.unwrap_or_default()).map_err(from_dpp_err))
    }

    #[wasm_bindgen(js_name = getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        either_st!(&self.0, st => (*st.get_owner_id()).into())
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self, options: JsValue) -> Result<JsValue, JsValue> {
        // TODO: construct options
        either_st!(&self.0, st => st.to_json(false).map_err(from_dpp_err))
    }

    #[wasm_bindgen(js_name = toBuffer)]
    pub fn to_buffer(&self, options: JsValue) -> Result<Buffer, JsValue> {
        // TODO: construct options
        either_st!(&self.0, st => st.to_buffer(false).map_err(from_dpp_err))
    }

    #[wasm_bindgen(js_name = hash)]
    pub fn hash(&self, options: JsValue) -> Result<Buffer, JsValue> {
        // TODO: construct options
        either_st!(&self.0, st => st.hash(false).map_err(from_dpp_err))
    }

    #[wasm_bindgen(js_name = signByPrivateKey)]
    pub fn sign_by_private_key(&self, private_key: Vec<u8>, key_type: u8) -> Result<(), JsValue> {
        either_st!(&self.0, st => st
                .sign_by_private_key(
                    &private_key,
                    KeyType::try_from(key_type).map_err(from_dpp_err)?,
                    todo!("pass correct BLS"),
                )
                .map_err(from_dpp_err))
    }

    #[wasm_bindgen(js_name = verifyByPublicKey)]
    pub fn verify_by_public_key(&self, public_key: Vec<u8>, key_type: u8) -> Result<bool, JsValue> {
        either_st!(&self.0, st => st.verify_by_public_key(
                &public_key,
                KeyType::try_from(key_type).map_err(from_dpp_err)?,
                todo!("set up BLS")))
    }

    // #[wasm_bindgen(js_name = getModifiedDataIds)]
    // pub fn get_modified_data_ids(&self) -> Vec<IdentifierWrapper> {
    //     // TODO: implement!
    //     vec![];
    // }

    #[wasm_bindgen(js_name = isDocumentStateTransition)]
    pub fn is_document_state_transition(&self) -> bool {
        either_st!(&self.0, st => st.is_document_state_transition())
    }

    #[wasm_bindgen(js_name = isDataContractStateTransition)]
    pub fn is_data_contract_state_transition(&self) -> bool {
        either_st!(&self.0, st => st.is_data_contract_state_transition())
    }

    #[wasm_bindgen(js_name = isIdentityStateTransition)]
    pub fn is_identity_state_transition(&self) -> bool {
        either_st!(&self.0, st => st.is_identity_state_transition())
    }

    #[wasm_bindgen(js_name = setExecutionContext)]
    pub fn set_execution_context(&self, execution_context: StateTransitionExecutionContextWasm) {
        either_st!(&self.0, st => st.set_execution_context(execution_context.into()))
    }

    #[wasm_bindgen(js_name = getExecutionContext)]
    pub fn get_execution_context(&self) -> StateTransitionExecutionContextWasm {
        either_st!(&self.0, st => st.get_execution_context().into())
    }
}

impl From<StateTransition> for StateTransitionWasm {
    fn from(value: StateTransition) -> Self {
        value.into()
    }
}

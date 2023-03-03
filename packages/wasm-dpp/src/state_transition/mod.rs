use std::{convert::TryFrom, sync::Arc};

use dpp::{
    identity::KeyType,
    prelude::Identifier,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext, StateTransition,
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
    },
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{
    bls_adapter::BlsAdapter, buffer::Buffer, errors::from_dpp_err, identifier::IdentifierWrapper,
    with_js_error,
};

pub mod errors;
pub mod state_transition_factory;

pub mod validation;
pub use validation::*;

pub(crate) mod conversion;

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
pub struct StateTransitionWasm {
    inner: StateTransition,
    bls_adapter: Arc<BlsAdapter>,
}

#[wasm_bindgen(js_class = AbstractStateTransition)]
impl StateTransitionWasm {
    #[wasm_bindgen(js_name = getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        either_st!(&self.inner, st => st.get_protocol_version())
    }

    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> u8 {
        either_st!(&self.inner, st => st.get_type().into())
    }

    #[wasm_bindgen(js_name = getSignature)]
    pub fn get_signature(&self) -> Buffer {
        either_st!(&self.inner, st => Buffer::from_bytes(st.get_signature()))
    }

    #[wasm_bindgen(js_name = setSignature)]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        let inner = &mut self.inner;
        either_st!(inner, st => st.set_signature(signature))
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options: AbstractStateTransitionOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        either_st!(&self.inner, st => {
            self.inner.serialize(&serializer).map_err(|e| e.into())
        })
    }

    #[wasm_bindgen(js_name = getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        either_st!(&self.inner, st => (*st.get_owner_id()).into())
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options: AbstractStateTransitionOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        either_st!(&self.inner, st => {
            let json_value = st
              .to_json(options.skip_signature.unwrap_or_default())
              .map_err(from_dpp_err)?;

            serde_wasm_bindgen::to_value(&json_value).map_err(|e| e.into())
        })
    }

    #[wasm_bindgen(js_name = toBuffer)]
    pub fn to_buffer(&self, options: JsValue) -> Result<Buffer, JsValue> {
        let options: AbstractStateTransitionOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        either_st!(&self.inner, st => st.to_buffer(options.skip_signature.unwrap_or_default()).map(|v| Buffer::from_bytes(&v)).map_err(from_dpp_err))
    }

    #[wasm_bindgen(js_name = hash)]
    pub fn hash(&self, options: JsValue) -> Result<Buffer, JsValue> {
        let options: AbstractStateTransitionOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        either_st!(&self.inner, st => st.hash(options.skip_signature.unwrap_or_default()).map(|v| Buffer::from_bytes(&v)).map_err(from_dpp_err))
    }

    #[wasm_bindgen(js_name = signByPrivateKey)]
    pub fn sign_by_private_key(
        &mut self,
        private_key: Vec<u8>,
        key_type: u8,
    ) -> Result<(), JsValue> {
        let inner = &mut self.inner;
        either_st!(inner, st => st
                .sign_by_private_key(
                    &private_key,
                    KeyType::try_from(key_type).map_err(|e| e.to_string())?,
                    self.bls_adapter.as_ref(),
                )
                .map_err(from_dpp_err))
    }

    #[wasm_bindgen(js_name = verifyByPublicKey)]
    pub fn verify_by_public_key(&self, public_key: Vec<u8>, key_type: u8) -> Result<bool, JsValue> {
        either_st!(&self.inner, st => st
                   .verify_by_public_key(
                     &public_key,
                     KeyType::try_from(key_type).map_err(|e| e.to_string())?,
                     self.bls_adapter.as_ref()).map(|_| true).map_err(from_dpp_err))
    }

    #[wasm_bindgen(js_name = getModifiedDataIds)]
    pub fn get_modified_data_ids(&self) -> Vec<JsValue> {
        either_st!(&self.inner, st => st.get_modified_data_ids().into_iter().map(|id| <IdentifierWrapper as std::convert::From<Identifier>>::from(id).into()).collect())
    }

    #[wasm_bindgen(js_name = isDocumentStateTransition)]
    pub fn is_document_state_transition(&self) -> bool {
        either_st!(&self.inner, st => st.is_document_state_transition())
    }

    #[wasm_bindgen(js_name = isDataContractStateTransition)]
    pub fn is_data_contract_state_transition(&self) -> bool {
        either_st!(&self.inner, st => st.is_data_contract_state_transition())
    }

    #[wasm_bindgen(js_name = isIdentityStateTransition)]
    pub fn is_identity_state_transition(&self) -> bool {
        either_st!(&self.inner, st => st.is_identity_state_transition())
    }

    #[wasm_bindgen(js_name = setExecutionContext)]
    pub fn set_execution_context(
        &mut self,
        execution_context: StateTransitionExecutionContextWasm,
    ) {
        let inner = &mut self.inner;
        either_st!(inner, st => st.set_execution_context(execution_context.into()))
    }

    #[wasm_bindgen(js_name = getExecutionContext)]
    pub fn get_execution_context(&self) -> StateTransitionExecutionContextWasm {
        either_st!(&self.inner, st => st.get_execution_context().into())
    }
}

impl From<StateTransition> for StateTransitionWasm {
    fn from(value: StateTransition) -> Self {
        value.into()
    }
}

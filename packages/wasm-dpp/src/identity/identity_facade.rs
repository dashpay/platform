use std::sync::Arc;

use wasm_bindgen::prelude::*;

use dpp::identity::validation::PublicKeysValidator;
use dpp::identity::IdentityFacade;

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::validation::ValidationResultWasm;
use dpp::version::ProtocolVersionValidator;
use dpp::NonConsensusError;

#[wasm_bindgen(js_name=IdentityFacade)]
pub struct IdentityFacadeWasm(IdentityFacade<BlsAdapter>);

#[wasm_bindgen(js_class=IdentityFacade)]
impl IdentityFacadeWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(bls_adapter: JsBlsAdapter) -> IdentityFacadeWasm {
        let bls = BlsAdapter(bls_adapter);
        // TODO: REMOVE THAT LINE, TAKE IT AS AN ARGUMENT
        let protocol_version_validator = ProtocolVersionValidator::default();
        let public_keys_validator = PublicKeysValidator::new(bls).unwrap();
        let identity_facade = IdentityFacade::new(
            Arc::new(protocol_version_validator),
            Arc::new(public_keys_validator),
        )
        .unwrap();

        IdentityFacadeWasm(identity_facade)
    }

    #[wasm_bindgen()]
    pub fn validate(
        &self,
        raw_identity_object: JsValue,
    ) -> Result<ValidationResultWasm, NonConsensusErrorWasm> {
        let identity_json = serde_wasm_bindgen::from_value(raw_identity_object)
            .expect("unable to serialize identity");
        // TODO: handle the case when
        let validation_result = self.0.validate(&identity_json)?;
        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}

#[wasm_bindgen(js_name=NonConsensusErrorWasm)]
pub struct NonConsensusErrorWasm(NonConsensusError);

impl From<NonConsensusError> for NonConsensusErrorWasm {
    fn from(err: NonConsensusError) -> Self {
        Self(err)
    }
}

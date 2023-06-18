use dpp::identity::state_transition::identity_credit_transfer_transition::validation::basic::identity_credit_transfer_basic::IdentityCreditTransferTransitionBasicValidator;

use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
use dpp::ProtocolError;

use wasm_bindgen::prelude::*;

use crate::errors::from_dpp_err;
use crate::utils::ToSerdeJSONExt;
use crate::validation::ValidationResultWasm;

#[wasm_bindgen(js_name=IdentityCreditTransferTransitionBasicValidator)]
pub struct IdentityCreditTransferTransitionBasicValidatorWasm(
    IdentityCreditTransferTransitionBasicValidator,
);

impl From<IdentityCreditTransferTransitionBasicValidator>
    for IdentityCreditTransferTransitionBasicValidatorWasm
{
    fn from(validator: IdentityCreditTransferTransitionBasicValidator) -> Self {
        Self(validator)
    }
}

#[wasm_bindgen(js_class=IdentityCreditTransferTransitionBasicValidator)]
impl IdentityCreditTransferTransitionBasicValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<IdentityCreditTransferTransitionBasicValidatorWasm, JsValue> {
        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );

        let validator =
            IdentityCreditTransferTransitionBasicValidator::new(protocol_version_validator)
                .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

        Ok(validator.into())
    }

    #[wasm_bindgen]
    pub async fn validate(
        mut self,
        raw_state_transition: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let current_protocol_version =
            js_sys::Reflect::get(&raw_state_transition, &"protocolVersion".into())
                .map(|v| v.as_f64().unwrap_or(LATEST_VERSION as f64) as u32)
                .unwrap_or(LATEST_VERSION);

        self.0
            .protocol_version_validator()
            .set_current_protocol_version(current_protocol_version);

        let state_transition_object = raw_state_transition.with_serde_to_platform_value()?;

        let validation_result = self
            .0
            .validate(&state_transition_object)
            .await
            .map_err(|e| from_dpp_err(e.into()))?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}

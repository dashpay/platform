use dpp::identity::state_transition::identity_update_transition::{
    validate_identity_update_transition_basic::ValidateIdentityUpdateTransitionBasic,
    validate_public_keys::IdentityUpdatePublicKeysValidator,
};
use dpp::identity::state_transition::validate_public_key_signatures::PublicKeysSignaturesValidator;

use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
use dpp::ProtocolError;

use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::errors::from_dpp_err;

use crate::utils::ToSerdeJSONExt;
use crate::validation::ValidationResultWasm;

#[wasm_bindgen(js_name=IdentityUpdateTransitionBasicValidator)]
pub struct IdentityUpdateTransitionBasicValidatorWasm(
    ValidateIdentityUpdateTransitionBasic<
        IdentityUpdatePublicKeysValidator,
        PublicKeysSignaturesValidator<BlsAdapter>,
    >,
);

impl
    From<
        ValidateIdentityUpdateTransitionBasic<
            IdentityUpdatePublicKeysValidator,
            PublicKeysSignaturesValidator<BlsAdapter>,
        >,
    > for IdentityUpdateTransitionBasicValidatorWasm
{
    fn from(
        validator: ValidateIdentityUpdateTransitionBasic<
            IdentityUpdatePublicKeysValidator,
            PublicKeysSignaturesValidator<BlsAdapter>,
        >,
    ) -> Self {
        Self(validator)
    }
}

#[wasm_bindgen(js_class=IdentityUpdateTransitionBasicValidator)]
impl IdentityUpdateTransitionBasicValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        js_bls: JsBlsAdapter,
    ) -> Result<IdentityUpdateTransitionBasicValidatorWasm, JsValue> {
        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );

        let public_keys_validator = IdentityUpdatePublicKeysValidator {};

        let public_keys_signatures_validator =
            PublicKeysSignaturesValidator::new(BlsAdapter(js_bls));

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(public_keys_validator),
            Arc::new(public_keys_signatures_validator),
        )
        .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

        Ok(validator.into())
    }

    #[wasm_bindgen(js_name=validate)]
    pub fn validate(
        &mut self,
        raw_state_transition: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let current_protocol_version =
            js_sys::Reflect::get(&raw_state_transition, &"protocolVersion".into())
                .map(|v| v.as_f64().unwrap_or(LATEST_VERSION as f64) as u32)
                .unwrap_or(LATEST_VERSION);

        self.0
            .protocol_version_validator()
            .set_current_protocol_version(current_protocol_version);

        let state_transition_json = raw_state_transition.with_serde_to_platform_value()?;

        let validation_result = self
            .0
            .validate(&state_transition_json)
            .map_err(|e| from_dpp_err(e.into()))?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}

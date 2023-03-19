use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};

use crate::errors::from_dpp_err;
use crate::utils::with_serde_to_platform_value;
use crate::validation::ValidationResultWasm;
use dpp::identity::validation::{IdentityValidator, PublicKeysValidator};
use dpp::version::ProtocolVersionValidator;
use std::sync::Arc;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen(js_name=IdentityValidator)]
pub struct IdentityValidatorWasm(IdentityValidator<PublicKeysValidator<BlsAdapter>>);

impl From<IdentityValidator<PublicKeysValidator<BlsAdapter>>> for IdentityValidatorWasm {
    fn from(v: IdentityValidator<PublicKeysValidator<BlsAdapter>>) -> Self {
        Self(v)
    }
}

impl From<IdentityValidatorWasm> for IdentityValidator<PublicKeysValidator<BlsAdapter>> {
    fn from(v: IdentityValidatorWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class=IdentityValidator)]
impl IdentityValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(bls: JsBlsAdapter) -> Result<IdentityValidatorWasm, JsError> {
        Ok(IdentityValidator::new(
            Arc::new(ProtocolVersionValidator::default()),
            Arc::new(PublicKeysValidator::new(BlsAdapter(bls))?),
        )?
        .into())
    }

    #[wasm_bindgen]
    pub fn validate(&self, raw_identity: JsValue) -> Result<ValidationResultWasm, JsValue> {
        let raw_identity = with_serde_to_platform_value(&raw_identity)?;
        let result = self
            .0
            .validate_identity_object(&raw_identity)
            .map_err(|e| from_dpp_err(e.into()))?;

        Ok(result.map(|_| JsValue::undefined()).into())
    }
}

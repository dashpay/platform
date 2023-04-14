use std::convert::TryFrom;

use anyhow::anyhow;
use dpp::{
    identity::KeyType,
    state_transition::fee::operations::{OperationLike, SignatureVerificationOperation},
    ProtocolError,
};
use js_sys::{Array, BigInt};
use wasm_bindgen::prelude::*;

use crate::{
    fee::refunds::RefundsWasm,
    utils::{Inner, WithJsError},
};

#[wasm_bindgen(js_name = "SignatureVerificationOperation")]
#[derive(Clone)]
pub struct SignatureVerificationOperationWasm(SignatureVerificationOperation);

impl From<SignatureVerificationOperation> for SignatureVerificationOperationWasm {
    fn from(value: SignatureVerificationOperation) -> Self {
        SignatureVerificationOperationWasm(value)
    }
}

impl From<SignatureVerificationOperationWasm> for SignatureVerificationOperation {
    fn from(value: SignatureVerificationOperationWasm) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_class=SignatureVerificationOperation)]
impl SignatureVerificationOperationWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(signature_type: u8) -> Result<SignatureVerificationOperationWasm, JsValue> {
        let key_type = KeyType::try_from(signature_type)
            .map_err(|e| anyhow!("invalid key type: {}", e))
            .with_js_error()?;

        Ok(SignatureVerificationOperation::new(key_type).into())
    }

    #[wasm_bindgen(js_name = getProcessingCost)]
    pub fn get_processing_cost(&self) -> Result<BigInt, JsValue> {
        Ok(BigInt::from(
            self.0
                .get_processing_cost()
                .map_err(ProtocolError::from)
                .with_js_error()?,
        ))
    }

    #[wasm_bindgen(js_name=getStorageCost)]
    pub fn get_storage_cost(&self) -> Result<BigInt, JsValue> {
        Ok(BigInt::from(
            self.0
                .get_storage_cost()
                .map_err(ProtocolError::from)
                .with_js_error()?,
        ))
    }

    #[wasm_bindgen(getter)]
    pub fn refunds(&self) -> Option<Array> {
        let array_refunds = Array::new();
        if let Some(refunds) = self.0.get_refunds() {
            for refund in refunds {
                let refund_wasm: RefundsWasm = refund.into();
                array_refunds.push(&refund_wasm.into());
            }
            Some(array_refunds)
        } else {
            None
        }
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = js_sys::Object::new();

        js_sys::Reflect::set(&json, &"type".into(), &"signatureVerification".into())?;

        js_sys::Reflect::set(
            &json,
            &"signatureType".into(),
            &JsValue::from(self.0.signature_type as u8),
        )?;

        Ok(json.into())
    }
}

impl Inner for SignatureVerificationOperationWasm {
    type InnerItem = SignatureVerificationOperation;

    fn into_inner(self) -> Self::InnerItem {
        self.0
    }

    fn inner(&self) -> &Self::InnerItem {
        &self.0
    }

    fn inner_mut(&mut self) -> &mut Self::InnerItem {
        &mut self.0
    }
}

use std::convert::TryFrom;

use anyhow::anyhow;
use dpp::{
    identity::KeyType,
    state_transition::fee::operations::{OperationLike, SignatureVerificationOperation},
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
    pub fn get_processing_cost(&self) -> BigInt {
        BigInt::from(self.0.get_processing_cost())
    }

    #[wasm_bindgen(js_name=getStorageCost)]
    pub fn get_storage_cost(&self) -> BigInt {
        BigInt::from(self.0.get_storage_cost())
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

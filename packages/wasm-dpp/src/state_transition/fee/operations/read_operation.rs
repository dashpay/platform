use dpp::{
    state_transition::fee::operations::{OperationLike, ReadOperation},
    ProtocolError,
};
use js_sys::{Array, BigInt};
use wasm_bindgen::prelude::*;

use crate::{
    fee::refunds::RefundsWasm,
    utils::{try_to_u64, Inner, WithJsError},
};

#[wasm_bindgen(js_name = "ReadOperation")]
#[derive(Clone)]
pub struct ReadOperationWasm(ReadOperation);

impl From<ReadOperation> for ReadOperationWasm {
    fn from(value: ReadOperation) -> Self {
        ReadOperationWasm(value)
    }
}

impl From<ReadOperationWasm> for ReadOperation {
    fn from(value: ReadOperationWasm) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_class=ReadOperation)]
impl ReadOperationWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(value_size: JsValue) -> Result<ReadOperationWasm, JsValue> {
        let value_size = try_to_u64(value_size).with_js_error()?;
        Ok(ReadOperation::new(value_size).into())
    }

    #[wasm_bindgen(getter,js_name = processingCost)]
    pub fn processing_cost(&self) -> Result<BigInt, JsValue> {
        Ok(BigInt::from(
            self.0
                .get_processing_cost()
                .map_err(ProtocolError::from)
                .with_js_error()?,
        ))
    }

    #[wasm_bindgen(getter, js_name=storageCost)]
    pub fn storage_cost(&self) -> Result<BigInt, JsValue> {
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

        js_sys::Reflect::set(
            &json,
            &JsValue::from_str("type"),
            &JsValue::from_str("read"),
        )?;

        js_sys::Reflect::set(
            &json,
            &JsValue::from_str("valueSize"),
            &JsValue::from(self.0.value_size),
        )?;

        Ok(json.into())
    }
}

impl Inner for ReadOperationWasm {
    type InnerItem = ReadOperation;

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

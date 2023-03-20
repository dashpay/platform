use dpp::state_transition::fee::operations::{OperationLike, WriteOperation};
use js_sys::BigInt;
use wasm_bindgen::prelude::*;

use crate::utils::{try_to_u64, Inner, WithJsError};

#[wasm_bindgen(js_name = "WriteOperation")]
#[derive(Clone)]
pub struct WriteOperationWasm(WriteOperation);

impl From<WriteOperation> for WriteOperationWasm {
    fn from(value: WriteOperation) -> Self {
        WriteOperationWasm(value)
    }
}

#[wasm_bindgen(js_class=WriteOperation)]
impl WriteOperationWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(value_size: JsValue, key_size: JsValue) -> Result<WriteOperationWasm, JsValue> {
        let value_size = try_to_u64(value_size).with_js_error()?;
        let key_size = try_to_u64(key_size).with_js_error()?;

        // TODO remove `as usize`
        Ok(WriteOperation::new(value_size as usize, key_size as usize).into())
    }

    #[wasm_bindgen(js_name = getProcessingCost)]
    pub fn get_processing_cost(&self) -> BigInt {
        BigInt::from(self.0.get_processing_cost())
    }

    #[wasm_bindgen(js_name=getStorageCost)]
    pub fn get_storage_cost(&self) -> BigInt {
        BigInt::from(self.0.get_storage_cost())
    }
}

impl Inner for WriteOperationWasm {
    type InnerItem = WriteOperation;

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

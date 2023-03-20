use dpp::state_transition::fee::operations::{OperationLike, ReadOperation};
use js_sys::BigInt;
use wasm_bindgen::prelude::*;

use crate::utils::{try_to_u64, Inner, WithJsError};

#[wasm_bindgen(js_name = "ReadOperation")]
#[derive(Clone)]
pub struct ReadOperationWasm(ReadOperation);

impl From<ReadOperation> for ReadOperationWasm {
    fn from(value: ReadOperation) -> Self {
        ReadOperationWasm(value)
    }
}

#[wasm_bindgen(js_class=ReadOperation)]
impl ReadOperationWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(value_size: JsValue) -> Result<ReadOperationWasm, JsValue> {
        let value_size = try_to_u64(value_size).with_js_error()?;

        // TODO remove `as usize`
        Ok(ReadOperation::new(value_size as usize).into())
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

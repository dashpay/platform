use std::convert::TryInto;

use anyhow::anyhow;
use dpp::state_transition::fee::operations::{OperationLike, PreCalculatedOperation};
use js_sys::BigInt;
use wasm_bindgen::prelude::*;

use crate::utils::{try_to_u64, Inner, WithJsError};

#[wasm_bindgen(js_name = "PreCalculatedOperation")]
#[derive(Clone)]
pub struct PreCalculatedOperationWasm(PreCalculatedOperation);

impl From<PreCalculatedOperation> for PreCalculatedOperationWasm {
    fn from(value: PreCalculatedOperation) -> Self {
        PreCalculatedOperationWasm(value)
    }
}

#[wasm_bindgen(js_class=PreCalculatedOperation)]
impl PreCalculatedOperationWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        storage_cost: JsValue,
        processing_cost: JsValue,
    ) -> Result<PreCalculatedOperationWasm, JsValue> {
        let storage_cost: i64 = try_to_u64(storage_cost)
            .with_js_error()?
            .try_into()
            .map_err(|e| anyhow!("unable convert storage cost to i64: {}", e))
            .with_js_error()?;

        let processing_cost: i64 = try_to_u64(processing_cost)
            .with_js_error()?
            .try_into()
            .map_err(|e| anyhow!("unable convert processing_cost to i64: {}", e))
            .with_js_error()?;

        Ok(PreCalculatedOperation::new(storage_cost, processing_cost).into())
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

impl Inner for PreCalculatedOperationWasm {
    type InnerItem = PreCalculatedOperation;

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

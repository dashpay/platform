use crate::{fee::dummy_fee_result::DummyFeesResultWasm, utils::Inner};
use dpp::state_transition::fee::{
    operations::{OperationLike, PreCalculatedOperation},
    Refunds,
};
use js_sys::{Array, BigInt};
use wasm_bindgen::prelude::*;

use crate::{
    fee::refunds::RefundsWasm,
    utils::{try_to_u64, IntoWasm, WithJsError},
};

#[wasm_bindgen(js_name = "PreCalculatedOperation")]
#[derive(Clone)]
pub struct PreCalculatedOperationWasm(PreCalculatedOperation);

impl From<PreCalculatedOperation> for PreCalculatedOperationWasm {
    fn from(value: PreCalculatedOperation) -> Self {
        PreCalculatedOperationWasm(value)
    }
}

impl From<PreCalculatedOperationWasm> for PreCalculatedOperation {
    fn from(value: PreCalculatedOperationWasm) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_class=PreCalculatedOperation)]
impl PreCalculatedOperationWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        storage_cost: JsValue,
        processing_cost: JsValue,
        js_fee_refunds: Array,
    ) -> Result<PreCalculatedOperationWasm, JsValue> {
        let storage_cost = try_to_u64(storage_cost).with_js_error()?;
        let processing_cost = try_to_u64(processing_cost).with_js_error()?;

        let mut refunds = vec![];
        for refund in js_fee_refunds.iter() {
            let transition: Refunds = refund
                .to_wasm::<RefundsWasm>("Refunds")?
                .to_owned()
                .into_inner();
            refunds.push(transition);
        }

        Ok(PreCalculatedOperation::new(storage_cost, processing_cost, refunds).into())
    }

    #[wasm_bindgen(js_name=fromFee)]
    pub fn from_fee(dummy_fee_result: &DummyFeesResultWasm) -> PreCalculatedOperationWasm {
        let operation = PreCalculatedOperation::from_fee(dummy_fee_result.inner().clone());
        PreCalculatedOperationWasm(operation)
    }

    #[wasm_bindgen(js_name = getProcessingCost)]
    pub fn processing_cost(&self) -> BigInt {
        BigInt::from(self.0.get_processing_cost())
    }

    #[wasm_bindgen(js_name=getStorageCost)]
    pub fn storage_cost(&self) -> BigInt {
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

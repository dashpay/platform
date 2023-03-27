use dpp::state_transition::fee::{DummyFeesResult, Refunds};
use js_sys::{Array, BigInt};
use wasm_bindgen::prelude::*;

use crate::{
    fee::refunds::RefundsWasm,
    utils::{try_to_u64, Inner, IntoWasm, WithJsError},
};

#[wasm_bindgen(js_name=DummyFeesResult)]
pub struct DummyFeesResultWasm(DummyFeesResult);

#[wasm_bindgen(js_class=DummyFeesResult)]
impl DummyFeesResultWasm {
    #[wasm_bindgen(getter, js_name = "storageFee")]
    pub fn storage_fee(&self) -> BigInt {
        BigInt::from(self.0.storage)
    }

    #[wasm_bindgen(getter, js_name = "processingFee")]
    pub fn processing_fee(&self) -> BigInt {
        BigInt::from(self.0.processing)
    }

    #[wasm_bindgen(getter, js_name = "feeRefunds")]
    pub fn fee_refunds(&self) -> js_sys::Array {
        let js_refunds = js_sys::Array::new();
        for refund in self.0.fee_refunds.iter().map(RefundsWasm::from) {
            js_refunds.push(&refund.into());
        }
        js_refunds
    }

    #[wasm_bindgen(setter, js_name = "storageFee")]
    pub fn set_storage_fee(&mut self, number: JsValue) -> Result<(), JsValue> {
        let number = try_to_u64(number).with_js_error()?;
        self.0.storage = number;
        Ok(())
    }

    #[wasm_bindgen(setter, js_name = "processingFee")]
    pub fn set_processing_fee(&mut self, number: JsValue) -> Result<(), JsValue> {
        let number = try_to_u64(number).with_js_error()?;
        self.0.processing = number;
        Ok(())
    }

    #[wasm_bindgen(setter, js_name = "feeRefunds")]
    pub fn set_fee_refunds(&mut self, js_fee_refunds: Array) -> Result<(), JsValue> {
        let mut refunds = vec![];
        for refund in js_fee_refunds.iter() {
            let transition: Refunds = refund
                .to_wasm::<RefundsWasm>("Refunds")?
                .to_owned()
                .into_inner();
            refunds.push(transition);
        }
        self.0.fee_refunds = refunds;
        Ok(())
    }
}

impl From<DummyFeesResult> for DummyFeesResultWasm {
    fn from(value: DummyFeesResult) -> Self {
        DummyFeesResultWasm(value)
    }
}

impl Inner for DummyFeesResultWasm {
    type InnerItem = DummyFeesResult;

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

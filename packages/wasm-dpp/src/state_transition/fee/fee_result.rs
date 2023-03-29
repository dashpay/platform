use dpp::state_transition::fee::{FeeResult, Refunds};
use js_sys::BigInt;
use wasm_bindgen::prelude::*;

use crate::{
    fee::refunds::RefundsWasm,
    utils::{try_to_u64, Inner, IntoWasm, WithJsError},
};

#[wasm_bindgen(js_name=FeeResult)]
pub struct FeeResultWasm(FeeResult);

#[wasm_bindgen(js_class=FeeResult)]
impl FeeResultWasm {
    #[allow(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> FeeResultWasm {
        FeeResultWasm(FeeResult::default())
    }

    #[wasm_bindgen(getter, js_name = "storageFee")]
    pub fn storage_fee(&self) -> BigInt {
        BigInt::from(self.0.storage_fee)
    }

    #[wasm_bindgen(getter, js_name = "processingFee")]
    pub fn processing_fee(&self) -> BigInt {
        BigInt::from(self.0.processing_fee)
    }

    #[wasm_bindgen(getter, js_name = "feeRefunds")]
    pub fn fee_refunds(&self) -> js_sys::Array {
        let js_refunds = js_sys::Array::new();
        for refund in self.0.fee_refunds.iter().map(RefundsWasm::from) {
            js_refunds.push(&refund.into());
        }
        js_refunds
    }

    #[wasm_bindgen(getter, js_name = "totalRefunds")]
    pub fn total_refunds(&self) -> BigInt {
        BigInt::from(self.0.total_refunds)
    }

    #[wasm_bindgen(getter, js_name = "desiredAmount")]
    pub fn desired_amount(&self) -> BigInt {
        BigInt::from(self.0.desired_amount)
    }

    #[wasm_bindgen(getter, js_name = "requiredAmount")]
    pub fn required_amount(&self) -> BigInt {
        BigInt::from(self.0.required_amount)
    }

    #[wasm_bindgen(setter, js_name = "storageFee")]
    pub fn set_storage_fee(&mut self, number: JsValue) -> Result<(), JsValue> {
        let number = try_to_u64(number).with_js_error()?;
        self.0.storage_fee = number;
        Ok(())
    }

    #[wasm_bindgen(setter, js_name = "processingFee")]
    pub fn set_processing_fee(&mut self, number: JsValue) -> Result<(), JsValue> {
        let number = try_to_u64(number).with_js_error()?;
        self.0.processing_fee = number;
        Ok(())
    }

    #[wasm_bindgen(setter, js_name = "feeRefunds")]
    pub fn set_fee_refunds(&mut self, js_fee_refunds: js_sys::Array) -> Result<(), JsValue> {
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

    #[wasm_bindgen(setter, js_name = "desiredAmount")]
    pub fn set_desired_amount(&mut self, number: JsValue) -> Result<(), JsValue> {
        let number = try_to_u64(number).with_js_error()?;
        self.0.desired_amount = number;
        Ok(())
    }

    #[wasm_bindgen(setter, js_name = "requiredAmount")]
    pub fn set_required_amount(&mut self, number: JsValue) -> Result<(), JsValue> {
        let number = try_to_u64(number).with_js_error()?;
        self.0.required_amount = number;
        Ok(())
    }

    #[wasm_bindgen(setter, js_name = "totalRefunds")]
    pub fn set_total_refunds(&mut self, number: JsValue) -> Result<(), JsValue> {
        let number = try_to_u64(number).with_js_error()?;
        self.0.total_refunds = number;
        Ok(())
    }
}

impl From<FeeResult> for FeeResultWasm {
    fn from(value: FeeResult) -> Self {
        FeeResultWasm(value)
    }
}

impl Inner for FeeResultWasm {
    type InnerItem = FeeResult;

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

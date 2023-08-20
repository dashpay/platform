use crate::{fee::dummy_fee_result::DummyFeesResultWasm, utils::Inner};
use dpp::fee::Credits;
use dpp::platform_value::Error as PlatformValueError;
use dpp::{
    state_transition::fee::{
        operations::{OperationLike, PreCalculatedOperation},
        Refunds,
    },
    ProtocolError,
};
use js_sys::{Array, BigInt};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use crate::errors::value_error::PlatformValueErrorWasm;
use crate::utils::ToSerdeJSONExt;
use crate::{
    fee::refunds::RefundsWasm,
    utils::{try_to_u64, WithJsError},
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
            let parsed_refund = refund.with_serde_to_platform_value()?;
            let identifier = parsed_refund
                .get_identifier("identifier")
                .map_err(PlatformValueErrorWasm::from)?;

            let mut credits_per_epoch: HashMap<String, Credits> = HashMap::new();
            if let Some(credits_per_epoch_value) = parsed_refund
                .get("creditsPerEpoch")
                .map_err(PlatformValueErrorWasm::from)?
            {
                let credits_per_epoch_map = credits_per_epoch_value.as_map().ok_or_else(|| {
                    let error =
                        PlatformValueError::PathError("Credits per epoch is not a map".to_string());
                    PlatformValueErrorWasm::from(error)
                })?;

                for (epoch, credits) in credits_per_epoch_map {
                    let epoch = epoch.to_str().map_err(PlatformValueErrorWasm::from)?;
                    let credits =
                        credits
                            .as_integer::<u64>()
                            .ok_or(PlatformValueErrorWasm::from(PlatformValueError::PathError(
                                "Credits per epoch is not an integer".to_string(),
                            )))?;

                    credits_per_epoch.insert(String::from(epoch), credits);
                }
            }
            refunds.push(Refunds {
                identifier,
                credits_per_epoch,
            });
        }

        Ok(PreCalculatedOperation::new(storage_cost, processing_cost, refunds).into())
    }

    #[wasm_bindgen(js_name=fromFee)]
    pub fn from_fee(dummy_fee_result: &DummyFeesResultWasm) -> PreCalculatedOperationWasm {
        let operation = PreCalculatedOperation::from_fee(dummy_fee_result.inner().clone());
        PreCalculatedOperationWasm(operation)
    }

    #[wasm_bindgen(js_name = getProcessingCost)]
    pub fn processing_cost(&self) -> Result<BigInt, JsValue> {
        Ok(BigInt::from(
            self.0
                .get_processing_cost()
                .map_err(ProtocolError::from)
                .with_js_error()?,
        ))
    }

    #[wasm_bindgen(js_name=getStorageCost)]
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

    pub fn refunds_as_objects(&self) -> Result<Option<Array>, JsValue> {
        let array_refunds = Array::new();
        if let Some(refunds) = self.0.get_refunds() {
            for refund in refunds {
                let refund_wasm: RefundsWasm = refund.into();
                array_refunds.push(&refund_wasm.to_object()?);
            }
            Ok(Some(array_refunds))
        } else {
            Ok(None)
        }
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = js_sys::Object::new();

        js_sys::Reflect::set(
            &json,
            &JsValue::from_str("type"),
            &JsValue::from_str("preCalculated"),
        )?;

        js_sys::Reflect::set(
            &json,
            &JsValue::from_str("storageCost"),
            &JsValue::from(self.storage_cost()?),
        )?;

        js_sys::Reflect::set(
            &json,
            &JsValue::from_str("processingCost"),
            &JsValue::from(self.processing_cost()?),
        )?;

        js_sys::Reflect::set(
            &json,
            &JsValue::from_str("feeRefunds"),
            &JsValue::from(self.refunds_as_objects()?.unwrap_or(Array::new())),
        )?;

        Ok(json.into())
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

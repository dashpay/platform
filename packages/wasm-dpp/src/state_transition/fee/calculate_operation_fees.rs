use dpp::state_transition::fee::{calculate_operations_fees, operations::Operation};
use wasm_bindgen::prelude::*;

use crate::{
    fee::FeesWasm,
    utils::{Inner, IntoWasm},
};

use super::OperationWasm;

#[wasm_bindgen(js_name=calculateOperationFees)]
pub fn calculate_operation_fees_wasm(operations: js_sys::Array) -> Result<FeesWasm, JsValue> {
    let mut inner_operations: Vec<Operation> = vec![];
    for operation in operations.iter() {
        let operation = operation.to_wasm::<OperationWasm>("Operation")?.to_owned();
        inner_operations.push(operation.into_inner())
    }

    Ok(calculate_operations_fees(inner_operations).into())
}

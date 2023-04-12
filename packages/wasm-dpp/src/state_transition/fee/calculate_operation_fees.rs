use dpp::{
    state_transition::fee::{
        calculate_operation_fees::calculate_operation_fees, operations::Operation,
    },
    ProtocolError,
};
use wasm_bindgen::prelude::*;

use crate::{fee::dummy_fee_result::DummyFeesResultWasm, utils::WithJsError};

use crate::state_transition::conversion::create_operation_from_wasm_instance;

#[wasm_bindgen(js_name=calculateOperationFees)]
pub fn calculate_operation_fees_wasm(
    operations: js_sys::Array,
) -> Result<DummyFeesResultWasm, JsValue> {
    let mut inner_operations: Vec<Operation> = vec![];
    for operation in operations.iter() {
        let operation = create_operation_from_wasm_instance(&operation)?;
        inner_operations.push(operation);
    }

    Ok(calculate_operation_fees(&inner_operations)
        .map_err(ProtocolError::from)
        .with_js_error()?
        .into())
}

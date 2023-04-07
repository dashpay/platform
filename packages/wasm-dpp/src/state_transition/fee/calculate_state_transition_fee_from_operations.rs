use dpp::state_transition::fee::calculate_state_transition_fee_from_operations_factory::calculate_state_transition_fee_from_operations;
use dpp::state_transition::fee::operations::Operation;
use dpp::ProtocolError;
use wasm_bindgen::prelude::*;

use crate::fee::fee_result::FeeResultWasm;
use crate::identifier::IdentifierWrapper;
use crate::state_transition::conversion::create_operation_from_wasm_instance;
use crate::utils::WithJsError;

#[wasm_bindgen(js_name=calculateStateTransitionFeeFromOperations)]
pub fn calculate_state_transition_fee_from_operations_wasm(
    operations: js_sys::Array,
    identity_id: &IdentifierWrapper,
) -> Result<FeeResultWasm, JsValue> {
    let mut inner_operations: Vec<Operation> = vec![];
    for operation in operations.iter() {
        let operation = create_operation_from_wasm_instance(&operation)?;
        inner_operations.push(operation);
    }

    Ok(calculate_state_transition_fee_from_operations(
        &inner_operations,
        &(identity_id.to_owned()).into(),
    )
    .map_err(ProtocolError::from)
    .with_js_error()?
    .into())
}

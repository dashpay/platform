use dpp::state_transition::fee::operations::Operation;
use wasm_bindgen::prelude::*;

pub use operations::*;

use crate::utils::Inner;
mod calculate_operation_fees;
mod calculate_state_transition_fee;
mod calculate_state_transition_fee_from_operations;
mod dummy_fee_result;
mod fee_result;
mod operations;
mod refunds;

pub use operations::*;

#[wasm_bindgen(js_name=Operation)]
#[derive(Clone)]
pub struct OperationWasm(Operation);

impl From<Operation> for OperationWasm {
    fn from(value: Operation) -> Self {
        OperationWasm(value)
    }
}

impl Inner for OperationWasm {
    type InnerItem = Operation;

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

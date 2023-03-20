use dpp::state_transition::fee::{calculate_operations_fees, operations::Operation, Fees};
use wasm_bindgen::prelude::*;

use crate::utils::Inner;
mod calculate_operation_fees;
mod calculate_state_transition_fee;
mod operations;

#[wasm_bindgen(js_name=Fees)]
pub struct FeesWasm(Fees);

#[wasm_bindgen(js_name=Operation)]
#[derive(Clone)]
pub struct OperationWasm(Operation);

impl From<Operation> for OperationWasm {
    fn from(value: Operation) -> Self {
        OperationWasm(value)
    }
}

impl From<Fees> for FeesWasm {
    fn from(value: Fees) -> Self {
        FeesWasm(value)
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

use std::borrow::Borrow;

use self::{
    constants::FEE_MULTIPLIER,
    operations::{Operation, OperationLike},
};

pub mod calculate_state_transition_fee;
pub mod constants;
pub mod operations;

#[derive(Default)]
pub struct Fees {
    storage: i64,
    processing: i64,
}

pub fn calculate_operations_fees(
    operations: impl IntoIterator<Item = impl Borrow<Operation>>,
) -> Fees {
    let mut fees = Fees::default();

    for operation in operations.into_iter() {
        let operation = operation.borrow();
        fees.processing += operation.get_processing_cost();
        fees.storage += operation.get_storage_cost();
    }

    fees.storage *= FEE_MULTIPLIER;
    fees.processing *= FEE_MULTIPLIER;

    fees
}

use self::operations::{Operation, OperationLike, PreCalculatedOperation};

pub mod calculate_state_transition_fee;
pub mod constants;
pub mod operations;

pub fn calculate_operations_costs<'a>(
    operations: impl IntoIterator<Item = &'a Operation>,
) -> PreCalculatedOperation {
    let mut storage_cost: i64 = 0;
    let mut processing_cost: i64 = 0;

    for operation in operations.into_iter() {
        processing_cost += operation.get_processing_cost();
        storage_cost += operation.get_storage_cost();
    }

    PreCalculatedOperation::new(storage_cost, processing_cost)
}

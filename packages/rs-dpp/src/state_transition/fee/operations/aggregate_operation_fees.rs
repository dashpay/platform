use crate::state_transition::fee::operations::Operation;
use crate::state_transition::fee::ExecutionFees;
use crate::ProtocolError;

pub fn aggregate_operation_fees(operations: &[Operation]) -> Result<ExecutionFees, ProtocolError> {
    let mut fee_result = ExecutionFees::default();

    for operation in operations {
        fee_result.checked_add_assign(operation.into())?;
    }

    Ok(fee_result)
}

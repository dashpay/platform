use dpp::{
    prelude::{DataContract, Identifier},
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};
use drive::drive::Drive;

#[derive(Clone, Debug)]
pub struct DataTriggerExecutionContext<'a> {
    pub drive: &'a Drive,
    pub owner_id: &'a Identifier,
    pub data_contract: &'a DataContract,
    pub state_transition_execution_context: &'a StateTransitionExecutionContext,
}

impl<'a> DataTriggerExecutionContext<'a> {
    /// Creates a new instance of DataTriggerExecutionContext
    pub fn new(
        drive: &'a Drive,
        owner_id: &'a Identifier,
        data_contract: &'a DataContract,
        state_transition_execution_context: &'a StateTransitionExecutionContext,
    ) -> Self {
        DataTriggerExecutionContext {
            drive,
            owner_id,
            data_contract,
            state_transition_execution_context,
        }
    }
}

use crate::{
    prelude::{DataContract, Identifier},
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};

#[derive(Clone, Debug)]
pub struct DataTriggerExecutionContext<'a, SR>
where
    SR: StateRepositoryLike,
{
    pub state_repository: &'a SR,
    pub owner_id: &'a Identifier,
    pub data_contract: &'a DataContract,
    pub state_transition_execution_context: &'a StateTransitionExecutionContext,
}

impl<'a, SR> DataTriggerExecutionContext<'a, SR>
where
    SR: StateRepositoryLike,
{
    /// Creates a new instance of DataTriggerExecutionContext
    pub fn new(
        state_repository: &'a SR,
        owner_id: &'a Identifier,
        data_contract: &'a DataContract,
        state_transition_execution_context: &'a StateTransitionExecutionContext,
    ) -> Self {
        DataTriggerExecutionContext {
            state_repository,
            owner_id,
            data_contract,
            state_transition_execution_context,
        }
    }
}

use crate::platform::PlatformStateRef;
use dpp::{
    prelude::{DataContract, Identifier},
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

#[derive(Clone)]
pub struct DataTriggerExecutionContext<'a> {
    pub platform: &'a PlatformStateRef<'a>,
    pub transaction: TransactionArg<'a, 'a>,
    pub owner_id: &'a Identifier,
    pub data_contract: &'a DataContract,
    pub state_transition_execution_context: &'a StateTransitionExecutionContext,
}

impl<'a> DataTriggerExecutionContext<'a> {
    /// Creates a new instance of DataTriggerExecutionContext
    pub fn new(
        platform: &'a PlatformStateRef<'a>,
        transaction: TransactionArg,
        owner_id: &'a Identifier,
        data_contract: &'a DataContract,
        state_transition_execution_context: &'a StateTransitionExecutionContext,
    ) -> Self {
        DataTriggerExecutionContext {
            platform,
            transaction,
            owner_id,
            data_contract,
            state_transition_execution_context,
        }
    }
}

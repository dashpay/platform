use crate::platform_types::platform::PlatformStateRef;
use dpp::block::epoch::Epoch;
use dpp::{
    prelude::{DataContract, Identifier},
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};
use drive::grovedb::TransactionArg;

/// DataTriggerExecutionContext represents the context in which a data trigger is executed.
/// It contains references to relevant state and transaction data needed for the trigger to perform its actions.
#[derive(Clone)]
pub struct DataTriggerExecutionContext<'a> {
    /// A reference to the platform state, which contains information about the current blockchain environment.
    pub platform: &'a PlatformStateRef<'a>,
    /// The transaction argument that triggered the data trigger.
    pub transaction: TransactionArg<'a, 'a>,
    /// The identifier of the owner of the data contract that the trigger is associated with.
    pub owner_id: &'a Identifier,
    /// A reference to the data contract associated with the data trigger.
    pub data_contract: &'a DataContract,
    /// A reference to the execution context for the state transition that triggered the data trigger.
    pub state_transition_execution_context: &'a StateTransitionExecutionContext,
}

impl<'a> DataTriggerExecutionContext<'a> {
    /// Creates a new instance of DataTriggerExecutionContext
    pub fn new(
        platform: &'a PlatformStateRef<'a>,
        transaction: TransactionArg<'a, 'a>,
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

    /// Returns the current epoch
    pub fn current_epoch(&self) -> Epoch {
        self.platform.epoch()
    }
}

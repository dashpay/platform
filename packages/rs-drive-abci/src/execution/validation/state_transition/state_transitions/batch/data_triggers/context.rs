use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::*;
use drive::grovedb::TransactionArg;
use std::fmt::{Debug, Formatter};

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
    /// The current block info, used as the source of `epoch` for trigger
    /// fee computations. Triggers must use `block_info.epoch` (matching
    /// the batch transformer's epoch source) rather than
    /// `platform.state.last_committed_block_epoch_ref()` so the per-batch
    /// fee accounting is consistent across all sites that bill on
    /// `transform_into_action: 1`.
    pub block_info: &'a BlockInfo,
    /// A reference to the execution context for the state transition that triggered the data trigger.
    pub state_transition_execution_context: &'a StateTransitionExecutionContext,
}

impl Debug for DataTriggerExecutionContext<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("data_trigger_execution_context")
            .field("platform", self.platform)
            .field("owner_id", self.owner_id)
            .field(
                "state_transition_execution_context",
                self.state_transition_execution_context,
            )
            .finish()
    }
}

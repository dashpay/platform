use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::*;
use drive::grovedb::TransactionArg;
use std::fmt::{Debug, Formatter};

/// DataTriggerExecutionContext represents the context in which a data trigger is executed.
/// It contains references to relevant state and transaction data needed for the trigger to perform its actions.
pub struct DataTriggerExecutionContext<'a> {
    /// A reference to the platform state, which contains information about the current blockchain environment.
    pub platform: &'a PlatformStateRef<'a>,
    /// The transaction argument that triggered the data trigger.
    pub transaction: TransactionArg<'a, 'a>,
    /// The identifier of the owner of the data contract that the trigger is associated with.
    pub owner_id: &'a Identifier,
    /// The current block info, used as the source of `epoch` for trigger
    /// fee computations on PROTOCOL_VERSION_12+. `_v1` triggers use
    /// `block_info.epoch` to match the batch transformer's epoch source.
    pub block_info: &'a BlockInfo,
    /// Mutable reference to the outer execution context — `_v1` triggers
    /// call `add_operation` on this to bill their drive reads directly.
    /// `_v0` triggers ignore it (preserving PROTOCOL_VERSION_11 chain
    /// replay byte-identity at the behavior level).
    pub state_transition_execution_context: &'a mut StateTransitionExecutionContext,
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

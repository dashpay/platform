use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Generation 1, except that a gas sponsor who pays is named as the owner of the storage
    /// the batch's documents write, so the refunds of that storage go back to them.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_event_v2(
        &self,
        event: ExecutionEvent,
        consensus_errors: Vec<ConsensusError>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
        block_credit_mints: &mut Credits,
        platform_version: &PlatformVersion,
    ) -> Result<EventExecutionResult, Error> {
        self.execute_event_v1_body(
            event,
            consensus_errors,
            block_info,
            transaction,
            address_balances_in_update,
            block_credit_mints,
            true,
            platform_version,
        )
    }
}

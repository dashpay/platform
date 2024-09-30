use crate::error::Error;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::{platform_types::platform::Platform, rpc::core::CoreRPCLike};
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn rebroadcast_expired_withdrawal_documents_v0(
        &self,
        block_info: &BlockInfo,
        last_committed_platform_state: &PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Currently Core only supports using the first 2 quorums (out of 24 for mainnet).
        // For us, we just use the latest quorum to be extra safe.
        let Some(position_of_current_quorum) =
            last_committed_platform_state.current_validator_set_position_in_list_by_most_recent()
        else {
            tracing::warn!("Current quorum not in current validator set, not making withdrawals");
            return Ok(());
        };
        if position_of_current_quorum != 0 {
            tracing::debug!(
                "Current quorum is not most recent, it is in position {}, not making withdrawals",
                position_of_current_quorum
            );
            return Ok(());
        }
        // Version 1 changes on Version 0, by not having the Core 2 Quorum limit.
        // Hence we can just use the v1 here after the extra logic of v0
        self.rebroadcast_expired_withdrawal_documents_v1(block_info, transaction, platform_version)
    }
}

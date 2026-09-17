use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Runs the protocol change events of generation 1 and then the transition to protocol
    /// version 17 when the chain crosses it: the token contract lifecycle ledger is built
    /// through the same helper genesis uses and one record per issuer is backfilled from the
    /// supply tree after reconciling every token.
    ///
    /// The transition runs after the cache refresh of generation 1 because it writes no
    /// contract; it touches only the `[Tokens]` ledger, which no cache holds.
    pub(super) fn perform_events_on_first_block_of_protocol_change_v2(
        &self,
        platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.perform_events_on_first_block_of_protocol_change_v1(
            platform_state,
            block_info,
            transaction,
            previous_protocol_version,
            platform_version,
        )?;

        if previous_protocol_version < 17 && platform_version.protocol_version >= 17 {
            self.transition_to_version_17_token_lifecycles(transaction, platform_version)?;
        }

        Ok(())
    }
}

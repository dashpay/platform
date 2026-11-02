use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Empties the contract cache, then runs the protocol change events of v1.
    ///
    /// A cached contract can carry the fee of the read that cached it, calculated under the fee
    /// schedule of the protocol version it was read in, and a cache hit bills that fee again. A
    /// protocol change is the only time the schedule can change, and nothing else empties the
    /// cache, so without this a node that stayed up across the change would keep billing reads
    /// at the old schedule while a node that restarted bills them at the new one.
    ///
    /// The cache is emptied first because v1 then seeds the block cache with the system
    /// contracts the events may have rewritten. `clear` keeps the record of what this block
    /// rewrote, so a transactional read of a rewritten contract still never falls back to a copy
    /// a query puts into the global cache.
    pub(super) fn perform_events_on_first_block_of_protocol_change_v2(
        &self,
        platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.cache.data_contracts.clear();

        self.perform_events_on_first_block_of_protocol_change_v1(
            platform_state,
            block_info,
            transaction,
            previous_protocol_version,
            platform_version,
        )
    }
}

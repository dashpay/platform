use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::data_contracts::SystemDataContract;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Runs the protocol change events and then refreshes the cached definitions of the system
    /// contracts those events may have rewritten.
    ///
    /// CONSENSUS-CRITICAL, and the reason this exists separately from v0. The transitions write
    /// contracts to state through `insert_contract`/`apply_contract`, which reach state outside
    /// the drive operation batch and so carry none of its cache invalidation. Left stale, a
    /// validator that had read a contract once would keep serializing documents against its
    /// cached copy while a validator with a cold cache reads the rewritten one, and the two
    /// would write different bytes for the same transition.
    ///
    /// Dropping the stale entries is not enough on its own: a read-only query thread reads
    /// committed state with no transaction and populates the *global* cache, and a transactional
    /// lookup falls back to the global cache when the block cache misses. A query landing
    /// between the eviction and the rest of this block would otherwise put the pre-change
    /// definition back and decide, by timing alone, which definition this block serializes
    /// documents against. Seeding the block cache gives transactional reads an authority they
    /// hit before ever reaching the global entry.
    ///
    /// This relies on the block cache having already been cleared for this block, which
    /// `run_block_proposal` does before invoking the protocol change events — seeding before
    /// that clear would be wiped before anything read it.
    pub(super) fn perform_events_on_first_block_of_protocol_change_v1(
        &self,
        platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.perform_events_on_first_block_of_protocol_change_v0(
            platform_state,
            block_info,
            transaction,
            previous_protocol_version,
            platform_version,
        )?;

        // Every system contract, not only the ones a given transition happens to write: the
        // set lives beside the enum that defines it, so a contract added later is covered here
        // without anyone remembering to update this. Refreshing one the migration did not touch
        // costs a single read, and one that is not in state caches nothing.
        for system_contract in SystemDataContract::ALL {
            self.drive.refresh_data_contract_cache_from_state(
                system_contract.id().to_buffer(),
                Some(transaction),
                platform_version,
            )?;
        }

        Ok(())
    }
}

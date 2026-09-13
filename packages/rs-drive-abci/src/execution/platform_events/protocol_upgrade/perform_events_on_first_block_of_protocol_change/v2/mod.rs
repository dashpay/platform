use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::version::{PlatformVersion, ProtocolVersion};
use drive::grovedb::Transaction;

impl<C> Platform<C> {
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
        // Any migration error deliberately halts this activation block for every
        // validator; the shared transaction prevents a partial migration from
        // committing. Its corruption checks are unreachable for valid pre-14
        // state and must never be downgraded to best-effort recovery.
        if previous_protocol_version < 14 {
            let stats = self
                .drive
                .migrate_document_history_storage(transaction, platform_version)?;
            tracing::info!(?stats, "Migrated document history storage");
        }
        Ok(())
    }
}

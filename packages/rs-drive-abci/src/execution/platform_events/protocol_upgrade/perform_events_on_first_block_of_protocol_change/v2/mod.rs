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
        if previous_protocol_version < 14 && platform_version.protocol_version >= 14 {
            let stats = self
                .drive
                .migrate_document_history_storage(transaction, platform_version)?;
            tracing::info!(?stats, "Migrated document history storage");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;

    #[test]
    fn should_not_migrate_history_before_protocol_14() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 should exist");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();

        platform
            .perform_events_on_first_block_of_protocol_change_v2(
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                13,
                platform_version,
            )
            .expect("a protocol 13 transition must not run the protocol 14 migration");
    }
}

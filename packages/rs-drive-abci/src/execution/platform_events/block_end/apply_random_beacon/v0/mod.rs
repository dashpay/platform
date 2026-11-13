use crate::error::Error;
use crate::execution::types::block_random_requests::BlockRandomRequests;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[inline(always)]
    pub(super) fn apply_random_beacon_v0(
        &self,
        beacon: &[u8],
        random_requests: &BlockRandomRequests,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Nothing is read or written for a block that asked for nothing, which is most of them
        if random_requests.is_empty() {
            return Ok(());
        }

        let drawn_documents = self.drive.draw_random_document_properties(
            beacon,
            &random_requests.documents,
            block_info,
            Some(transaction),
            platform_version,
        )?;

        let settled_token_amounts = self.drive.settle_random_token_amounts(
            beacon,
            &random_requests.tokens,
            block_info,
            Some(transaction),
            platform_version,
        )?;

        tracing::debug!(
            height = block_info.height,
            drawn_documents,
            settled_token_amounts,
            "applied the block's random beacon",
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::execution::types::block_random_requests::BlockRandomRequests;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_leave_the_state_alone_when_the_block_asked_for_nothing() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();
        let root_before = platform
            .drive
            .grove
            .root_hash(Some(&transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("root hash");

        platform
            .apply_random_beacon(
                &[7u8; 96],
                &BlockRandomRequests::default(),
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("an empty block applies its beacon");

        let root_after = platform
            .drive
            .grove
            .root_hash(Some(&transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("root hash");
        assert_eq!(root_before, root_after);
    }

    #[test]
    fn should_refuse_a_protocol_version_before_15() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();

        let result = platform.apply_random_beacon(
            &[7u8; 96],
            &BlockRandomRequests::default(),
            &BlockInfo::default(),
            &transaction,
            PlatformVersion::get(14).expect("protocol version 14"),
        );
        assert!(
            result.is_err(),
            "the step is not active before protocol version 15"
        );
    }
}

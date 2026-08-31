use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::sync::Arc;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the state cache at the end of finalize block. This is done by overriding the current
    /// platform state cache with the block execution state cache.
    ///
    /// This function takes an `ExtendedBlockInfo` and a `Transaction` as input and updates the
    /// state cache and quorums based on the given block information. It handles protocol version
    /// updates and sets the current and next epoch protocol versions.
    ///
    /// # Arguments
    ///
    /// * `extended_block_info` - Extended block information for the current block.
    /// * `block_platform_state` - The platform state for this block.
    /// * `transaction` - The transaction associated with the block.
    /// * `platform_version` - The platform version.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with updating the state cache
    /// and quorums or storing the ephemeral data.
    ///
    #[inline(always)]
    pub(super) fn update_state_cache_v0(
        &self,
        extended_block_info: ExtendedBlockInfo,
        mut block_platform_state: PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Update block state and store it in shared lock
        if let Some(next_validator_set_quorum_hash) =
            block_platform_state.take_next_validator_set_quorum_hash()
        {
            block_platform_state
                .set_current_validator_set_quorum_hash(next_validator_set_quorum_hash);
        }

        block_platform_state.set_last_committed_block_info(Some(extended_block_info));

        block_platform_state.set_genesis_block_info(None);

        // Persist block state
        self.store_platform_state(&block_platform_state, Some(transaction), platform_version)?;

        // Whatever the store wrote is now what is on disk for this block, so the
        // next block only has to write the full record if it changes something
        // heavy itself.
        block_platform_state.heavy_fields_dirty = false;

        let block_platform_state = Arc::new(block_platform_state);

        self.state.store(block_platform_state);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
    use dpp::block::extended_block_info::ExtendedBlockInfo;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::QuorumHash;
    use dpp::version::PlatformVersion;

    use crate::platform_types::platform_state::PlatformStateV0Methods;

    fn make_extended_block_info(height: u64) -> ExtendedBlockInfo {
        ExtendedBlockInfo::V0(ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                time_ms: 1_000_000,
                height,
                core_height: 10,
                epoch: Default::default(),
            },
            app_hash: [1u8; 32],
            quorum_hash: [2u8; 32],
            block_id_hash: [3u8; 32],
            proposer_pro_tx_hash: [4u8; 32],
            signature: [5u8; 96],
            round: 0,
        })
    }

    /// When no `next_validator_set_quorum_hash` is set on `block_platform_state`,
    /// the current quorum hash must remain unchanged, and both the genesis info and
    /// last committed block info must be updated accordingly.
    #[test]
    fn v0_no_next_quorum_hash_preserves_current_quorum_and_sets_block_info() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let loaded = platform.state.load();
        let mut block_platform_state = loaded.as_ref().clone();
        drop(loaded);

        let original_quorum = QuorumHash::from_byte_array([0x42u8; 32]);
        block_platform_state.set_current_validator_set_quorum_hash(original_quorum);
        block_platform_state.set_next_validator_set_quorum_hash(None);

        let transaction = platform.drive.grove.start_transaction();
        let extended = make_extended_block_info(7);

        platform
            .update_state_cache_v0(
                extended,
                block_platform_state,
                &transaction,
                platform_version,
            )
            .expect("update_state_cache_v0 must succeed");

        let state = platform.state.load();
        assert_eq!(
            state.current_validator_set_quorum_hash(),
            original_quorum,
            "current quorum hash must be preserved when next is None"
        );
        assert!(
            state.last_committed_block_info().is_some(),
            "last_committed_block_info must be populated"
        );
        assert_eq!(
            state.last_committed_block_height(),
            7,
            "height must come from the extended block info we passed"
        );
        assert!(
            state.genesis_block_info().is_none(),
            "genesis_block_info must be cleared"
        );
    }

    /// When `next_validator_set_quorum_hash` is set, it must be moved to the
    /// current quorum hash and cleared from `next`.
    #[test]
    fn v0_next_quorum_hash_rotates_into_current() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let loaded = platform.state.load();
        let mut block_platform_state = loaded.as_ref().clone();
        drop(loaded);

        let old = QuorumHash::from_byte_array([0x11u8; 32]);
        let new = QuorumHash::from_byte_array([0x22u8; 32]);
        block_platform_state.set_current_validator_set_quorum_hash(old);
        block_platform_state.set_next_validator_set_quorum_hash(Some(new));

        let transaction = platform.drive.grove.start_transaction();
        let extended = make_extended_block_info(42);

        platform
            .update_state_cache_v0(
                extended,
                block_platform_state,
                &transaction,
                platform_version,
            )
            .expect("must succeed");

        let state = platform.state.load();
        assert_eq!(
            state.current_validator_set_quorum_hash(),
            new,
            "current quorum must become the prior next quorum"
        );
        assert!(
            state.next_validator_set_quorum_hash().is_none(),
            "next quorum must be consumed"
        );
    }

    /// `genesis_block_info` is always cleared on update_state_cache, regardless of
    /// whether the prior block platform state had one set.
    #[test]
    fn v0_clears_genesis_block_info() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let loaded = platform.state.load();
        let mut block_platform_state = loaded.as_ref().clone();
        drop(loaded);

        // Even if genesis was set, update_state_cache must clear it.
        block_platform_state.set_genesis_block_info(Some(BlockInfo {
            time_ms: 1,
            height: 0,
            core_height: 0,
            epoch: Default::default(),
        }));

        let transaction = platform.drive.grove.start_transaction();
        let extended = make_extended_block_info(1);

        platform
            .update_state_cache_v0(
                extended,
                block_platform_state,
                &transaction,
                platform_version,
            )
            .expect("must succeed");

        assert!(
            platform.state.load().genesis_block_info().is_none(),
            "genesis_block_info must always be cleared"
        );
    }
}

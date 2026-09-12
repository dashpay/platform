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

    /// While replaying history the full saved record is rewritten only when a
    /// heavy field changed; the small record carries the block info in between.
    /// A node restarted from disk must see the newest block info and the heavy
    /// fields from the last full write.
    #[test]
    fn v0_historical_block_with_clean_heavy_fields_reloads_from_the_small_record() {
        use crate::config::{PlatformConfig, PlatformTestConfig};
        use crate::platform_types::platform::Platform;
        use crate::platform_types::platform_state::PlatformState;
        use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
        use dpp::dashcore::{ProTxHash, Txid};
        use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem, MasternodeType};
        use dpp::serialization::PlatformDeserializableFromVersionedStructure;

        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig {
                testing_configs: PlatformTestConfig {
                    store_platform_state: true,
                    ..PlatformTestConfig::default_minimal_verifications()
                },
                ..Default::default()
            })
            .build_with_mock_rpc()
            .set_genesis_state();

        let loaded = platform.state.load();
        let mut block_platform_state = loaded.as_ref().clone();
        drop(loaded);

        // A block from long ago, so the store treats it as replayed history.
        let mut old_block = make_extended_block_info(7);
        old_block.basic_info_mut().time_ms = 1_000_000;

        // Block 7 changes a heavy field, so it is written in full.
        let pro_tx_hash = ProTxHash::from_byte_array([0x77u8; 32]);
        let masternode = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array([0u8; 32]),
            collateral_index: 0,
            collateral_address: [0u8; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:1234".parse().expect("socket address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: [0u8; 20],
                voting_address: [0u8; 20],
                payout_address: [0u8; 20],
                pub_key_operator: vec![0u8; 48],
                operator_payout_address: None,
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };
        block_platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode);
        assert!(block_platform_state.heavy_fields_dirty);

        let transaction = platform.drive.grove.start_transaction();
        platform
            .update_state_cache_v0(
                old_block,
                block_platform_state,
                &transaction,
                platform_version,
            )
            .expect("block 7 must be stored");

        // Block 8 changes nothing heavy, so only the small record is written.
        let loaded = platform.state.load();
        let block_platform_state = loaded.as_ref().clone();
        drop(loaded);
        assert!(!block_platform_state.heavy_fields_dirty);

        let mut old_block = make_extended_block_info(8);
        old_block.basic_info_mut().time_ms = 1_000_001;
        platform
            .update_state_cache_v0(
                old_block,
                block_platform_state,
                &transaction,
                platform_version,
            )
            .expect("block 8 must be stored");

        let reloaded = Platform::<crate::rpc::core::MockCoreRPCLike>::fetch_platform_state(
            &platform.drive,
            Some(&transaction),
            platform_version,
        )
        .expect("fetch must succeed")
        .expect("a state was stored");

        assert_eq!(
            reloaded.last_committed_block_height(),
            8,
            "block info comes from the small record written at block 8"
        );
        assert!(
            reloaded.full_masternode_list().contains_key(&pro_tx_hash),
            "heavy fields come from the full record written at block 7"
        );

        // The full record on disk must still be block 7's: that is what proves
        // block 8 skipped it rather than rewriting it with the same contents.
        let full_bytes = platform
            .drive
            .fetch_platform_state_bytes(Some(&transaction), platform_version)
            .expect("fetch must succeed")
            .expect("a full record was stored");
        let full_record = PlatformState::versioned_deserialize(&full_bytes, platform_version)
            .expect("full record must deserialize");
        assert_eq!(
            full_record.last_committed_block_height(),
            7,
            "the full record is not rewritten for a historical block that changed nothing heavy"
        );
    }
}

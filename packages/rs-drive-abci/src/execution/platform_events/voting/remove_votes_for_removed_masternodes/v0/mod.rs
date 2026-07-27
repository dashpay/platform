use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Removes the votes for removed masternodes
    pub(super) fn remove_votes_for_removed_masternodes_v0(
        &self,
        block_info: &BlockInfo,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let masternode_list_changes =
            block_platform_state.full_masternode_list_changes(last_committed_platform_state);

        if !masternode_list_changes.removed_masternodes.is_empty() {
            self.drive.remove_all_votes_given_by_identities(
                masternode_list_changes
                    .removed_masternodes
                    .iter()
                    .map(|pro_tx_hash| pro_tx_hash.as_byte_array().to_vec())
                    .collect(),
                block_info.height,
                self.config.network,
                self.config.abci.chain_id.as_str(),
                transaction,
                platform_version,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;

    /// When `last_committed` and `block_platform_state` are identical clones, there
    /// are no masternode list changes, so the method must early-return `Ok(())`
    /// without touching storage.
    #[test]
    fn v0_noop_when_no_removed_masternodes() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let state = platform.state.load();
        let a = state.as_ref().clone();
        let b = state.as_ref().clone();
        drop(state);

        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 1_000,
            height: 1,
            core_height: 1,
            epoch: Default::default(),
        };

        platform
            .remove_votes_for_removed_masternodes_v0(
                &block_info,
                &a,
                &b,
                Some(&transaction),
                platform_version,
            )
            .expect("no changes must be a no-op success");
    }

    /// The method must still succeed when the platform state has no masternodes at
    /// all (genesis with empty masternode list) — this is the canonical empty case
    /// for the "no removed masternodes" branch.
    #[test]
    fn v0_ok_on_empty_state_cloned_thrice() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 1_000,
            height: 1,
            core_height: 1,
            epoch: Default::default(),
        };

        let state = platform.state.load();
        for _ in 0..3 {
            let a = state.as_ref().clone();
            let b = state.as_ref().clone();
            platform
                .remove_votes_for_removed_masternodes_v0(
                    &block_info,
                    &a,
                    &b,
                    Some(&transaction),
                    platform_version,
                )
                .expect("must remain OK across repeated calls with no diff");
        }
    }
}

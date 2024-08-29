mod update_state_masternode_list;
mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;

use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the masternode list in the platform state based on changes in the masternode list
    /// from Dash Core between two block heights.
    ///
    /// This function fetches the masternode list difference between the current core block height
    /// and the previous core block height, then updates the full masternode list and the
    /// HPMN (high performance masternode) list in the platform state accordingly.
    ///
    /// # Arguments
    ///
    /// * `state` - A mutable reference to the platform state to be updated.
    /// * `core_block_height` - The current block height in the Dash Core.
    /// * `transaction` - The current groveDB transaction.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns `Ok(())` if the update is successful. Returns an error if
    ///   there is a problem fetching the masternode list difference or updating the state.
    pub(super) fn update_masternode_list(
        &self,
        platform_state: Option<&PlatformState>,
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        is_init_chain: bool,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .update_masternode_list
        {
            0 => self.update_masternode_list_v0(
                platform_state,
                block_platform_state,
                core_block_height,
                is_init_chain,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_masternode_list".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::PlatformConfig;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dashcore_rpc::json::MasternodeListDiff;
    use std::env;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::PathBuf;

    #[test]
    fn test_update_masternode_list() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig::default();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let mut init_chain_platform_state = platform_state.as_ref().clone();

        let genesis_core_block_height = 2128896;
        let first_block_core_block_height = 2129440;
        let genesis_time = 1;
        let first_block_time = 2;

        let genesis_block_info = BlockInfo {
            height: 1,
            core_height: genesis_core_block_height,
            time_ms: genesis_time,
            ..Default::default()
        };

        let first_block_info = BlockInfo {
            height: 1,
            core_height: first_block_core_block_height,
            time_ms: first_block_time,
            ..Default::default()
        };

        fn adjust_path_based_on_current_dir(relative_path: &str) -> PathBuf {
            let current_dir = env::current_dir().expect("expected to get current directory");
            // Check if the current directory ends with "platform"
            let adjusted_path = if current_dir.ends_with("platform") {
                current_dir
                    .join("packages/rs-drive-abci")
                    .join(relative_path)
            } else {
                current_dir.join(relative_path)
            };

            adjusted_path
        }

        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |base_block, block| {
                if block == 2128896 {
                    let file_path = adjust_path_based_on_current_dir(
                        "tests/supporting_files/mainnet_protx_list_diffs/1-2128896.json",
                    );
                    println!(
                        "Current directory: {:?}, using {:?}",
                        std::env::current_dir(),
                        &file_path
                    );
                    // Deserialize the first JSON file
                    let file = File::open(file_path).expect("expected to open file");
                    let reader = BufReader::new(file);
                    let init_chain_masternode_list_diff: MasternodeListDiff =
                        serde_json::from_reader(reader)
                            .expect("expected to deserialize into a masternode list diff");

                    Ok(init_chain_masternode_list_diff)
                } else {
                    // Deserialize the second JSON file
                    let file = File::open(adjust_path_based_on_current_dir(
                        "tests/supporting_files/mainnet_protx_list_diffs/2128896-2129440.json",
                    ))
                    .expect("expected to open file");
                    let reader = BufReader::new(file);
                    let block_1_masternode_list_diff: MasternodeListDiff =
                        serde_json::from_reader(reader)
                            .expect("expected to deserialize into a masternode list diff");

                    Ok(block_1_masternode_list_diff)
                }
            });

        let transaction = platform.drive.grove.start_transaction();

        platform
            .update_masternode_list(
                None,
                &mut init_chain_platform_state,
                genesis_core_block_height,
                true,
                &genesis_block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to update masternode list");

        let platform_state = init_chain_platform_state.clone();

        let mut block_platform_state = platform_state.clone();

        platform
            .update_masternode_list(
                Some(&platform_state),
                &mut block_platform_state,
                first_block_core_block_height,
                false,
                &first_block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to update masternode list");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
    }
}

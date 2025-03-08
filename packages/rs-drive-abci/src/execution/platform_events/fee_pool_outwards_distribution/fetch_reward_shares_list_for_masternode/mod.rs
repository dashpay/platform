pub mod v0;
/// Masternode reward shares document type
pub const MN_REWARD_SHARES_DOCUMENT_TYPE: &str = "rewardShare";

use crate::error::Error;
use crate::platform_types::platform::Platform;

use dpp::version::PlatformVersion;

use drive::dpp::document::Document;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;

impl<C> Platform<C> {
    /// Fetches the reward shares list for a masternode based on the platform version.
    ///
    /// # Arguments
    ///
    /// * `masternode_owner_id` - The owner ID of the masternode.
    /// * `transaction` - The transaction argument.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<Document>, Error>` - A list of `Document` if successful. Otherwise, an `Error`.
    pub fn fetch_reward_shares_list_for_masternode(
        &self,
        masternode_owner_id: &[u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        match platform_version
            .drive_abci
            .methods
            .fee_pool_outwards_distribution
            .fetch_reward_shares_list_for_masternode
        {
            0 => self.fetch_reward_shares_list_for_masternode_v0(
                masternode_owner_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_reward_shares_list_for_masternode".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

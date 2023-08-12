pub mod v0;

/// Masternode reward shares contract ID
pub const MN_REWARD_SHARES_CONTRACT_ID: [u8; 32] = [
    0x0c, 0xac, 0xe2, 0x05, 0x24, 0x66, 0x93, 0xa7, 0xc8, 0x15, 0x65, 0x23, 0x62, 0x0d, 0xaa, 0x93,
    0x7d, 0x2f, 0x22, 0x47, 0x93, 0x44, 0x63, 0xee, 0xb0, 0x1f, 0xf7, 0x21, 0x95, 0x90, 0x95, 0x8c,
];

/// Masternode reward shares document type
pub const MN_REWARD_SHARES_DOCUMENT_TYPE: &str = "rewardShare";

use crate::error::Error;
use crate::platform_types::platform::Platform;

use dpp::platform_value::Value;
use dpp::version::PlatformVersion;

use drive::dpp::document::Document;
use drive::drive::document::query::QueryDocumentsOutcome;
use drive::grovedb::TransactionArg;
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};

use crate::error::execution::ExecutionError;
use std::collections::BTreeMap;

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
        masternode_owner_id: &[u8],
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

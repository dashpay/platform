mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;

impl Drive {
    /// Marks a perpetual token release as distributed in the state tree.
    ///
    /// This function updates the perpetual distribution record by:
    /// - Removing the previous distribution moment.
    /// - Setting the new distribution moment.
    /// - Associating the new distribution with the correct recipient.
    ///
    /// # Parameters
    /// - `token_id`: The unique identifier of the token.
    /// - `owner_id`: The unique identifier of the owner who initiated the distribution.
    /// - `previous_moment`: The previous moment when the reward was last distributed.
    /// - `next_moment`: The next moment when the reward should be distributed.
    /// - `distribution_recipient`: The recipient of the distributed reward.
    /// - `block_info`: Metadata about the current block, including epoch details.
    /// - `estimated_costs_only_with_layer_info`: Optional storage layer information for cost estimation.
    /// - `batch_operations`: A mutable reference to the batch operation queue.
    /// - `transaction`: The transaction context.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok(())` if the operation succeeds.
    /// - `Err(Error::Drive(DriveError::UnknownVersionMismatch))` if an unsupported version is encountered.
    ///
    /// # Behavior
    /// - If `estimated_costs_only_with_layer_info` is `Some`, the function only estimates costs.
    /// - The previous distribution entry is deleted from the tree.
    /// - The new distribution entry is inserted with a reference to the corresponding recipient.
    ///
    /// # Versioning
    /// - Uses version 0 of `mark_perpetual_release_as_distributed_operations_v0` if supported.
    /// - Returns an error if an unknown version is received.
    pub fn mark_perpetual_release_as_distributed_operations(
        &self,
        token_id: [u8; 32],
        owner_id: [u8; 32],
        previous_moment: RewardDistributionMoment,
        next_moment: RewardDistributionMoment,
        distribution_recipient: TokenDistributionRecipient,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .distribution
            .mark_perpetual_release_as_distributed
        {
            0 => self.mark_perpetual_release_as_distributed_operations_v0(
                token_id,
                owner_id,
                previous_moment,
                next_moment,
                distribution_recipient,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "mark_perpetual_release_as_distributed".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;

impl Drive {
    /// Marks a perpetual token distribution as "distributed" by updating the last claimed moment for an identity.
    ///
    /// This function records the current distribution moment in the GroveDB tree, indicating that the recipient
    /// identified by `owner_id` has claimed the distribution for the specified perpetual token.
    ///
    /// # Actions Performed
    /// - Inserts or updates the distribution record for the recipient (`owner_id`) at the specified token's perpetual distribution path.
    /// - If provided, updates estimated costs for storage layers involved.
    ///
    /// # Parameters
    /// - `token_id`: A 32-byte identifier uniquely representing the token.
    /// - `recipient_id`: A 32-byte identifier uniquely representing the recipient (identity) claiming the distribution.
    /// - `current_moment`: The moment (`RewardDistributionMoment`) representing when the distribution occurred.
    /// - `estimated_costs_only_with_layer_info`: Optional mutable reference to a hashmap for estimating storage costs. If `Some`, cost estimation is performed without executing database writes.
    /// - `platform_version`: A reference to the current `PlatformVersion` to determine correct version-specific behavior.
    ///
    /// # Returns
    /// - `Ok(Vec<LowLevelDriveOperation>)`: Batch operations to perform the storage update if successful.
    /// - `Err(Error::Drive(DriveError::UnknownVersionMismatch))`: If an unsupported `platform_version` is encountered.
    ///
    /// # Versioning
    /// - Currently supports version `0`. If an unknown version is specified, an error is returned.
    pub fn mark_perpetual_release_as_distributed_operations(
        &self,
        token_id: [u8; 32],
        recipient_id: [u8; 32],
        current_moment: RewardDistributionMoment,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
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
                recipient_id,
                current_moment,
                estimated_costs_only_with_layer_info,
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

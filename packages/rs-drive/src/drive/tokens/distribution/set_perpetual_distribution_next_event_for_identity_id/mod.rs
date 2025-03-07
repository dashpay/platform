mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Sets the next scheduled event time for a perpetual distribution for a given identity,
    /// using the appropriate versioned method.
    ///
    /// This method updates the perpetual distributions tree at the path
    /// `token_perpetual_distributions_path_vec(token_id)`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `identity_id`: The identifier of the identity whose next event timestamp is being set.
    /// - `moment`: The `RewardDistributionMoment` indicating the moment the identity just made their claim.
    /// - `block_info`: Block metadata used for setting storage flags.
    /// - `drive_operations`: A mutable vector to accumulate low-level drive operations.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// A `Result<(), Error>` indicating success or failure.
    pub(crate) fn set_perpetual_distribution_claimed_for_identity_id_operations(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        moment: RewardDistributionMoment,
        block_info: &BlockInfo,
        known_to_be_replace: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .token
            .update
            .perpetual_distribution_next_event_for_identity_id
        {
            0 => self.set_perpetual_distribution_claimed_for_identity_id_operations_v0(
                token_id,
                identity_id,
                moment,
                block_info,
                known_to_be_replace,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "set_perpetual_distribution_next_event_for_identity_id_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

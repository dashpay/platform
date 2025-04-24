mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;

impl Drive {
    /// Fetches the last paid timestamp for a perpetual distribution for a given identity,
    /// using the appropriate versioned method.
    ///
    /// This method queries the perpetual distributions tree at the path
    /// `perpetual_distribution_last_paid_time_path_vec(token_id, identity_id)`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `identity_id`: The identifier of the identity whose last paid time is being queried.
    /// - `distribution_type`: The distribution type known from the Token configuration.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// A `Result` containing the last paid `RewardDistributionMoment` on success or an `Error` on failure.
    pub fn fetch_perpetual_distribution_last_paid_moment(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        distribution_type: &RewardDistributionType,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<RewardDistributionMoment>, Error> {
        self.fetch_perpetual_distribution_last_paid_moment_operations(
            token_id,
            identity_id,
            distribution_type,
            &mut vec![],
            transaction,
            platform_version,
        )
    }

    /// Fetches the last paid moment as raw bytes for a perpetual distribution for a given identity,
    /// using the appropriate versioned method.
    ///
    /// This method queries the perpetual distributions tree at the path
    /// `perpetual_distribution_last_paid_time_path_vec(token_id, identity_id)`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `identity_id`: The identifier of the identity whose last paid time is being queried.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Vec<u8>))`: The raw stored bytes if a moment exists.
    /// - `Ok(None)`: If no moment is recorded for the identity.
    /// - `Err(_)`: If an internal GroveDB or decoding error occurs.
    pub fn fetch_perpetual_distribution_last_paid_moment_raw(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .perpetual_distribution_last_paid_time
        {
            0 => self.fetch_perpetual_distribution_last_paid_moment_raw_operations_v0(
                token_id,
                identity_id,
                &mut vec![],
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_perpetual_distribution_last_paid_moment_raw".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the last paid timestamp for a perpetual distribution for a given identity,
    /// using the appropriate versioned method.
    ///
    /// This method queries the perpetual distributions tree at the path
    /// `perpetual_distribution_last_paid_time_path_vec(token_id, identity_id)`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `identity_id`: The identifier of the identity whose last paid time is being queried.
    /// - `distribution_type`: The distribution type known from the Token configuration.
    /// - `drive_operations`: A mutable vector to accumulate low-level drive operations.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// A `Result` containing the last paid `RewardDistributionMoment` on success or an `Error` on failure.
    pub(crate) fn fetch_perpetual_distribution_last_paid_moment_operations(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        distribution_type: &RewardDistributionType,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<RewardDistributionMoment>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .perpetual_distribution_last_paid_time
        {
            0 => self.fetch_perpetual_distribution_last_paid_moment_operations_v0(
                token_id,
                identity_id,
                distribution_type,
                drive_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_perpetual_distribution_last_paid_moment_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

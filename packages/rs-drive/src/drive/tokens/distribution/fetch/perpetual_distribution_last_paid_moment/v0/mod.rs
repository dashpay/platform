use crate::drive::tokens::paths::{token_perpetual_distributions_identity_last_claimed_time_path};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;

impl Drive {
    /// Fetches the raw bytes for the last paid moment of a perpetual distribution for a given identity.
    ///
    /// This method queries the `perpetual_distribution_last_paid_time_path_vec(token_id, identity_id)`
    /// and returns the raw GroveDB value associated with the identity. This is a low-level utility used
    /// when the caller wants to interpret the encoding (e.g., timestamp, block height, epoch) themselves.
    ///
    /// The value is expected to be stored as an `Item`, and if found, is returned as a raw `Vec<u8>`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32-byte identifier of the token.
    /// - `identity_id`: The identifier of the identity whose last paid moment is queried.
    /// - `drive_operations`: A mutable vector that accumulates low-level GroveDB operations.
    /// - `transaction`: The GroveDB transaction under which the query is executed.
    /// - `platform_version`: The current platform version, used for compatibility checks.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Vec<u8>))`: The raw stored bytes if a moment exists.
    /// - `Ok(None)`: If no moment is recorded for the identity.
    /// - `Err(_)`: If an internal GroveDB or decoding error occurs.
    pub(super) fn fetch_perpetual_distribution_last_paid_moment_raw_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, Error> {
        let direct_query_type = DirectQueryType::StatefulDirectQuery;

        let perpetual_distributions_path =
            token_perpetual_distributions_identity_last_claimed_time_path(&token_id);

        match self.grove_get_raw_optional(
            (&perpetual_distributions_path).into(),
            identity_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(value, _))) => Ok(Some(value)),

            Ok(None) => Ok(None),
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_)) => {
                Ok(None)
            }

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "Last moment was present but was not an item",
            ))),

            Err(e) => Err(e),
        }
    }

    /// Fetches the decoded last paid moment (`RewardDistributionMoment`) of a perpetual distribution
    /// for a given identity by using the distribution type's deserialization logic.
    ///
    /// This method wraps [`fetch_perpetual_distribution_last_paid_moment_raw_operations_v0`]
    /// and performs decoding from raw bytes into a structured [`RewardDistributionMoment`] using the
    /// given [`RewardDistributionType`].
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte token identifier.
    /// - `identity_id`: The identifier of the identity.
    /// - `distribution_type`: The configured distribution encoding strategy.
    /// - `drive_operations`: A mutable vector that accumulates GroveDB operations.
    /// - `transaction`: The GroveDB transaction for the query.
    /// - `platform_version`: The platform version selector.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `Ok(Some(moment))`: If a moment was found and successfully decoded.
    /// - `Ok(None)`: If no record was found.
    /// - `Err(_)`: If decoding failed or an internal error occurred.
    pub(super) fn fetch_perpetual_distribution_last_paid_moment_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        distribution_type: &RewardDistributionType,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<RewardDistributionMoment>, Error> {
        let raw_opt = self.fetch_perpetual_distribution_last_paid_moment_raw_operations_v0(
            token_id,
            identity_id,
            drive_operations,
            transaction,
            platform_version,
        )?;

        match raw_opt {
            Some(raw_bytes) => {
                let moment = distribution_type
                    .moment_from_bytes(&raw_bytes)
                    .map_err(|e| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "Moment should be specific amount of bytes: {}",
                            e
                        )))
                    })?;
                Ok(Some(moment))
            }
            None => Ok(None),
        }
    }
}

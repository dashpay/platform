use crate::drive::tokens::paths::token_pre_programmed_distributions_identity_last_claimed_time_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the last paid timestamp for a pre_programmed distribution for a given identity.
    ///
    /// This method queries the `token_pre_programmed_distributions_path_vec(token_id)` tree and
    /// retrieves the last recorded payment timestamp (`TimestampMillis`) associated with
    /// `identity_id`. The timestamp is expected to be stored as an 8-byte big-endian value.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `identity_id`: The identifier of the identity whose last paid time is being queried.
    /// - `drive_operations`: A mutable vector to accumulate low-level drive operations.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// A `Result` containing `Some(RewardDistributionMoment)` if a record exists, `None` if no record is found,
    /// or an `Error` if retrieval fails.
    pub(super) fn fetch_pre_programmed_distribution_last_paid_time_ms_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TimestampMillis>, Error> {
        let direct_query_type = DirectQueryType::StatefulDirectQuery;

        let pre_programmed_distributions_path =
            token_pre_programmed_distributions_identity_last_claimed_time_path(&token_id);

        match self.grove_get_raw_optional(
            (&pre_programmed_distributions_path).into(),
            identity_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(value, _))) => {
                if value.len() != 8 {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "Pre programmed last claimed time should be encoded on 8 bytes".to_string(),
                    )));
                }
                let mut array = [0u8; 8];
                array.copy_from_slice(&value);
                Ok(Some(u64::from_be_bytes(array)))
            }

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
}

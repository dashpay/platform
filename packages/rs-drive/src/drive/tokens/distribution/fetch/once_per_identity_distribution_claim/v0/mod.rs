use crate::drive::tokens::paths::token_once_per_identity_distributions_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::prelude::{Identifier, TimestampMillis};
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;

impl Drive {
    /// Reads the claim item of `identity_id` under the token's once-per-identity distribution
    /// subtree. The item holds the claim's block time as 8 big-endian bytes.
    pub(super) fn fetch_once_per_identity_distribution_claim_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TimestampMillis>, Error> {
        let path = token_once_per_identity_distributions_path(&token_id);

        let raw = match self.grove_get_raw_optional(
            (&path).into(),
            identity_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(value, _))) => value,
            Ok(None) => return Ok(None),
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_)) => {
                return Ok(None)
            }
            Ok(Some(_)) => {
                return Err(Error::Drive(DriveError::CorruptedElementType(
                    "once-per-identity distribution claim was present but was not an item",
                )))
            }
            Err(e) => return Err(e),
        };

        let bytes: [u8; 8] = raw.as_slice().try_into().map_err(|_| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "once-per-identity distribution claim should be 8 bytes, found {}",
                raw.len()
            )))
        })?;

        Ok(Some(TimestampMillis::from_be_bytes(bytes)))
    }
}

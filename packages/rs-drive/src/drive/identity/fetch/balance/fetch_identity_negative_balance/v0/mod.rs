use crate::drive::identity::{identity_path, IdentityRootStructure};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::fee::Credits;

use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's negative balance operations from the backing store.
    pub(super) fn fetch_identity_negative_balance_operations_v0(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            // 8 is the size of a encoded u64
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: true,
                query_target: QueryTargetValue(8),
            }
        };

        let identity_path = identity_path(identity_id.as_slice());

        match self.grove_get_raw(
            (&identity_path).into(),
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeNegativeCredit),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(encoded_balance, _))) => {
                let balance = Credits::from_be_bytes(
                    encoded_balance.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(String::from(
                            "negative balance must be u64",
                        )))
                    })?,
                );

                Ok(Some(balance as Credits))
            }

            Ok(None) | Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))) => {
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity balance was present but was not identified as an item",
            ))),

            Err(e) => Err(e),
        }
    }
}

use crate::drive::identity::key::budget::identity_key_budgets_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::fee::Credits;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use integer_encoding::VarInt;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_identity_key_remaining_budget_operations_v0(
        &self,
        identity_id: [u8; 32],
        key_id: KeyID,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        let key_budgets_path = identity_key_budgets_path(identity_id.as_slice());

        match self.grove_get_raw_optional(
            (&key_budgets_path).into(),
            key_id.encode_var_vec().as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(encoded_budget, _))) => Ok(Some(Credits::from_be_bytes(
                encoded_budget.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(String::from(
                        "the remaining budget of a key must be a u64",
                    )))
                })?,
            ))),
            Ok(None) => Ok(None),
            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "the remaining budget of a key was present but was not an item",
            ))),
            // An identity that was never given a budgeted key has no key budgets subtree.
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathKeyNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathNotFound(_)
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

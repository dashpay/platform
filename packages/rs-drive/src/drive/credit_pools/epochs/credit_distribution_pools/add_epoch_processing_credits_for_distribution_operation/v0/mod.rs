use grovedb::{Element, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::credit_pools::epochs::epoch_key_constants;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Gets the amount of processing fees to be distributed for the Epoch and adds to it.
    pub(super) fn add_epoch_processing_credits_for_distribution_operation_v0(
        &self,
        epoch: &Epoch,
        amount: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<LowLevelDriveOperation, Error> {
        let epoch_tree_path = epoch.get_path();
        let element = self.grove_get_raw_optional(
            (&epoch_tree_path).into(),
            epoch_key_constants::KEY_POOL_PROCESSING_FEES.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        let existing_value = match element {
            None => 0,
            Some(Element::SumItem(existing_value, _)) => existing_value,
            _ => {
                return Err(Error::Drive(DriveError::UnexpectedElementType(
                    "epochs processing fee must be an item",
                )))
            }
        };

        if amount > i64::MAX as u64 {
            return Err(Error::Protocol(ProtocolError::Overflow(
                "adding over i64::Max to processing fee pool",
            )));
        }

        let updated_value = existing_value
            .checked_add(amount as i64)
            .ok_or(ProtocolError::Overflow("overflow when adding to sum item"))?;
        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            epoch_tree_path.iter().map(|a| a.to_vec()).collect(),
            epoch_key_constants::KEY_POOL_PROCESSING_FEES.to_vec(),
            Element::new_sum_item(updated_value),
        ))
    }
}

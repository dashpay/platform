use crate::drive::identity::key::budget::identity_key_budgets_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use integer_encoding::VarInt;

impl Drive {
    #[inline(always)]
    pub(super) fn add_to_identity_key_budget_operations_v0(
        &self,
        identity_id: [u8; 32],
        key_id: KeyID,
        amount: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<LowLevelDriveOperation>, Credits), Error> {
        let mut drive_operations = vec![];
        let previous_remaining_budget = self
            .fetch_identity_key_remaining_budget_operations(
                identity_id,
                key_id,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(format!(
                "key {} is budgeted but has no remaining budget in state",
                key_id
            ))))?;

        // The remaining budget never exceeds the total budget, and the total budget is raised by
        // the same amount, so this cannot overflow.
        let remaining_budget =
            previous_remaining_budget
                .checked_add(amount)
                .ok_or(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "the remaining budget of key {} overflows when {} credits are added",
                    key_id, amount
                ))))?;
        if remaining_budget == previous_remaining_budget {
            return Ok((drive_operations, remaining_budget));
        }

        drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
            identity_key_budgets_path_vec(identity_id.as_slice()),
            key_id.encode_var_vec(),
            Element::new_item(remaining_budget.to_be_bytes().to_vec()),
        ));

        Ok((drive_operations, remaining_budget))
    }
}

use crate::drive::Drive;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The method to add balance to the previous balance. This function is version controlled.
    pub(super) fn add_to_previous_token_balance_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        previous_balance: Credits,
        added_balance: Credits,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<AddToPreviousBalanceOutcome, Error> {
        // Deduct added balance from existing one
        let new_balance = previous_balance
            .checked_add(added_balance)
            .ok_or(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "identity balance add overflow error",
            )))?;

        Ok(AddToPreviousBalanceOutcomeV0 {
            balance_modified: Some(new_balance),
            negative_credit_balance_modified: None,
        }
        .into())
    }
}

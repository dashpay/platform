use crate::drive::identity::update::add_to_previous_balance_outcome::{
    AddToPreviousBalanceOutcome, AddToPreviousBalanceOutcomeV0,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::MAX_CREDITS;
use dpp::fee::Credits;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The method to add balance to the previous balance. This function is version controlled.
    pub(super) fn add_to_previous_balance_v0(
        &self,
        identity_id: [u8; 32],
        previous_balance: Credits,
        added_balance: Credits,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<AddToPreviousBalanceOutcome, Error> {
        if previous_balance == 0 {
            // Deduct debt from added amount if exists
            let negative_balance = self
                .fetch_identity_negative_balance_operations(
                    identity_id,
                    apply,
                    transaction,
                    drive_operations,
                    platform_version,
                )?
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "there should always be a balance if apply is set to true",
                )))?;

            if apply {
                if negative_balance > added_balance {
                    Ok(AddToPreviousBalanceOutcomeV0 {
                        balance_modified: None,
                        negative_credit_balance_modified: Some(negative_balance - added_balance),
                    }
                    .into())
                } else {
                    let negative_credit_balance_modified =
                        if negative_balance > 0 { Some(0) } else { None };

                    Ok(AddToPreviousBalanceOutcomeV0 {
                        balance_modified: Some(added_balance - negative_balance),
                        negative_credit_balance_modified,
                    }
                    .into())
                }
            } else {
                // For dry run we want worst possible case + some room for tests (1000)
                Ok(AddToPreviousBalanceOutcomeV0 {
                    balance_modified: Some(MAX_CREDITS - 1000),
                    negative_credit_balance_modified: Some(0),
                }
                .into())
            }
        } else {
            // Deduct added balance from existing one
            let new_balance =
                previous_balance
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
}

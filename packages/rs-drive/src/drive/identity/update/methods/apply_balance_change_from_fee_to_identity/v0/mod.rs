use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::state_transition::fee::fee_result::{BalanceChange, BalanceChangeForIdentity, FeeResult};
use dpp::version::drive_versions::DriveVersion;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::error::identity::IdentityError;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id
    pub(super) fn apply_balance_change_from_fee_to_identity_v0(
        &self,
        balance_change: BalanceChangeForIdentity,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<ApplyBalanceChangeOutcome, Error> {
        let (batch_operations, actual_fee_paid) =
            self.apply_balance_change_from_fee_to_identity_operations_v0(balance_change, transaction, drive_version)?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut drive_operations,
            drive_version,
        )?;

        Ok(ApplyBalanceChangeOutcome { actual_fee_paid })
    }


    /// Applies a balance change based on Fee Result
    /// If calculated balance is below 0 it will go to negative balance
    ///
    /// Balances are stored in the identity under key 0
    pub(super) fn apply_balance_change_from_fee_to_identity_operations_v0(
        &self,
        balance_change: BalanceChangeForIdentity,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(Vec<LowLevelDriveOperation>, FeeResult), Error> {
        let mut drive_operations = vec![];

        if matches!(balance_change.change(), BalanceChange::NoBalanceChange) {
            return Ok((drive_operations, balance_change.into_fee_result()));
        }

        // Update identity's balance according to calculated fees
        let previous_balance = self
            .fetch_identity_balance_operations(
                balance_change.identity_id.to_buffer(),
                true,
                transaction,
                &mut drive_operations,
                drive_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance if apply is set to true",
            )))?;

        let AddToPreviousBalanceOutcomeV0 {
            balance_modified,
            negative_credit_balance_modified,
        } = match balance_change.change() {
            BalanceChange::AddToBalance(balance_to_add) => self.add_to_previous_balance(
                balance_change.identity_id.to_buffer(),
                previous_balance,
                *balance_to_add,
                true,
                transaction,
                &mut drive_operations,
                drive_version,
            )?,
            BalanceChange::RemoveFromBalance {
                required_removed_balance,
                desired_removed_balance,
            } => {
                if *desired_removed_balance > previous_balance {
                    // we do not have enough balance
                    // there is a part we absolutely need to pay for
                    if *required_removed_balance > previous_balance {
                        return Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                            format!(
                                "identity with balance {} does not have the required balance {}",
                                previous_balance, *required_removed_balance
                            ),
                        )));
                    }
                    AddToPreviousBalanceOutcome {
                        balance_modified: Some(0),
                        negative_credit_balance_modified: Some(
                            *desired_removed_balance - previous_balance,
                        ),
                    }
                } else {
                    // we have enough balance
                    AddToPreviousBalanceOutcome {
                        balance_modified: Some(previous_balance - desired_removed_balance),
                        negative_credit_balance_modified: None,
                    }
                }
            }
            BalanceChange::NoBalanceChange => unreachable!(),
        };

        if let Some(new_balance) = balance_modified {
            drive_operations.push(
                self.update_identity_balance_operation(balance_change.identity_id, new_balance, drive_version)?,
            );
        }

        if let Some(new_negative_balance) = negative_credit_balance_modified {
            drive_operations.push(self.update_identity_negative_credit_operation(
                balance_change.identity_id,
                new_negative_balance,
                drive_version,
            ));
        }

        // Update other refunded identity balances
        for (identity_id, credits) in balance_change.other_refunds() {
            let mut estimated_costs_only_with_layer_info =
                None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>;

            drive_operations.extend(self.add_to_identity_balance_operations(
                identity_id,
                credits,
                &mut estimated_costs_only_with_layer_info,
                transaction,
                drive_version,
            )?);
        }

        Ok((
            drive_operations,
            balance_change.fee_result_outcome(previous_balance)?,
        ))
    }
}

use crate::drive::balances::balance_path_vec;
use crate::drive::block_info::BlockInfo;
use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::credits::MAX_CREDITS;
use crate::fee::op::DriveOperation;
use crate::fee::result::{BalanceChangeForIdentity, FeeChangeForIdentity, FeeResult};
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// The outcome of paying for a fee
pub struct FeeRemovalOutcome {
    /// The cost of the removal
    pub cost_of_removal: Option<FeeResult>,
    /// The actual fee paid by the identity
    pub actual_fee_paid: FeeResult,
}

impl Drive {
    /// We can set an identities balance
    pub(crate) fn update_identity_balance_operation(
        &self,
        identity_id: [u8; 32],
        balance: u64,
        is_replace: bool,
    ) -> Result<DriveOperation, Error> {
        let balance_path = balance_path_vec();
        // while i64::MAX could potentially work, best to avoid it.
        if balance >= MAX_CREDITS {
            Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set balance to over max credits amount (i64::MAX)",
            )))
        } else if is_replace {
            Ok(DriveOperation::replace_for_known_path_key_element(
                balance_path,
                identity_id.to_vec(),
                Element::new_sum_item(balance as i64),
            ))
        } else {
            Ok(DriveOperation::insert_for_known_path_key_element(
                balance_path,
                identity_id.to_vec(),
                Element::new_sum_item(balance as i64),
            ))
        }
    }

    /// We can set an identities negative credit balance
    pub(super) fn update_identity_negative_credit_operation(
        &self,
        identity_id: [u8; 32],
        negative_credit: u64,
    ) -> DriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let new_negative_credit_bytes = negative_credit.to_be_bytes().to_vec();
        DriveOperation::insert_for_known_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeNegativeCredit).to_vec(),
            Element::new_item(new_negative_credit_bytes),
        )
    }

    /// Balances are stored in the balance tree under the identity's id
    pub fn add_to_identity_balance(
        &self,
        identity_id: [u8; 32],
        added_balance: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.add_to_identity_balance_operations(
            identity_id,
            added_balance,
            &mut estimated_costs_only_with_layer_info,
            transaction,
        )?;

        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.apply_batch_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
        )?;

        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;

        Ok(fees)
    }

    /// Balances are stored in the balance tree under the identity's id
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn add_to_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        added_balance: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(estimated_costs_only_with_layer_info);
        }

        let previous_balance = self.fetch_identity_balance_operations(
            identity_id,
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            &mut drive_operations,
        )?;

        let new_balance = if estimated_costs_only_with_layer_info.is_none() {
            previous_balance
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "there should always be a balance if apply is set to true",
                )))?
                .checked_add(added_balance)
                .ok_or(Error::Identity(IdentityError::CriticalBalanceOverflow(
                    "identity overflow error",
                )))?
        } else {
            // Leave some room for tests
            MAX_CREDITS - 1000
        };

        drive_operations.push(self.update_identity_balance_operation(
            identity_id,
            new_balance,
            true,
        )?);
        Ok(drive_operations)
    }

    /// Balances are stored in the balance tree under the identity's id
    pub fn apply_balance_change_from_fee_to_identity(
        &self,
        identity_id: [u8; 32],
        balance_change_from_fee: FeeChangeForIdentity,
        block_info: &BlockInfo,
        apply: bool,
        should_return_cost: bool,
        transaction: TransactionArg,
    ) -> Result<FeeRemovalOutcome, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let (batch_operations, actual_fee_paid) = self
            .apply_balance_change_from_fee_to_identity_operations(
                identity_id,
                balance_change_from_fee,
                &mut estimated_costs_only_with_layer_info,
                transaction,
            )?;

        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.apply_batch_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
        )?;

        let cost_of_removal = if should_return_cost {
            Some(calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
            )?)
        } else {
            None
        };
        Ok(FeeRemovalOutcome {
            cost_of_removal,
            actual_fee_paid,
        })
    }

    /// Balances are stored in the identity under key 0
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn apply_balance_change_from_fee_to_identity_operations(
        &self,
        identity_id: [u8; 32],
        balance_change_from_fee: FeeChangeForIdentity,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<(Vec<DriveOperation>, FeeResult), Error> {
        let mut drive_operations = vec![];
        if matches!(
            balance_change_from_fee.balance_change,
            BalanceChangeForIdentity::NoBalanceChange
        ) {
            return Ok((drive_operations, balance_change_from_fee.into_fee_result()));
        }
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(estimated_costs_only_with_layer_info);
        }

        let previous_balance = if estimated_costs_only_with_layer_info.is_none() {
            self.fetch_identity_balance_operations(
                identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance if apply is set to true",
            )))?
        } else {
            MAX_CREDITS
        };

        let (new_balance, negative_credit_amount) = match balance_change_from_fee.balance_change {
            BalanceChangeForIdentity::AddBalanceChange { balance_to_add } => {
                let new_balance = previous_balance
                    .checked_add(balance_to_add)
                    .ok_or(Error::Fee(FeeError::Overflow(
                    "add balance change overflow on paying for an operation (by getting refunds)",
                )))?;
                (new_balance, None)
            }
            BalanceChangeForIdentity::RemoveBalanceChange {
                required_removed_balance,
                desired_removed_balance,
            } => {
                if desired_removed_balance > previous_balance {
                    // we do not have enough balance
                    // there is a part we absolutely need to pay for
                    if required_removed_balance > previous_balance {
                        return Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                            "identity does not have the required balance",
                        )));
                    }
                    (0, Some(desired_removed_balance - previous_balance))
                } else {
                    // we have enough balance
                    (previous_balance - desired_removed_balance, None)
                }
            }
            BalanceChangeForIdentity::NoBalanceChange => {
                unreachable!()
            }
        };

        drive_operations.push(self.update_identity_balance_operation(
            identity_id,
            new_balance,
            true,
        )?);

        if let Some(negative_credit_amount) = negative_credit_amount {
            drive_operations.push(
                self.update_identity_negative_credit_operation(identity_id, negative_credit_amount),
            );
        }

        Ok((
            drive_operations,
            balance_change_from_fee.fee_result_outcome(previous_balance)?,
        ))
    }

    /// Balances are stored in the balance tree under the identity's id
    pub fn remove_from_identity_balance(
        &self,
        identity_id: [u8; 32],
        balance_to_remove: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.remove_from_identity_balance_operations(
            identity_id,
            balance_to_remove,
            &mut estimated_costs_only_with_layer_info,
            transaction,
        )?;

        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.apply_batch_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
        )?;

        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }

    /// Balances are stored in the identity under key 0
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn remove_from_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        balance_to_remove: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(estimated_costs_only_with_layer_info);
        }

        let previous_balance = if estimated_costs_only_with_layer_info.is_none() {
            self.fetch_identity_balance_operations(
                identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance if apply is set to true",
            )))?
        } else {
            MAX_CREDITS
        };

        // we do not have enough balance
        // there is a part we absolutely need to pay for
        if balance_to_remove > previous_balance {
            return Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                "identity does not have the required balance",
            )));
        }

        drive_operations.push(self.update_identity_balance_operation(
            identity_id,
            previous_balance - balance_to_remove,
            true,
        )?);

        Ok(drive_operations)
    }
}

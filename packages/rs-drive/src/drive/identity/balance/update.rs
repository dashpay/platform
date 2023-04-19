use crate::drive::balances::balance_path_vec;
use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::credits::{Credits, MAX_CREDITS};
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::result::{BalanceChange, BalanceChangeForIdentity, FeeResult};
use dpp::block::block_info::BlockInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// The outcome of paying for a fee
pub struct ApplyBalanceChangeOutcome {
    /// The actual fee paid by the identity
    pub actual_fee_paid: FeeResult,
}

/// The outcome of adding to a previous balance
struct AddToPreviousBalanceOutcome {
    /// Is some if the balance was modified
    balance_modified: Option<Credits>,
    /// Is some if the negative credit balance was modified
    negative_credit_balance_modified: Option<Credits>,
}

impl Drive {
    /// Creates a balance key-value with specified amount
    /// Must be used only to create initial key-value. To update balance
    /// use `add_to_identity_balance`, `remove_from_identity_balance`,
    /// and `apply_balance_change_from_fee_to_identity`
    pub(in crate::drive::identity) fn insert_identity_balance_operation(
        &self,
        identity_id: [u8; 32],
        balance: Credits,
    ) -> Result<LowLevelDriveOperation, Error> {
        // while i64::MAX could potentially work, best to avoid it.
        if balance >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set balance to over max credits amount (i64::MAX)",
            )));
        };

        let balance_path = balance_path_vec();

        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            balance_path,
            identity_id.to_vec(),
            Element::new_sum_item(balance as i64),
        ))
    }

    pub(in crate::drive::identity) fn initialize_negative_identity_balance_operation(
        &self,
        identity_id: [u8; 32],
    ) -> LowLevelDriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());

        LowLevelDriveOperation::insert_for_known_path_key_element(
            identity_path,
            vec![IdentityRootStructure::IdentityTreeNegativeCredit as u8],
            Element::new_item(0u64.to_be_bytes().to_vec()),
        )
    }

    fn update_identity_balance_operation(
        &self,
        identity_id: [u8; 32],
        balance: Credits,
    ) -> Result<LowLevelDriveOperation, Error> {
        // while i64::MAX could potentially work, best to avoid it.
        if balance >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set balance to over max credits amount (i64::MAX)",
            )));
        }

        let balance_path = balance_path_vec();

        Ok(LowLevelDriveOperation::replace_for_known_path_key_element(
            balance_path,
            identity_id.to_vec(),
            Element::new_sum_item(balance as i64),
        ))
    }

    /// We can set an identities negative credit balance
    pub(in crate::drive::identity) fn update_identity_negative_credit_operation(
        &self,
        identity_id: [u8; 32],
        negative_credit: Credits,
    ) -> LowLevelDriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());

        // The value needs to be replaced without changing storage fees so we use bytes instead of varint
        let new_negative_credit_bytes = negative_credit.to_be_bytes().to_vec();

        LowLevelDriveOperation::replace_for_known_path_key_element(
            identity_path,
            vec![IdentityRootStructure::IdentityTreeNegativeCredit as u8],
            Element::new_item(new_negative_credit_bytes),
        )
    }

    /// Balances are stored in the balance tree under the identity's id
    pub fn add_to_identity_balance(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
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

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
        )?;

        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;

        Ok(fees)
    }

    fn add_to_previous_balance(
        &self,
        identity_id: [u8; 32],
        previous_balance: Credits,
        added_balance: Credits,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<AddToPreviousBalanceOutcome, Error> {
        if previous_balance == 0 {
            // Deduct debt from added amount if exists
            let negative_balance = self
                .fetch_identity_negative_balance_operations(
                    identity_id,
                    apply,
                    transaction,
                    drive_operations,
                )?
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "there should always be a balance if apply is set to true",
                )))?;

            if apply {
                if negative_balance > added_balance {
                    Ok(AddToPreviousBalanceOutcome {
                        balance_modified: None,
                        negative_credit_balance_modified: Some(negative_balance - added_balance),
                    })
                } else {
                    let negative_credit_balance_modified =
                        if negative_balance > 0 { Some(0) } else { None };

                    Ok(AddToPreviousBalanceOutcome {
                        balance_modified: Some(added_balance - negative_balance),
                        negative_credit_balance_modified,
                    })
                }
            } else {
                // For dry run we want worst possible case + some room for tests (1000)
                Ok(AddToPreviousBalanceOutcome {
                    balance_modified: Some(MAX_CREDITS - 1000),
                    negative_credit_balance_modified: Some(0),
                })
            }
        } else {
            // Deduct added balance from existing one
            let new_balance =
                previous_balance
                    .checked_add(added_balance)
                    .ok_or(Error::Identity(IdentityError::CriticalBalanceOverflow(
                        "identity balance add overflow error",
                    )))?;

            Ok(AddToPreviousBalanceOutcome {
                balance_modified: Some(new_balance),
                negative_credit_balance_modified: None,
            })
        }
    }

    /// Balances are stored in the balance tree under the identity's id
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn add_to_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(estimated_costs_only_with_layer_info);
            Self::add_estimation_costs_for_negative_credit(
                identity_id,
                estimated_costs_only_with_layer_info,
            );
        }

        let previous_balance = self
            .fetch_identity_balance_operations(
                identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance",
            )))?;

        let AddToPreviousBalanceOutcome {
            balance_modified,
            negative_credit_balance_modified,
        } = self.add_to_previous_balance(
            identity_id,
            previous_balance,
            added_balance,
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            &mut drive_operations,
        )?;

        if let Some(new_balance) = balance_modified {
            drive_operations
                .push(self.update_identity_balance_operation(identity_id, new_balance)?);
        }

        if let Some(new_negative_balance) = negative_credit_balance_modified {
            drive_operations.push(
                self.update_identity_negative_credit_operation(identity_id, new_negative_balance),
            );
        }

        Ok(drive_operations)
    }

    /// Balances are stored in the balance tree under the identity's id
    pub fn apply_balance_change_from_fee_to_identity(
        &self,
        balance_change: BalanceChangeForIdentity,
        transaction: TransactionArg,
    ) -> Result<ApplyBalanceChangeOutcome, Error> {
        let (batch_operations, actual_fee_paid) =
            self.apply_balance_change_from_fee_to_identity_operations(balance_change, transaction)?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut drive_operations,
        )?;

        Ok(ApplyBalanceChangeOutcome { actual_fee_paid })
    }

    /// Applies a balance change based on Fee Result
    /// If calculated balance is below 0 it will go to negative balance
    ///
    /// Balances are stored in the identity under key 0
    pub(crate) fn apply_balance_change_from_fee_to_identity_operations(
        &self,
        balance_change: BalanceChangeForIdentity,
        transaction: TransactionArg,
    ) -> Result<(Vec<LowLevelDriveOperation>, FeeResult), Error> {
        let mut drive_operations = vec![];

        if matches!(balance_change.change(), BalanceChange::NoBalanceChange) {
            return Ok((drive_operations, balance_change.into_fee_result()));
        }

        // Update identity's balance according to calculated fees
        let previous_balance = self
            .fetch_identity_balance_operations(
                balance_change.identity_id,
                true,
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance if apply is set to true",
            )))?;

        let AddToPreviousBalanceOutcome {
            balance_modified,
            negative_credit_balance_modified,
        } = match balance_change.change() {
            BalanceChange::AddToBalance(balance_to_add) => self.add_to_previous_balance(
                balance_change.identity_id,
                previous_balance,
                *balance_to_add,
                true,
                transaction,
                &mut drive_operations,
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
                self.update_identity_balance_operation(balance_change.identity_id, new_balance)?,
            );
        }

        if let Some(new_negative_balance) = negative_credit_balance_modified {
            drive_operations.push(self.update_identity_negative_credit_operation(
                balance_change.identity_id,
                new_negative_balance,
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
            )?);
        }

        Ok((
            drive_operations,
            balance_change.fee_result_outcome(previous_balance)?,
        ))
    }

    /// Balances are stored in the balance tree under the identity's id
    pub fn remove_from_identity_balance(
        &self,
        identity_id: [u8; 32],
        balance_to_remove: Credits,
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

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
        )?;

        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }

    /// Removes specified amount of credits from identity balance
    /// This function doesn't go below nil balance (negative balance)
    ///
    /// Balances are stored in the identity under key 0
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn remove_from_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        balance_to_remove: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(estimated_costs_only_with_layer_info);
            Self::add_estimation_costs_for_negative_credit(
                identity_id,
                estimated_costs_only_with_layer_info,
            );
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
                format!(
                    "identity with balance {} does not have the required balance to remove {}",
                    previous_balance, balance_to_remove
                ),
            )));
        }

        drive_operations.push(self.update_identity_balance_operation(
            identity_id,
            previous_balance - balance_to_remove,
        )?);

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::*;

    use dpp::block::epoch::Epoch;

    use crate::{
        common::helpers::identities::create_test_identity,
        tests::helpers::setup::setup_drive_with_initial_state_structure,
    };

    mod add_to_identity_balance {
        use super::*;

        #[test]
        fn should_add_to_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let old_balance = identity.balance;

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(identity.clone(), &block_info, true, None)
                .expect("expected to insert identity");

            let db_transaction = drive.grove.start_transaction();

            let amount = 300;

            let fee_result = drive
                .add_to_identity_balance(
                    identity.id.to_buffer(),
                    amount,
                    &block_info,
                    true,
                    Some(&db_transaction),
                )
                .expect("expected to add to identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 530020,
                    removed_bytes_from_system: 0,
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(identity.id.to_buffer(), &block_info, true, None)
                .expect("expected to get balance");

            assert_eq!(balance.unwrap(), old_balance + amount);
        }

        #[test]
        fn should_fail_if_balance_is_not_persisted() {
            let drive = setup_drive_with_initial_state_structure();

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let result = drive.add_to_identity_balance([0; 32], 300, &block_info, true, None);

            assert!(
                matches!(result, Err(Error::Drive(DriveError::CorruptedCodeExecution(m))) if m == "there should always be a balance")
            );
        }

        #[test]
        fn should_deduct_from_debt_if_balance_is_nil() {
            let drive = setup_drive_with_initial_state_structure();
            let identity = create_test_identity(&drive, [0; 32], Some(1), None);

            let added_balance = 300;
            let negative_amount = 100;

            // Persist negative balance
            let batch = vec![drive.update_identity_negative_credit_operation(
                identity.id.to_buffer(),
                negative_amount,
            )];

            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
            drive
                .apply_batch_low_level_drive_operations(None, None, batch, &mut drive_operations)
                .expect("should apply batch");

            let block_info = BlockInfo::default();

            let fee_result = drive
                .add_to_identity_balance(
                    identity.id.to_buffer(),
                    added_balance,
                    &block_info,
                    true,
                    None,
                )
                .expect("expected to add to identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 0,
                    processing_fee: 1201880,
                    removed_bytes_from_system: 0,
                    ..Default::default()
                }
            );

            let (updated_balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(identity.id.to_buffer(), &block_info, true, None)
                .expect("expected to get balance");

            assert_eq!(
                updated_balance.expect("balance should present"),
                added_balance - negative_amount
            );

            let updated_negative_balance = drive
                .fetch_identity_negative_balance_operations(
                    identity.id.to_buffer(),
                    true,
                    None,
                    &mut drive_operations,
                )
                .expect("expected to get balance")
                .expect("balance should present");

            assert_eq!(updated_negative_balance, 0)
        }

        #[test]
        fn should_keep_nil_balance_and_reduce_debt_if_added_balance_is_lower() {
            let drive = setup_drive_with_initial_state_structure();
            let identity = create_test_identity(&drive, [0; 32], Some(1), None);

            let added_balance = 50;
            let negative_amount = 100;

            // Persist negative balance
            let batch = vec![drive.update_identity_negative_credit_operation(
                identity.id.to_buffer(),
                negative_amount,
            )];

            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
            drive
                .apply_batch_low_level_drive_operations(None, None, batch, &mut drive_operations)
                .expect("should apply batch");

            let block_info = BlockInfo::default();

            let fee_result = drive
                .add_to_identity_balance(
                    identity.id.to_buffer(),
                    added_balance,
                    &block_info,
                    true,
                    None,
                )
                .expect("expected to add to identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 0,
                    processing_fee: 875150,
                    removed_bytes_from_system: 0,
                    ..Default::default()
                }
            );

            let (updated_balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(identity.id.to_buffer(), &block_info, true, None)
                .expect("expected to get balance");

            assert_eq!(updated_balance.expect("balance should present"), 0);

            let updated_negative_balance = drive
                .fetch_identity_negative_balance_operations(
                    identity.id.to_buffer(),
                    true,
                    None,
                    &mut drive_operations,
                )
                .expect("expected to get balance")
                .expect("balance should present");

            assert_eq!(updated_negative_balance, negative_amount - added_balance)
        }

        #[test]
        fn should_estimate_costs_without_state() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let app_hash_before = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            let fee_result = drive
                .add_to_identity_balance(identity.id.to_buffer(), 300, &block, false, None)
                .expect("expected to get estimated costs to update an identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 10175440,
                    ..Default::default()
                }
            );

            let app_hash_after = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(identity.id.to_buffer(), &block, true, None)
                .expect("expected to get balance");

            assert!(balance.is_none()); //shouldn't have changed
        }
    }

    mod remove_from_identity_balance {
        use super::*;

        #[test]
        fn should_remove_from_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let old_balance = identity.balance;

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(identity.clone(), &block, true, None)
                .expect("expected to insert identity");

            let db_transaction = drive.grove.start_transaction();

            let amount = 10;

            let fee_result = drive
                .remove_from_identity_balance(
                    identity.id.to_buffer(),
                    amount,
                    &block,
                    true,
                    Some(&db_transaction),
                )
                .expect("expected to add to identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 530020,
                    removed_bytes_from_system: 0,
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(identity.id.to_buffer(), &block, true, None)
                .expect("expected to get balance");

            assert_eq!(balance.unwrap(), old_balance - amount);
        }

        #[test]
        fn should_estimated_costs_without_state() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let app_hash_before = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            let amount = 10;

            let fee_result = drive
                .remove_from_identity_balance(identity.id.to_buffer(), amount, &block, false, None)
                .expect("expected to add to identity balance");

            let app_hash_after = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 5431170,
                    ..Default::default()
                }
            );

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(identity.id.to_buffer(), &block, true, None)
                .expect("expected to get balance");

            assert!(balance.is_none()); //shouldn't have changed
        }
    }

    mod apply_balance_change_from_fee_to_identity_operations {
        use super::*;
        use crate::common::helpers::identities::create_test_identity;
        use crate::fee::credits::SignedCredits;
        use crate::fee::epoch::{CreditsPerEpoch, GENESIS_EPOCH_INDEX};
        use crate::fee::result::refunds::{CreditsPerEpochByIdentifier, FeeRefunds};
        use grovedb::batch::Op;
        use nohash_hasher::IntMap;
        use std::collections::BTreeMap;

        #[test]
        fn should_do_nothing_if_there_is_no_balance_change() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None);

            let fee_result = FeeResult::default_with_fees(0, 0);
            let fee_change = fee_result
                .clone()
                .into_balance_change(identity.id.to_buffer());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            assert_eq!(drive_operations.len(), 0);
            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_add_to_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None);
            let other_identity = create_test_identity(&drive, [1; 32], Some(16), None);

            let removed_credits = 100000;
            let other_removed_credits = 200000;

            let credits_per_epoch: CreditsPerEpoch =
                IntMap::from_iter([(GENESIS_EPOCH_INDEX, removed_credits)]);

            let other_credits_per_epoch: CreditsPerEpoch =
                IntMap::from_iter([(GENESIS_EPOCH_INDEX, other_removed_credits)]);

            let refunds_per_epoch_by_identifier: CreditsPerEpochByIdentifier =
                BTreeMap::from_iter([
                    (identity.id.to_buffer(), credits_per_epoch),
                    (other_identity.id.to_buffer(), other_credits_per_epoch),
                ]);

            let fee_result = FeeResult {
                fee_refunds: FeeRefunds(refunds_per_epoch_by_identifier),
                ..Default::default()
            };
            let fee_change = fee_result
                .clone()
                .into_balance_change(identity.id.to_buffer());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            assert!(matches!(
                drive_operations[..],
                [
                    _,
                    _,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                        op: Op::Replace {
                            element: Element::SumItem(refund_amount, None),
                        },
                        ..
                    }),
                    ..,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                        op: Op::Replace {
                            element: Element::SumItem(other_refund_amount, None),
                        },
                        ..
                    })
                ] if refund_amount as Credits == removed_credits && other_refund_amount as Credits == other_removed_credits
            ));

            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_fail_if_balance_is_not_persisted() {
            let drive = setup_drive_with_initial_state_structure();

            let fee_result = FeeResult::default_with_fees(100000, 100);
            let fee_change = fee_result.into_balance_change([0; 32]);

            let result =
                drive.apply_balance_change_from_fee_to_identity_operations(fee_change, None);

            assert!(
                matches!(result, Err(Error::Drive(DriveError::CorruptedCodeExecution(m))) if m == "there should always be a balance if apply is set to true")
            );
        }

        #[test]
        fn should_deduct_from_debt_if_balance_is_nil() {
            let drive = setup_drive_with_initial_state_structure();

            let removed_credits = 10000;
            let negative_amount = 1000;

            let identity = create_test_identity(&drive, [0; 32], Some(15), None);

            // Persist negative balance
            let batch = vec![drive.update_identity_negative_credit_operation(
                identity.id.to_buffer(),
                negative_amount,
            )];

            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
            drive
                .apply_batch_low_level_drive_operations(None, None, batch, &mut drive_operations)
                .expect("should apply batch");

            let credits_per_epoch: CreditsPerEpoch =
                IntMap::from_iter([(GENESIS_EPOCH_INDEX, removed_credits)]);

            let refunds_per_epoch_by_identifier: CreditsPerEpochByIdentifier =
                BTreeMap::from_iter([(identity.id.to_buffer(), credits_per_epoch)]);

            let fee_result = FeeResult {
                fee_refunds: FeeRefunds(refunds_per_epoch_by_identifier),
                ..Default::default()
            };
            let fee_change = fee_result
                .clone()
                .into_balance_change(identity.id.to_buffer());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            assert!(matches!(
                &drive_operations[..],
                [
                    _,
                    _,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                        op: Op::Replace {
                            element: Element::SumItem(refund_amount, None),
                        },
                    ..
                    }),
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                        op: Op::Replace {
                            element: Element::Item(debt_bytes, None),
                        },
                        ..
                    })
                ] if *refund_amount as Credits == removed_credits - negative_amount && debt_bytes == &0u64.to_be_bytes()
            ));

            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_keep_nil_balance_and_reduce_debt_if_added_balance_is_lower() {
            let drive = setup_drive_with_initial_state_structure();

            let removed_credits = 1000;
            let negative_amount = 3000;

            let identity = create_test_identity(&drive, [0; 32], Some(15), None);

            // Persist negative balance
            let batch = vec![drive.update_identity_negative_credit_operation(
                identity.id.to_buffer(),
                negative_amount,
            )];

            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
            drive
                .apply_batch_low_level_drive_operations(None, None, batch, &mut drive_operations)
                .expect("should apply batch");

            let credits_per_epoch: CreditsPerEpoch =
                IntMap::from_iter([(GENESIS_EPOCH_INDEX, removed_credits)]);

            let refunds_per_epoch_by_identifier: CreditsPerEpochByIdentifier =
                BTreeMap::from_iter([(identity.id.to_buffer(), credits_per_epoch)]);

            let fee_result = FeeResult {
                fee_refunds: FeeRefunds(refunds_per_epoch_by_identifier),
                ..Default::default()
            };
            let fee_change = fee_result
                .clone()
                .into_balance_change(identity.id.to_buffer());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            assert!(matches!(
                &drive_operations[..],
                [
                    _,
                    _,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                        op: Op::Replace {
                            element: Element::Item(debt_bytes, None),
                        },
                        ..
                    })
                ] if debt_bytes == &2000u64.to_be_bytes()
            ));

            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_remove_from_balance_less_amount() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None);

            let initial_balance = 100;

            drive
                .add_to_identity_balance(
                    identity.id.to_buffer(),
                    initial_balance,
                    &BlockInfo::default(),
                    true,
                    None,
                )
                .expect("should set initial balance");

            let processing_fee = 20;
            let storage_fee = 10;

            let fee_result = FeeResult {
                processing_fee,
                storage_fee,
                ..Default::default()
            };

            let fee_change = fee_result
                .clone()
                .into_balance_change(identity.id.to_buffer());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            assert!(matches!(
                drive_operations[..],
                [_, LowLevelDriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                    op: Op::Replace {
                        element: Element::SumItem(balance, None),
                    },
                    ..
                })] if balance == (initial_balance - storage_fee - processing_fee) as SignedCredits
            ));

            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_remove_from_balance_bigger_amount_and_get_into_debt() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None);

            let initial_balance = 100;

            drive
                .add_to_identity_balance(
                    identity.id.to_buffer(),
                    initial_balance,
                    &BlockInfo::default(),
                    true,
                    None,
                )
                .expect("should set initial balance");

            let processing_fee = 110;
            let storage_fee = 80;

            let fee_result = FeeResult {
                processing_fee,
                storage_fee,
                ..Default::default()
            };

            let fee_change = fee_result.into_balance_change(identity.id.to_buffer());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            let expected_debt_bytes =
                (storage_fee + processing_fee - initial_balance).to_be_bytes();

            assert!(matches!(
                &drive_operations[..],
                [
                    _,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                        op: Op::Replace {
                            element: Element::SumItem(balance, None),
                        },
                        ..
                    }),
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                        op: Op::Replace {
                            element: Element::Item(debt_bytes, None),
                        },
                        ..
                    })
                ] if balance == &(0 as SignedCredits) && debt_bytes == &expected_debt_bytes
            ));

            assert_eq!(
                fee_result_outcome,
                FeeResult {
                    storage_fee,
                    processing_fee: initial_balance - storage_fee,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn should_return_error_if_required_amount_bigger_than_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None);

            let processing_fee = 110;
            let storage_fee = 80;

            let fee_result = FeeResult {
                processing_fee,
                storage_fee,
                ..Default::default()
            };

            let fee_change = fee_result.into_balance_change(identity.id.to_buffer());

            let result =
                drive.apply_balance_change_from_fee_to_identity_operations(fee_change, None);

            assert!(matches!(
                result,
                Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                    _
                )))
            ));
        }
    }
}

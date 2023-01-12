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

        // The value needs to be replaced without changing storage fees so we use bytes instead of varint
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
        balance_change_from_fee: FeeChangeForIdentity,
        transaction: TransactionArg,
    ) -> Result<FeeRemovalOutcome, Error> {
        let (batch_operations, actual_fee_paid) = self
            .apply_balance_change_from_fee_to_identity_operations(
                balance_change_from_fee,
                transaction,
            )?;

        let mut drive_operations: Vec<DriveOperation> = vec![];

        self.apply_batch_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut drive_operations,
        )?;

        Ok(FeeRemovalOutcome { actual_fee_paid })
    }

    /// Balances are stored in the identity under key 0
    pub(crate) fn apply_balance_change_from_fee_to_identity_operations(
        &self,
        balance_change_from_fee: FeeChangeForIdentity,
        transaction: TransactionArg,
    ) -> Result<(Vec<DriveOperation>, FeeResult), Error> {
        let mut drive_operations = vec![];

        if matches!(
            balance_change_from_fee.balance_change,
            BalanceChangeForIdentity::NoBalanceChange
        ) {
            return Ok((drive_operations, balance_change_from_fee.into_fee_result()));
        }

        let previous_balance = self
            .fetch_identity_balance_operations(
                balance_change_from_fee.identifier,
                true,
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance if apply is set to true",
            )))?;

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
            BalanceChangeForIdentity::NoBalanceChange => unreachable!(),
        };

        drive_operations.push(self.update_identity_balance_operation(
            balance_change_from_fee.identifier,
            new_balance,
            true,
        )?);

        if let Some(negative_credit_amount) = negative_credit_amount {
            drive_operations.push(self.update_identity_negative_credit_operation(
                balance_change_from_fee.identifier,
                negative_credit_amount,
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::*;

    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    use crate::fee_pools::epochs::Epoch;

    mod add_to_identity_balance {
        use super::*;

        #[test]
        fn should_add_to_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let old_balance = identity.balance;

            let block = BlockInfo::default_with_epoch(Epoch::new(0));

            drive
                .add_new_identity(identity.clone(), &block, true, None)
                .expect("expected to insert identity");

            let db_transaction = drive.grove.start_transaction();

            let amount = 300;

            let fee_result = drive
                .add_to_identity_balance(
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
                    processing_fee: 562420,
                    removed_bytes_from_system: 24, // TODO: That's fine?
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_fees(identity.id.to_buffer(), &block, true, None)
                .expect("expected to get balance");

            assert_eq!(balance.unwrap(), old_balance + amount);
        }

        #[test]
        fn should_estimated_costs_without_state() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block = BlockInfo::default_with_epoch(Epoch::new(0));

            let app_hash_before = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            let fee_result = drive
                .add_to_identity_balance(identity.id.to_buffer(), 300, &block, false, None)
                .expect("expected to get estimated costs to update an identity balance");

            let app_hash_after = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 5532770,
                    ..Default::default()
                }
            );

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_fees(identity.id.to_buffer(), &block, true, None)
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

            let block = BlockInfo::default_with_epoch(Epoch::new(0));

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
                    processing_fee: 562420,
                    removed_bytes_from_system: 24, // TODO: That's fine?
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_fees(identity.id.to_buffer(), &block, true, None)
                .expect("expected to get balance");

            assert_eq!(balance.unwrap(), old_balance - amount);
        }

        #[test]
        fn should_estimated_costs_without_state() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block = BlockInfo::default_with_epoch(Epoch::new(0));

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
                    processing_fee: 5430770,
                    ..Default::default()
                }
            );

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_fees(identity.id.to_buffer(), &block, true, None)
                .expect("expected to get balance");

            assert!(balance.is_none()); //shouldn't have changed
        }
    }

    mod apply_balance_change_from_fee_to_identity_operations {
        use super::*;
        use crate::common::helpers::identities::create_test_identity;
        use crate::fee::credits::SignedCredits;
        use crate::fee::epoch::distribution::calculate_storage_fee_distribution_amount_and_leftovers;
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
                .to_fee_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
                .expect("should calculate fee change for identity");

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

            let removed_credits = 10000;

            let credits_per_epoch: CreditsPerEpoch =
                IntMap::from_iter([(GENESIS_EPOCH_INDEX, removed_credits)]);

            let refunds_per_epoch_by_identifier: CreditsPerEpochByIdentifier =
                BTreeMap::from_iter([(identity.id.to_buffer(), credits_per_epoch)]);

            let fee_result = FeeResult {
                fee_refunds: FeeRefunds(refunds_per_epoch_by_identifier),
                ..Default::default()
            };
            let fee_change = fee_result
                .to_fee_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
                .expect("should calculate fee change for identity");

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            let (refund_amount, _) = calculate_storage_fee_distribution_amount_and_leftovers(
                removed_credits,
                GENESIS_EPOCH_INDEX,
                GENESIS_EPOCH_INDEX,
            )
            .expect("should calculate refund amount");

            assert!(matches!(
                drive_operations[..],
                [_, DriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                    op: Op::Replace {
                        element: Element::SumItem(balance, None),
                    },
                    ..
                })] if balance == refund_amount as SignedCredits
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
                .to_fee_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
                .expect("should calculate fee change for identity");

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            assert!(matches!(
                drive_operations[..],
                [_, DriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
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

            let fee_change = fee_result
                .to_fee_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
                .expect("should calculate fee change for identity");

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            let expected_debt_bytes =
                (storage_fee + processing_fee - initial_balance).to_be_bytes();

            assert!(matches!(
                &drive_operations[..],
                [_, DriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                    op: Op::Replace {
                        element: Element::SumItem(balance, None),
                    },
                    ..
                }),
                DriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                    op: Op::Insert {
                        element: Element::Item(debt_bytes, None),
                    },
                    ..
                })] if balance == &(0 as SignedCredits) && debt_bytes == &expected_debt_bytes
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

            let fee_change = fee_result
                .to_fee_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
                .expect("should calculate fee change for identity");

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

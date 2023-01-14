use crate::drive::balances::balance_path_vec;
use crate::drive::block_info::BlockInfo;
use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::credits::{Credits, MAX_CREDITS};
use crate::fee::op::DriveOperation;
use crate::fee::result::{BalanceChange, BalanceChangeForIdentity, FeeResult};
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// The outcome of paying for a fee
pub struct FeeRemovalOutcome {
    /// The actual fee paid by the identity
    pub actual_fee_paid: FeeResult,
}

impl Drive {
    fn replace_identity_balance_operation(
        &self,
        identity_id: [u8; 32],
        balance: Credits,
    ) -> Result<DriveOperation, Error> {
        // while i64::MAX could potentially work, best to avoid it.
        if balance >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set balance to over max credits amount (i64::MAX)",
            )));
        }

        let balance_path = balance_path_vec();

        Ok(DriveOperation::replace_for_known_path_key_element(
            balance_path,
            identity_id.to_vec(),
            Element::new_sum_item(balance as i64),
        ))
    }

    /// We can set an identities negative credit balance
    pub(crate) fn insert_identity_negative_credit_operation(
        &self,
        identity_id: [u8; 32],
        negative_credit: Credits,
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

    /// We can set an identities negative credit balance
    fn replace_identity_negative_credit_operation(
        &self,
        identity_id: [u8; 32],
        negative_credit: Credits,
    ) -> DriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());

        // The value needs to be replaced without changing storage fees so we use bytes instead of varint
        let new_negative_credit_bytes = negative_credit.to_be_bytes().to_vec();

        DriveOperation::replace_for_known_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeNegativeCredit).to_vec(),
            Element::new_item(new_negative_credit_bytes),
        )
    }

    /// Creates a balance key-value with specified amount
    /// Must be used only to create initial key-value. To update balance
    /// use `add_to_identity_balance`, `remove_from_identity_balance`,
    /// and `apply_balance_change_from_fee_to_identity`
    pub(crate) fn insert_identity_balance_operation(
        &self,
        identity_id: [u8; 32],
        balance: Credits,
    ) -> Result<DriveOperation, Error> {
        // while i64::MAX could potentially work, best to avoid it.
        if balance >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set balance to over max credits amount (i64::MAX)",
            )));
        };

        let balance_path = balance_path_vec();

        Ok(DriveOperation::insert_for_known_path_key_element(
            balance_path,
            identity_id.to_vec(),
            Element::new_sum_item(balance as i64),
        ))
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

    fn add_to_previous_balance_operations(
        &self,
        identity_id: [u8; 32],
        previous_balance: Credits,
        added_balance: Credits,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(Option<Credits>, Option<Credits>), Error> {
        if previous_balance == 0 {
            // Deduct debt from added amount if exists
            let negative_balance = self
                .fetch_identity_negative_balance_operations(
                    identity_id,
                    apply,
                    transaction,
                    drive_operations,
                )?
                .ok_or(Error::Identity(IdentityError::CriticalBalanceOverflow(
                    "added balance is lower than negative balance",
                )))?;

            if apply {
                if negative_balance > added_balance {
                    Ok((None, Some(negative_balance - added_balance)))
                } else {
                    Ok((Some(added_balance - negative_balance), None))
                }
            } else {
                // For dry run we want worst possible case + some room for tests (1000)
                Ok((Some(MAX_CREDITS - 1000), Some(0)))
            }
        } else {
            // Deduct added balance from existing one
            let new_balance =
                previous_balance
                    .checked_add(added_balance)
                    .ok_or(Error::Identity(IdentityError::CriticalBalanceOverflow(
                        "identity balance add overflow error",
                    )))?;

            Ok((Some(new_balance), None))
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
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(estimated_costs_only_with_layer_info);
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

        let (maybe_new_balance, maybe_new_negative_balance) = self
            .add_to_previous_balance_operations(
                identity_id,
                previous_balance,
                added_balance,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
            )?;

        if let Some(new_balance) = maybe_new_balance {
            drive_operations
                .push(self.replace_identity_balance_operation(identity_id, new_balance)?);
        }

        if let Some(new_negative_balance) = maybe_new_negative_balance {
            drive_operations.push(
                self.replace_identity_negative_credit_operation(identity_id, new_negative_balance),
            );
        }

        Ok(drive_operations)
    }

    /// Balances are stored in the balance tree under the identity's id
    pub fn apply_balance_change_from_fee_to_identity(
        &self,
        balance_change: BalanceChangeForIdentity,
        transaction: TransactionArg,
    ) -> Result<FeeRemovalOutcome, Error> {
        let (batch_operations, actual_fee_paid) =
            self.apply_balance_change_from_fee_to_identity_operations(balance_change, transaction)?;

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
        balance_change: BalanceChangeForIdentity,
        transaction: TransactionArg,
    ) -> Result<(Vec<DriveOperation>, FeeResult), Error> {
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

        let (maybe_new_balance, maybe_new_negative_balance) = match balance_change.change() {
            BalanceChange::AddToBalance(balance_to_add) => self
                .add_to_previous_balance_operations(
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
                            "identity does not have the required balance",
                        )));
                    }
                    (Some(0), Some(*desired_removed_balance - previous_balance))
                } else {
                    // we have enough balance
                    (Some(previous_balance - desired_removed_balance), None)
                }
            }
            BalanceChange::NoBalanceChange => unreachable!(),
        };

        if let Some(new_balance) = maybe_new_balance {
            drive_operations.push(
                self.replace_identity_balance_operation(balance_change.identity_id, new_balance)?,
            );
        }

        if let Some(new_negative_balance) = maybe_new_negative_balance {
            drive_operations.push(self.replace_identity_negative_credit_operation(
                balance_change.identity_id,
                new_negative_balance,
            ));
        }

        // Update other refunded identity balances
        for (identity_id, credits) in balance_change.other_refunds()? {
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
        balance_to_remove: Credits,
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

        drive_operations.push(self.replace_identity_balance_operation(
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

    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    use crate::fee_pools::epochs::Epoch;

    mod insert_identity_balance_operation {
        #[test]
        fn should_fail_if_balance_already_persisted() {
            todo!()
        }

        #[test]
        fn should_insert_balance() {
            todo!()
        }
    }

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
        fn should_fail_if_balance_is_not_persisted() {
            todo!()
        }

        #[test]
        fn should_deduct_from_debt_if_balance_is_nil() {
            todo!()
        }

        #[test]
        fn should_reduce_debt_if_added_balance_is_lower() {
            todo!()
        }

        #[test]
        fn should_estimate_costs_without_state() {
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
                    processing_fee: 5609970,
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
                .to_balance_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
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
                .to_balance_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
                .expect("should calculate fee change for identity");

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(fee_change, None)
                .expect("should apply fee change");

            assert!(matches!(
                drive_operations[..],
                [
                    _,
                    _,
                    DriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                        op: Op::Replace {
                            element: Element::SumItem(99390, None),
                        },
                        ..
                    }),
                    ..,
                    DriveOperation::GroveOperation(grovedb::batch::GroveDbOp {
                        op: Op::Replace {
                            element: Element::SumItem(199320, None),
                        },
                        ..
                    })
                ]
            ));

            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_fail_if_balance_is_not_persisted() {
            todo!()
        }

        #[test]
        fn should_deduct_from_debt_if_balance_is_nil() {
            todo!()
        }

        #[test]
        fn should_reduce_debt_if_added_balance_is_lower() {
            todo!()
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
                .to_balance_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
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
                .to_balance_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
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
                .to_balance_change(identity.id.to_buffer(), GENESIS_EPOCH_INDEX)
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

use crate::drive::block_info::BlockInfo;

use crate::drive::flags::StorageFlags;
use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use grovedb::batch::KeyInfoPath;

use crate::drive::balances::balance_path_vec;
use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
};
use crate::fee::credits::MAX_CREDITS;
use crate::fee::result::FeeResult;
use crate::fee_pools::epochs::Epoch;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::{Revision, TimestampMillis};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    /// We can set an identities balance
    pub(super) fn update_identity_balance_operation(
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

    /// Update revision for specific identity
    pub fn update_identity_revision(
        &self,
        identity_id: [u8; 32],
        revision: Revision,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];

        // TODO: In case of dry run we will get less because we replace the same bytes

        let estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        batch_operations.push(self.update_revision_operation(identity_id, revision));

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

    /// Update the revision of the identity
    /// Revisions get bumped on all changes except for the balance and negative credit fields
    pub(super) fn update_revision_operation(
        &self,
        identity_id: [u8; 32],
        revision: Revision,
    ) -> DriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let revision_bytes = revision.encode_var_vec();
        DriveOperation::insert_for_known_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeRevision).to_vec(),
            Element::new_item(revision_bytes),
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
        let mut batch_operations: Vec<DriveOperation> = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        self.add_to_identity_balance_operations(
            identity_id,
            added_balance,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
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
    pub fn add_to_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        added_balance: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(estimated_costs_only_with_layer_info);
        }

        let previous_balance = self.fetch_identity_balance_operations(
            identity_id,
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            drive_operations,
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
        Ok(())
    }

    /// Balances are stored in the balance tree under the identity's id
    pub fn remove_from_identity_balance(
        &self,
        identity_id: [u8; 32],
        required_removed_balance: u64,      // storage_fee - refunds
        total_desired_removed_balance: u64, // storage_fee + processing fees - refunds
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];

        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        self.remove_from_identity_balance_operations(
            identity_id,
            required_removed_balance,
            total_desired_removed_balance,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
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
    pub fn remove_from_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        required_removed_balance: u64,
        total_desired_removed_balance: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(estimated_costs_only_with_layer_info);
        }

        let previous_balance = if estimated_costs_only_with_layer_info.is_none() {
            self.fetch_identity_balance_operations(
                identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance if apply is set to true",
            )))?
        } else {
            MAX_CREDITS
        };

        let (new_balance, negative_credit_amount) =
            if total_desired_removed_balance > previous_balance {
                // we do not have enough balance
                // there is a part we absolutely need to pay for
                if required_removed_balance > previous_balance {
                    return Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                        "identity does not have the required balance",
                    )));
                }
                (0, Some(total_desired_removed_balance - previous_balance))
            } else {
                // we have enough balance
                (previous_balance - total_desired_removed_balance, None)
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

        Ok(())
    }

    /// Add new keys to an identity
    pub fn add_new_keys_to_identity(
        &self,
        identity_id: [u8; 32],
        keys_to_add: Vec<IdentityPublicKey>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        self.add_new_keys_to_identity_operations(
            identity_id,
            keys_to_add,
            &block_info.epoch,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
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

    /// Disable identity keys
    pub fn disable_identity_keys(
        &self,
        identity_id: [u8; 32],
        keys_ids: Vec<KeyID>,
        disable_at: TimestampMillis,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        self.disable_identity_keys_operations(
            identity_id,
            keys_ids,
            disable_at,
            &block_info.epoch,
            &mut estimated_costs_only_with_layer_info,
            apply,
            transaction,
            &mut batch_operations,
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

    pub(crate) fn disable_identity_keys_operations(
        &self,
        identity_id: [u8; 32],
        key_ids: Vec<KeyID>,
        disable_at: TimestampMillis,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
            );
        }

        let key_ids_len = key_ids.len();

        let key_request = IdentityKeysRequest {
            identity_id,
            request_type: KeyRequestType::SpecificKeys(key_ids.clone()),
            limit: Some(key_ids_len as u16),
            offset: None,
        };

        let keys: KeyIDIdentityPublicKeyPairVec = if apply {
            self.fetch_identity_keys_operations(key_request, transaction, drive_operations)?
        } else {
            key_ids
                .into_iter()
                .map(|key_id| (key_id, IdentityPublicKey::max_possible_size_key(key_id)))
                .collect()
        };

        if keys.len() != key_ids_len {
            // TODO Choose / add an appropriate error
            return Err(Error::Drive(DriveError::UpdatingDocumentThatDoesNotExist(
                "key to disable with specified ID is not found",
            )));
        }

        for (_, mut key) in keys {
            key.set_disabled_at(disable_at);

            let key_id_bytes = key.id.encode_var_vec();

            self.replace_key_in_storage_operations(
                identity_id.as_slice(),
                &key,
                &key_id_bytes,
                &StorageFlags::SingleEpoch(epoch.index),
                estimated_costs_only_with_layer_info,
                drive_operations,
            )?;
        }

        Ok(())
    }

    /// The operations for adding new keys to an identity
    pub fn add_new_keys_to_identity_operations(
        &self,
        identity_id: [u8; 32],
        keys_to_add: Vec<IdentityPublicKey>,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
            );
        }

        for key in keys_to_add {
            self.insert_new_key_operations(
                identity_id.as_slice(),
                key,
                &StorageFlags::SingleEpoch(epoch.index),
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::*;

    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

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
                    processing_fee: 620020,
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

    mod add_new_keys_to_identity {
        use super::*;

        #[test]
        fn should_add_one_new_key_to_identity() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block = BlockInfo::default_with_epoch(Epoch::new(0));

            drive
                .add_new_identity(identity.clone(), &block, true, None)
                .expect("expected to insert identity");

            let new_keys_to_add = IdentityPublicKey::random_keys(5, 1, Some(15));

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .add_new_keys_to_identity(
                    identity.id.to_buffer(),
                    new_keys_to_add,
                    &block,
                    true,
                    Some(&db_transaction),
                )
                .expect("expected to update identity with new keys");

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 15498000,
                    processing_fee: 2642980,
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let identity_keys = drive
                .fetch_all_identity_keys(identity.id.to_buffer(), None)
                .expect("expected to get balance");

            assert_eq!(identity_keys.len(), 6); // we had 5 keys and we added 1
        }

        #[test]
        fn should_add_two_dozen_new_keys_to_identity() {
            let drive = setup_drive_with_initial_state_structure();

            drive
                .create_initial_state_structure(None)
                .expect("expected to create root tree successfully");

            let identity = Identity::random_identity(5, Some(12345));

            let block = BlockInfo::default_with_epoch(Epoch::new(0));

            drive
                .add_new_identity(identity.clone(), &block, true, None)
                .expect("expected to insert identity");

            let new_keys_to_add = IdentityPublicKey::random_keys(5, 24, Some(15));

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .add_new_keys_to_identity(
                    identity.id.to_buffer(),
                    new_keys_to_add,
                    &block,
                    true,
                    Some(&db_transaction),
                )
                .expect("expected to update identity with new keys");

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 390636000,
                    processing_fee: 10289310,
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let identity_keys = drive
                .fetch_all_identity_keys(identity.id.to_buffer(), None)
                .expect("expected to get balance");

            assert_eq!(identity_keys.len(), 29); // we had 5 keys and we added 24
        }

        #[test]
        fn should_estimated_costs_without_state() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block = BlockInfo::default_with_epoch(Epoch::new(0));

            let new_keys_to_add = IdentityPublicKey::random_keys(5, 1, Some(15));

            let app_hash_before = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            let fee_result = drive
                .add_new_keys_to_identity(
                    identity.id.to_buffer(),
                    new_keys_to_add,
                    &block,
                    false,
                    None,
                )
                .expect("expected to update identity with new keys");

            let app_hash_after = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 15498000,
                    processing_fee: 2642980,
                    ..Default::default()
                }
            );
        }
    }

    mod disable_identity_keys {
        use super::*;
        use chrono::Utc;

        #[test]
        fn should_disable_a_few_keys() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0));

            drive
                .add_new_identity(identity.clone(), &block_info, true, None)
                .expect("expected to insert identity");

            let new_keys_to_add = IdentityPublicKey::random_keys(5, 2, Some(15));

            drive
                .add_new_keys_to_identity(
                    identity.id.to_buffer(),
                    new_keys_to_add.clone(),
                    &block_info,
                    true,
                    None,
                )
                .expect("expected to update identity with new keys");

            let db_transaction = drive.grove.start_transaction();

            let key_ids = new_keys_to_add.into_iter().map(|key| key.id).collect();

            let disable_at = Utc::now().timestamp_millis() as TimestampMillis;

            let fee_result = drive
                .disable_identity_keys(
                    identity.id.to_buffer(),
                    key_ids,
                    disable_at,
                    &block_info,
                    true,
                    Some(&db_transaction),
                )
                .expect("should disable a few keys");

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 513000,
                    processing_fee: 1787720,
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let identity_keys = drive
                .fetch_all_identity_keys(identity.id.to_buffer(), None)
                .expect("expected to get balance");

            assert_eq!(identity_keys.len(), 7); // we had 5 keys and we added 2

            for (_, key) in identity_keys.into_iter().skip(5) {
                assert_eq!(key.disabled_at, Some(disable_at));
            }
        }

        #[test]
        fn should_estimated_costs_without_state() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0));

            let disable_at = Utc::now().timestamp_millis() as TimestampMillis;

            let app_hash_before = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            let fee_result = drive
                .disable_identity_keys(
                    identity.id.to_buffer(),
                    vec![0, 1],
                    disable_at,
                    &block_info,
                    false,
                    None,
                )
                .expect("should estimate the disabling of a few keys");

            let app_hash_after = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 10368000,
                    processing_fee: 5877330,
                    ..Default::default()
                }
            );

            todo!("fees shouldn't be so big")
        }
    }

    mod update_identity_revision {
        use super::*;

        #[test]
        fn should_update_revision() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0));

            drive
                .add_new_identity(identity.clone(), &block_info, true, None)
                .expect("expected to insert identity");

            let revision = 2;

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .update_identity_revision(
                    identity.id.to_buffer(),
                    revision,
                    &block_info,
                    true,
                    Some(&db_transaction),
                )
                .expect("should update revision");

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 0,
                    processing_fee: 832780,
                    removed_bytes_from_system: 8,
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let updated_revision = drive
                .fetch_identity_revision(identity.id.to_buffer(), true, None)
                .expect("expected to get revision");

            assert_eq!(updated_revision, Some(revision));
        }

        #[test]
        fn should_estimated_costs_without_state() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0));

            let revision = 2;

            let app_hash_before = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            let fee_result = drive
                .update_identity_revision(
                    identity.id.to_buffer(),
                    revision,
                    &block_info,
                    false,
                    None,
                )
                .expect("should estimate the revision update");

            let app_hash_after = drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 0,
                    processing_fee: 832780,
                    removed_bytes_from_system: 8,
                    ..Default::default()
                }
            );
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
                    amount,
                    &block,
                    true,
                    Some(&db_transaction),
                )
                .expect("expected to add to identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 620020,
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
                .remove_from_identity_balance(
                    identity.id.to_buffer(),
                    amount,
                    amount,
                    &block,
                    false,
                    None,
                )
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
}

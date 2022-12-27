use crate::drive::block_info::BlockInfo;

use crate::drive::flags::StorageFlags;
use crate::drive::identity::{balance_path_vec, identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use grovedb::batch::KeyInfoPath;

use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
};
use crate::fee::result::FeeResult;
use crate::fee_pools::epochs::Epoch;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::TimestampMillis;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    /// We can set an identities balance
    pub(super) fn set_identity_balance_operation(
        &self,
        identity_id: [u8; 32],
        balance: u64,
        is_replace: bool,
    ) -> Result<DriveOperation, Error> {
        let balance_path = balance_path_vec();
        // while i64::MAX could potentially work, best to avoid it.
        if balance >= i64::MAX as u64 {
            Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set balance to over i64::Max",
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
    pub(super) fn set_identity_negative_credit_operation(
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

    /// Update the revision of the identity
    /// Revisions get bumped on all changes except for the balance and negative credit fields
    pub(super) fn set_revision_operation(
        &self,
        identity_id: [u8; 32],
        revision: u64,
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
            (i64::MAX - 1) as u64
        };

        drive_operations.push(self.set_identity_balance_operation(
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
        required_removed_balance: u64,
        total_desired_removed_balance: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.remove_from_identity_balance_operations(
            identity_id,
            required_removed_balance,
            total_desired_removed_balance,
            apply,
            transaction,
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
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let previous_balance = self.fetch_identity_balance_operations(
            identity_id,
            apply,
            transaction,
            drive_operations,
        )?;

        let (new_balance, negative_credit_amount) = if apply {
            let previous_balance =
                previous_balance.ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "there should always be a balance if apply is set to true",
                )))?;
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
            }
        } else {
            // As these are just estimations, let's be conservative and say that they are going
            // 10M credits in the red
            (i64::MAX as u64, Some(10000000))
        };

        drive_operations.push(self.set_identity_balance_operation(
            identity_id,
            new_balance,
            true,
        )?);
        if let Some(negative_credit_amount) = negative_credit_amount {
            drive_operations.push(
                self.set_identity_negative_credit_operation(identity_id, negative_credit_amount),
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

    /// Disable identity keys.
    /// Modification of keys is prohibited on protocol level.
    /// This method introduced ONLY to disable keys. All references and query data won't be updated.
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

    /// Modification of keys is prohibited on protocol level.
    /// This method introduced ONLY to disable keys. All references and query data won't be updated.
    pub(crate) fn disable_identity_keys_operations(
        &self,
        identity_id: [u8; 32],
        key_ids: Vec<KeyID>,
        disable_at: TimestampMillis,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // TODO: This probably wrong
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
            );
        }

        let key_ids_len = key_ids.len();

        let key_request = IdentityKeysRequest {
            identity_id,
            request_type: KeyRequestType::SpecificKeys(key_ids),
            limit: Some(key_ids_len as u16),
            offset: None,
        };

        // TODO We need to implement dry run for fetch
        let keys: KeyIDIdentityPublicKeyPairVec =
            self.fetch_identity_keys_operations(key_request, transaction, drive_operations)?;

        if keys.len() != key_ids_len {
            // TODO Choose / add an appropriate error
            return Err(Error::Drive(DriveError::UpdatingDocumentThatDoesNotExist(
                "key to disable with specified ID is not found",
            )));
        }

        for (_, mut key) in keys {
            key.disabled_at = Some(disable_at);

            let key_id_bytes = key.id.encode_var_vec();

            self.insert_key_to_storage_operations(
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
    use crate::drive::block_info::BlockInfo;
    use dpp::identity::{Identity, IdentityPublicKey};

    use tempfile::TempDir;

    use crate::drive::Drive;
    use crate::fee::result::FeeResult;
    use crate::fee_pools::epochs::Epoch;

    #[test]
    fn test_update_identity_balance() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345));

        let old_balance = identity.balance;

        let block = BlockInfo::default_with_epoch(Epoch::new(0));

        drive
            .add_new_identity(identity.clone(), &block, true, None)
            .expect("expected to insert identity");

        let db_transaction = drive.grove.start_transaction();

        drive
            .add_to_identity_balance(
                identity.id.to_buffer(),
                300,
                &block,
                true,
                Some(&db_transaction),
            )
            .expect("expected to add to identity balance");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("expected to be able to commit a transaction");

        let (balance, _fee_cost) = drive
            .fetch_identity_balance_with_fees(identity.id.to_buffer(), &block, true, None)
            .expect("expected to get balance");

        assert_eq!(balance.unwrap(), old_balance + 300);
    }

    #[test]
    fn test_update_identity_balance_estimated_costs() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345));

        let old_balance = identity.balance;

        let block = BlockInfo::default_with_epoch(Epoch::new(0));

        drive
            .add_new_identity(identity.clone(), &block, true, None)
            .expect("expected to insert identity");

        let fee_result = drive
            .add_to_identity_balance(identity.id.to_buffer(), 300, &block, false, None)
            .expect("expected to get estimated costs to update an identity balance");

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 0,
                processing_fee: 5528800,
                fee_refunds: Default::default(),
                removed_bytes_from_system: 0,
            }
        );

        let (balance, _fee_cost) = drive
            .fetch_identity_balance_with_fees(identity.id.to_buffer(), &block, true, None)
            .expect("expected to get balance");

        assert_eq!(balance.unwrap(), old_balance); //shouldn't have changed
    }

    #[test]
    fn test_add_one_new_key_to_identity() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345));

        let block = BlockInfo::default_with_epoch(Epoch::new(0));

        drive
            .add_new_identity(identity.clone(), &block, true, None)
            .expect("expected to insert identity");

        let new_keys_to_add = IdentityPublicKey::random_keys(5, 1, Some(15));

        let db_transaction = drive.grove.start_transaction();

        drive
            .add_new_keys_to_identity(
                identity.id.to_buffer(),
                new_keys_to_add,
                &block,
                true,
                Some(&db_transaction),
            )
            .expect("expected to update identity with new keys");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("expected to be able to commit a transaction");

        let identity_keys = drive
            .fetch_all_identity_keys(identity.id.to_buffer(), None)
            .expect("expected to get balance");

        assert_eq!(identity_keys.len(), 6); // we had 5 keys and we added 30
    }

    #[test]
    fn test_add_two_dozen_new_keys_to_identity() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

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

        drive
            .add_new_keys_to_identity(
                identity.id.to_buffer(),
                new_keys_to_add,
                &block,
                true,
                Some(&db_transaction),
            )
            .expect("expected to update identity with new keys");

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
}

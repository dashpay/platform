use crate::drive::block_info::BlockInfo;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::BatchInsertTreeApplyType;
use crate::drive::object_size_info::PathKeyInfo::PathFixedSizeKey;
use crate::drive::{identity_tree_path, Drive};
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::{calculate_fee};
use dpp::identity::Identity;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use crate::fee::result::FeeResult;

impl Drive {
    /// Deserialize an identity from cbor encoded bytes and potentially apply it to the state
    pub fn add_new_identity_from_cbor_encoded_bytes(
        &self,
        identity_bytes: Vec<u8>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let identity = Identity::from_cbor(identity_bytes.as_slice()).map_err(Error::Protocol)?;

        self.add_new_identity(identity, block_info, apply, transaction)
    }

    /// Adds a identity by inserting a new identity subtree structure to the `Identities` subtree.
    pub fn add_new_identity(
        &self,
        identity: Identity,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.add_new_identity_add_to_operations(
            identity,
            block_info,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }

    /// Adds identity creation operations to drive operations
    pub(crate) fn add_new_identity_add_to_operations(
        &self,
        identity: Identity,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.add_insert_identity_operations(
            identity,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
        )?;

        self.apply_batch_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
        )
    }

    /// The operations needed to create an identity
    pub(crate) fn add_insert_identity_operations(
        &self,
        identity: Identity,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];

        // There is no reason to store the owner id as we always know the owner id of identity information.
        let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, None);

        let identity_tree_path = identity_tree_path();

        let Identity {
            id,
            public_keys,
            revision,
            balance,
            ..
        } = identity;

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsert
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsert {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: storage_flags.serialized_size(),
            }
        };

        // We insert the identity tree
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKey((identity_tree_path, id.to_buffer_vec())),
            Some(&storage_flags),
            apply_type,
            transaction,
            &mut batch_operations,
        )?;

        if !inserted {
            return Err(Error::Identity(IdentityError::IdentityAlreadyExists(
                "trying to insert an identity that already exists",
            )));
        }

        // We insert the balance
        batch_operations.push(self.set_identity_balance_operation(
            id.to_buffer(),
            balance,
            false,
        )?);

        // We insert the revision
        // todo: we might not need the revision
        batch_operations.push(self.set_revision_operation(id.to_buffer(), revision));

        let mut create_tree_keys_operations = self.create_key_tree_with_keys_operations(
            id.to_buffer(),
            public_keys.into_values().collect(),
            &storage_flags,
            estimated_costs_only_with_layer_info,
            transaction,
        )?;
        // We insert the key tree and keys
        batch_operations.append(&mut create_tree_keys_operations);

        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive;
    use crate::drive::block_info::BlockInfo;
    use dpp::identity::Identity;

    use tempfile::TempDir;

    use crate::drive::Drive;

    #[test]
    fn test_insert_and_fetch_identity() {
        let drive = setup_drive(None);

        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction))
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345));

        drive
            .add_new_identity(
                identity.clone(),
                &BlockInfo::default(),
                true,
                Some(&transaction),
            )
            .expect("expected to insert identity");

        let fetched_identity = drive
            .fetch_full_identity(identity.id.buffer, Some(&transaction))
            .expect("should fetch an identity")
            .expect("should have an identity");

        assert_eq!(identity, fetched_identity);
    }

    #[test]
    fn test_insert_identity() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let identity = Identity::random_identity(5, Some(12345));
        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        drive
            .add_new_identity(identity, &BlockInfo::default(), true, Some(&db_transaction))
            .expect("expected to insert identity");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("expected to be able to commit a transaction");
    }
}

use crate::drive::flags::StorageFlags;
use crate::drive::identity::{
    identity_key_location_vec, identity_key_tree_path, identity_path, identity_path_vec,
    identity_query_keys_full_tree_path, identity_query_keys_purpose_tree_path,
    identity_query_keys_tree_path, IdentityRootStructure, IDENTITY_KEY,
};
use std::collections::HashMap;

use crate::contract::types::encode_u16;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::block_info::BlockInfo;
use crate::drive::grove_operations::QueryTarget::QueryTargetTree;
use crate::drive::grove_operations::{BatchInsertTreeApplyType, DirectQueryType, QueryType};
use crate::drive::object_size_info::ElementInfo::Element;
use crate::drive::object_size_info::PathKeyElementInfo::PathFixedSizeKeyElement;
use crate::drive::{identity_tree_path, key_hashes_tree_path, Drive, RootTree};
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation::FunctionOperation;
use crate::fee::op::{DriveOperation, FunctionOp, HashFunction};
use crate::fee::{calculate_fee, FeeResult};
use crate::identity::key::IdentityKey;
use crate::identity::Identity;
use dpp::identity::Identity;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::SiblingReference;
use grovedb::Element::{Item, Reference};
use grovedb::{ElementFlags, EstimatedLayerInformation, TransactionArg};
use sha2::{Digest, Sha256};

impl Drive {
    /// Insert new Identity
    pub fn insert_new_identity(
        &self,
        identity: Identity,
        storage_flags: &StorageFlags,
        verify: bool,
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
        self.add_insert_identity_operations(
            identity,
            storage_flags,
            verify,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
        )?;

        let mut drive_operations: Vec<DriveOperation> = vec![];

        self.apply_batch(apply, transaction, batch_operations, &mut drive_operations)?;

        calculate_fee(None, Some(drive_operations), &block_info.epoch)
    }

    /// The operations needed to create an identity
    pub(crate) fn add_insert_identity_operations(
        &self,
        identity: Identity,
        storage_flags: &StorageFlags,
        verify: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let identity_tree_path = identity_tree_path();

        let Identity {
            protocol_version,
            id,
            public_keys,
            revision,
            balance,
            asset_lock_proof,
            metadata,
        } = identity;

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetTree(storage_flags.serialized_size(), false),
            }
        };
        // If we are asking to verify we check to make sure the tree for this identity doesn't yet exist
        if verify {
            let exists = self.grove_has_raw(
                identity_tree_path,
                id.as_slice(),
                apply_type,
                transaction,
                drive_operations,
            )?;
            if exists {
                return Err(Error::Identity(IdentityError::IdentityAlreadyExists(
                    "trying to insert an identity that already exists",
                )));
            }
        }

        // We insert the identity tree
        self.batch_insert(
            PathFixedSizeKeyElement((
                identity_tree_path,
                id.as_slice(),
                Element::empty_tree_with_flags(Some(storage_flags.to_element_flags())),
            )),
            drive_operations,
        )?;

        // We insert the balance
        drive_operations.push(self.set_identity_balance_operation(
            id,
            balance,
            Some(storage_flags.to_element_flags()),
        ));

        // We insert the revision
        drive_operations.push(self.set_revision_operation(
            id,
            revision,
            storage_flags.to_element_flags(),
        ));

        // We insert the key tree and keys
        self.create_key_tree_with_keys_operations(
            id,
            public_keys.into_values().collect(),
            storage_flags.to_element_flags(),
            apply,
            transaction,
            drive_operations,
        )
    }

    // /// Adds operations to the op batch to insert a new identity in the `Identities` subtree
    // /// with its own empty subtree.
    // pub fn add_insert_identity_operations(
    //     &self,
    //     identity: Identity,
    //     storage_flags: Option<&StorageFlags>,
    //     batch: &mut GroveDbOpBatch,
    // ) -> Result<(), Error> {
    //     // Serialize identity
    //     let identity_bytes = identity.to_buffer().map_err(|_| {
    //         Error::Identity(IdentityError::IdentitySerialization(
    //             "failed to serialize identity to CBOR",
    //         ))
    //     })?;
    //
    //     // Adds an operation to the op batch which inserts an empty subtree with flags
    //     // at the key of the given identity in the `Identities` subtree.
    //     batch.add_insert_empty_tree_with_flags(
    //         vec![vec![RootTree::Identities as u8]],
    //         identity.id.buffer.to_vec(),
    //         storage_flags,
    //     );
    //
    //     // Adds an operation to the op batch which inserts the serialized identity
    //     // in the `IDENTITY_KEY` key of the new subtree that was just created.
    //     batch.add_insert(
    //         vec![
    //             vec![RootTree::Identities as u8],
    //             identity.id.buffer.to_vec(),
    //         ],
    //         IDENTITY_KEY.to_vec(),
    //         Element::Item(
    //             identity_bytes,
    //             StorageFlags::map_to_some_element_flags(storage_flags),
    //         ),
    //     );
    //
    //     Ok(())
    // }

    // pub fn insert_identity(
    //     &self,
    //     identity_key: &[u8],
    //     identity_bytes: Element,
    //     apply: bool,
    //     transaction: TransactionArg,
    // ) -> Result<(i64, u64), Error> {
    //     let mut batch_operations: Vec<DriveOperation> = vec![];
    //
    //     self.batch_insert(
    //         PathFixedSizeKeyElement((
    //             [Into::<&[u8; 1]>::into(RootTree::Identities).as_slice()],
    //             identity_key,
    //             identity_bytes,
    //         )),
    //         &mut batch_operations,
    //     )?;
    //
    //     let mut drive_operations: Vec<DriveOperation> = vec![];
    //
    //     self.apply_batch(apply, transaction, batch_operations, &mut drive_operations)?;
    //
    //     calculate_fee(None, Some(drive_operations))
    // }

    /// Inserts a new identity to the `Identities` subtree.
    pub fn insert_identity(
        &self,
        identity: Identity,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        storage_flags: &StorageFlags,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut batch = GroveDbOpBatch::new();

        self.add_insert_identity_operations(identity, storage_flags, &mut batch)?;

        let mut drive_operations: Vec<DriveOperation> = vec![];

        self.apply_batch_grovedb_operations(apply, transaction, batch, &mut drive_operations)?;

        calculate_fee(None, Some(drive_operations), &block_info.epoch)
    }

    pub fn insert_identity_cbor(
        &self,
        identity_id: Option<&[u8]>,
        identity_bytes: Vec<u8>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let identity_id = match identity_id {
            None => {
                let identity = Identity::from_cbor(identity_bytes.as_slice())?;
                Vec::from(identity.id)
            }
            Some(identity_id) => Vec::from(identity_id),
        };

        let epoch = self.epoch_info.borrow().current_epoch;

        let storage_flags = StorageFlags { epoch };

        self.insert_identity(
            identity_id.as_slice(),
            block_info,
            Element::new_item_with_flags(identity_bytes, storage_flags.to_element_flags()),
            apply,
            transaction,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive;
    use crate::drive::block_info::BlockInfo;
    use dpp::identity::Identity;
    use grovedb::Element;
    use tempfile::TempDir;

    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;
    use crate::identity::Identity;

    #[test]
    fn test_insert_and_fetch_identity() {
        let drive = setup_drive(None);

        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction))
            .expect("expected to create root tree successfully");

        let identity_bytes = hex::decode("01000000a462696458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6762616c616e636500687265766973696f6e006a7075626c69634b65797381a6626964006464617461582102abb64674c5df796559eb3cf92a84525cc1a6068e7ad9d4ff48a1f0b179ae29e164747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00").expect("expected to decode identity hex");

        let identity = Identity::from_buffer(identity_bytes.as_slice())
            .expect("expected to deserialize an identity");

        drive
            .insert_identity(
                identity.clone(),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&transaction),
            )
            .expect("expected to insert identity");

        let (fetched_identity, _) = drive
            .fetch_identity(&identity.id.buffer, Some(&transaction))
            .expect("should fetch an identity");

        assert_eq!(
            fetched_identity.to_buffer().expect("should serialize"),
            identity.to_buffer().expect("should serialize")
        );
    }

    #[test]
    fn test_insert_identity() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let identity = Identity::random_identity(5, Some(12345));
        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        drive
            .insert_new_identity(
                identity,
                &StorageFlags::SingleEpoch(0),
                false,
                &BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert identity");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("expected to be able to commit a transaction");
    }

    #[test]
    fn test_insert_identity_old() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let identity_bytes = hex::decode("01000000a462696458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6762616c616e636500687265766973696f6e006a7075626c69634b65797381a6626964006464617461582102abb64674c5df796559eb3cf92a84525cc1a6068e7ad9d4ff48a1f0b179ae29e164747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00").expect("expected to decode identity hex");

        let identity = Identity::from_cbor(identity_bytes.as_slice())
            .expect("expected to deserialize an identity");

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .insert_identity(
                &identity.id,
                Element::Item(identity_bytes, storage_flags.to_element_flags()),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert identity");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("expected to be able to commit a transaction");
    }
}

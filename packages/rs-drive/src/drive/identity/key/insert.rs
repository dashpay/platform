use crate::drive::defaults::DEFAULT_HASH_SIZE_U8;
use crate::drive::flags::{StorageFlags, SINGLE_EPOCH_FLAGS_SIZE};
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::drive::identity::IdentityRootStructure::{IdentityTreeKeyReferences, IdentityTreeKeys};
use crate::drive::identity::{
    identity_key_location_within_identity_vec, identity_key_path_vec, identity_key_tree_path,
    identity_path, identity_query_keys_full_tree_path, identity_query_keys_purpose_tree_path,
    identity_query_keys_tree_path,
};
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElement, PathKeyElementSize,
};
use crate::drive::object_size_info::PathKeyInfo::PathFixedSizeKey;
use crate::drive::object_size_info::{DriveKeyInfo, PathKeyElementInfo};
use crate::drive::{key_hashes_tree_path_vec, Drive};
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation::FunctionOperation;
use crate::fee::op::{DriveOperation, FunctionOp, HashFunction};
use dpp::identity::{IdentityPublicKey, Purpose, SecurityLevel};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::reference_path::ReferencePathType::AbsolutePathReference;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use serde::Serialize;
use std::collections::HashMap;

impl Drive {
    fn insert_reference_to_key_operations(
        &self,
        identity_id: &[u8],
        identity_key: &IdentityPublicKey,
        storage_flags: &StorageFlags,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let identity_path = identity_key_path_vec(identity_id, identity_key.id);

        let reference = Element::new_reference_with_max_hops_and_flags(
            AbsolutePathReference(identity_path),
            Some(1),
            storage_flags.to_some_element_flags(),
        );

        let key_hashes_tree = key_hashes_tree_path_vec();

        let (apply_type, path_key_element_info) = if estimated_costs_only_with_layer_info.is_none()
        {
            let key_hash = identity_key.hash()?;
            let path_fixed_sized_key_element: PathKeyElementInfo<'_, 0> =
                PathKeyElement((key_hashes_tree, key_hash, reference));
            (
                BatchInsertApplyType::StatefulBatchInsert,
                path_fixed_sized_key_element,
            )
        } else {
            let ref_size = reference.serialized_size() as u32;
            let path_fixed_sized_key_element = PathKeyElementSize((
                KeyInfoPath::from_known_owned_path(key_hashes_tree),
                KeyInfo::MaxKeySize {
                    unique_id: b"key_hash".to_vec(),
                    max_size: DEFAULT_HASH_SIZE_U8,
                },
                reference,
            ));

            // We use key_hash just not to use an empty string, but it doesn't matter what it is
            // as long as it is unique
            (
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_using_sums: false,
                    target: QueryTargetValue(ref_size),
                },
                path_fixed_sized_key_element,
            )
        };

        let key_len = identity_key.data.len();
        drive_operations.push(FunctionOperation(FunctionOp::new_with_byte_count(
            HashFunction::Sha256,
            key_len as u16,
        )));

        // Let's first insert the hash with a reference to the identity
        let inserted = self.batch_insert_if_not_exists(
            path_key_element_info,
            apply_type,
            transaction,
            drive_operations,
        )?;

        if inserted {
            Ok(())
        } else {
            Err(Error::Identity(IdentityError::IdentityAlreadyExists(
                "trying to insert a key that already exists",
            )))
        }
    }

    fn insert_key_to_storage_operations(
        &self,
        identity_id: &[u8],
        identity_key: &IdentityPublicKey,
        key_id_bytes: &[u8],
        storage_flags: &StorageFlags,
        _estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let serialized_identity_key = identity_key.serialize().map_err(Error::Protocol)?;
        // Now lets insert the public key
        let identity_key_tree = identity_key_tree_path(identity_id);

        self.batch_insert(
            PathFixedSizeKeyRefElement((
                identity_key_tree,
                key_id_bytes,
                Element::new_item_with_flags(
                    serialized_identity_key,
                    storage_flags.to_some_element_flags(),
                ),
            )),
            drive_operations,
        )
    }

    fn insert_key_searchable_references_operations(
        &self,
        identity_id: &[u8],
        identity_key: &IdentityPublicKey,
        key_id_bytes: &[u8],
        storage_flags: &StorageFlags,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let purpose = identity_key.purpose;
        let security_level = identity_key.security_level;
        let purpose_vec = vec![purpose as u8];
        let security_level_vec = vec![security_level as u8];

        // Now lets add in references so we can query keys.
        // We assume the following, the identity already has a the basic Query Tree

        if purpose != Purpose::AUTHENTICATION {
            // Not authentication
            if security_level != SecurityLevel::MEDIUM {
                // Not Medium (Medium is already pre-inserted)

                let purpose_path =
                    identity_query_keys_purpose_tree_path(identity_id, purpose_vec.as_slice());

                let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                    BatchInsertTreeApplyType::StatefulBatchInsert
                } else {
                    BatchInsertTreeApplyType::StatelessBatchInsert {
                        in_tree_using_sums: false,
                        is_sum_tree: false,
                        flags_len: SINGLE_EPOCH_FLAGS_SIZE,
                    }
                };

                // We need to insert the security level if it doesn't yet exist
                self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
                    PathFixedSizeKey((purpose_path, vec![security_level as u8])),
                    Some(storage_flags),
                    apply_type,
                    transaction,
                    drive_operations,
                )?;
            }
        }

        // Now let's set the reference
        let reference_path = identity_query_keys_full_tree_path(
            identity_id,
            purpose_vec.as_slice(),
            security_level_vec.as_slice(),
        );

        let key_reference = identity_key_location_within_identity_vec(key_id_bytes);
        self.batch_insert(
            PathFixedSizeKeyRefElement((
                reference_path,
                key_id_bytes,
                Element::new_reference_with_flags(
                    ReferencePathType::UpstreamRootHeightReference(2, key_reference),
                    storage_flags.to_some_element_flags(),
                ),
            )),
            drive_operations,
        )
    }

    /// Insert a new key into an identity operations
    pub(crate) fn insert_new_key_operations(
        &self,
        identity_id: &[u8],
        identity_key: IdentityPublicKey,
        storage_flags: &StorageFlags,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        self.insert_reference_to_key_operations(
            identity_id,
            &identity_key,
            storage_flags,
            estimated_costs_only_with_layer_info,
            transaction,
            drive_operations,
        )?;

        let key_id_bytes = identity_key.id.encode_var_vec();

        self.insert_key_to_storage_operations(
            identity_id,
            &identity_key,
            key_id_bytes.as_slice(),
            storage_flags,
            estimated_costs_only_with_layer_info,
            drive_operations,
        )?;

        self.insert_key_searchable_references_operations(
            identity_id,
            &identity_key,
            key_id_bytes.as_slice(),
            storage_flags,
            estimated_costs_only_with_layer_info,
            transaction,
            drive_operations,
        )
    }

    pub(crate) fn create_key_tree_with_keys_operations(
        &self,
        identity_id: [u8; 32],
        keys: Vec<IdentityPublicKey>,
        storage_flags: &StorageFlags,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];
        let identity_path = identity_path(identity_id.as_slice());
        self.batch_insert_empty_tree(
            identity_path,
            IdentityTreeKeys.to_drive_key_info(),
            Some(storage_flags),
            &mut batch_operations,
        )?;

        self.batch_insert_empty_tree(
            identity_path,
            IdentityTreeKeyReferences.to_drive_key_info(),
            Some(storage_flags),
            &mut batch_operations,
        )?;

        // We create the query trees structure
        self.create_new_identity_key_query_trees_operations(
            identity_id,
            storage_flags,
            &mut batch_operations,
        )?;

        for key in keys.into_iter() {
            self.insert_new_key_operations(
                identity_id.as_slice(),
                key,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                &mut batch_operations,
            )?;
        }
        Ok(batch_operations)
    }

    /// This creates the key query tree structure operations and adds them to the
    /// mutable drive_operations vector
    fn create_new_identity_key_query_trees_operations(
        &self,
        identity_id: [u8; 32],
        storage_flags: &StorageFlags,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let identity_query_key_tree = identity_query_keys_tree_path(identity_id.as_slice());

        // There are 4 Purposes: Authentication, Encryption, Decryption, Withdrawal
        for purpose in 0..4 {
            self.batch_insert_empty_tree(
                identity_query_key_tree,
                DriveKeyInfo::Key(vec![purpose]),
                Some(storage_flags),
                drive_operations,
            )?;
        }
        // There are 4 Security Levels: Master, Critical, High, Medium
        // For the Authentication Purpose we insert every tree
        let identity_key_authentication_tree =
            identity_query_keys_purpose_tree_path(identity_id.as_slice(), &[0]);
        for security_level in 0..4 {
            self.batch_insert_empty_tree(
                identity_key_authentication_tree,
                DriveKeyInfo::Key(vec![security_level]),
                Some(storage_flags),
                drive_operations,
            )?;
        }
        // For Encryption and Decryption we only insert the medium security level
        for purpose in 1..3 {
            let purpose_vec = vec![purpose];
            let identity_key_purpose_tree = identity_query_keys_purpose_tree_path(
                identity_id.as_slice(),
                purpose_vec.as_slice(),
            );

            self.batch_insert_empty_tree(
                identity_key_purpose_tree,
                DriveKeyInfo::Key(vec![SecurityLevel::MEDIUM as u8]),
                Some(storage_flags),
                drive_operations,
            )?;
        }
        Ok(())
    }
}

use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, MAX_ELEMENT_SIZE};
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::QueryTarget::{QueryTargetTree, QueryTargetValue};
use crate::drive::grove_operations::{BatchInsertApplyType, DirectQueryType};
use crate::drive::identity::IdentityRootStructure::IdentityTreeKeys;
use crate::drive::identity::{
    identity_key_location_within_identity_vec, identity_key_tree_path, identity_path,
    identity_path_vec, identity_query_keys_full_tree_path, identity_query_keys_purpose_tree_path,
    identity_query_keys_tree_path,
};
use crate::drive::object_size_info::DriveKeyInfo;
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyElement, PathKeyElementSize,
};
use crate::drive::object_size_info::PathKeyInfo::{PathFixedSizeKey, PathFixedSizeKeyRef};
use crate::drive::{key_hashes_tree_path, Drive};
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation::FunctionOperation;
use crate::fee::op::{DriveOperation, FunctionOp, HashFunction};
use dpp::identity::IdentityPublicKey;
use dpp::identity::SecurityLevel::MEDIUM;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::reference_path::ReferencePathType::AbsolutePathReference;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use serde::Serialize;
use std::collections::HashMap;

impl Drive {
    /// Insert a new key into an identity operations
    pub fn insert_new_key_operations(
        &self,
        identity_id: &[u8],
        identity_key: IdentityPublicKey,
        storage_flags: &StorageFlags,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let serialized_identity_key = identity_key.serialize().map_err(|e| {
            Error::Identity(IdentityError::IdentitySerialization(e.to_string().as_str()))
        })?;
        let IdentityPublicKey {
            id,
            purpose,
            security_level,
            key_type,
            data,
            read_only,
            disabled_at,
            signature,
        } = identity_key;
        let key_hashes_tree = key_hashes_tree_path();
        let key_len = data.len();
        drive_operations.push(FunctionOperation(FunctionOp {
            hash: HashFunction::Sha256,
            byte_count: key_len as u16,
        }));

        let identity_path = identity_path_vec(identity_id);

        let reference = Element::new_reference_with_max_hops_and_flags(
            AbsolutePathReference(identity_path),
            Some(1),
            storage_flags.to_some_element_flags(),
        );

        let (apply_type, path_key_element_info) = if apply {
            let key_hash = identity_key.hash()?;
            let path_fixed_sized_key_element =
                PathFixedSizeKeyElement((key_hashes_tree, key_hash.as_slice(), reference));
            (
                BatchInsertApplyType::StatefulBatchInsert,
                path_fixed_sized_key_element,
            )
        } else {
            let ref_size = reference.serialized_size() as u32;
            let path_fixed_sized_key_element = PathKeyElementSize((
                KeyInfoPath::from_known_path(key_hashes_tree),
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

        // Let's first insert the hash with a reference to the identity
        let already_existed = self.batch_insert_if_not_exists(
            path_key_element_info,
            apply_type,
            transaction,
            drive_operations,
        )?;

        if already_existed {
            return Err(Error::Identity(IdentityError::IdentityAlreadyExists(
                "trying to insert a key that already exists",
            )));
        }

        // Now lets insert the public key
        let identity_key_tree = identity_key_tree_path(identity_id);

        let key_id_bytes = encode_u16(id)?;
        self.batch_insert(
            PathFixedSizeKeyElement((
                identity_key_tree,
                key_id_bytes.as_slice(),
                Element::new_item_with_flags(
                    serialized_identity_key,
                    storage_flags.to_some_element_flags(),
                ),
            )),
            drive_operations,
        )?;

        let purpose_vec = vec![purpose];
        let security_level_vec = vec![security_level];

        // Now lets add in references so we can query keys.
        // We assume the following, the identity already has a the basic Query Tree

        if purpose != 0 {
            // Not authentication
            if security_level != 3 {
                // Not Medium (Medium is already pre-inserted)

                let purpose_path =
                    identity_query_keys_purpose_tree_path(identity_id, purpose_vec.as_slice());

                //todo: is this really a tree?
                let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                    DirectQueryType::StatefulDirectQuery
                } else {
                    DirectQueryType::StatelessDirectQuery {
                        in_tree_using_sums: false,
                        query_target: QueryTargetTree(storage_flags.serialized_size(), false),
                    }
                };

                // We need to insert the security level if it doesn't yet exist
                self.batch_insert_empty_tree_if_not_exists(
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

        let key_reference = identity_key_location_within_identity_vec(key_id_bytes.as_slice());
        self.batch_insert(
            PathFixedSizeKeyElement((
                reference_path,
                key_id_bytes.as_slice(),
                Element::new_reference_with_flags(
                    ReferencePathType::UpstreamRootHeightReference(2, key_reference),
                    Some(element_flags),
                ),
            )),
            drive_operations,
        )
    }

    pub(super) fn create_key_tree_with_keys_operations(
        &self,
        identity_id: [u8; 32],
        keys: Vec<IdentityKey>,
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

        // We create the query trees structure
        self.create_new_identity_key_query_trees_operations(
            identity_id,
            element_flags.clone(),
            &mut batch_operations,
        )?;

        for key in keys.into_iter() {
            self.insert_new_key_operations(
                identity_id.as_slice(),
                key,
                element_flags.clone(),
                false,
                apply,
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
        let identity_key_tree = identity_key_tree_path(identity_id.as_slice());

        // We need to insert the query tree
        self.batch_insert_empty_tree(
            identity_key_tree,
            DriveKeyInfo::Key(vec![]),
            Some(storage_flags),
            drive_operations,
        )?;

        let identity_query_key_tree = identity_query_keys_tree_path(identity_id.as_slice());

        // There are 3 Purposes: Authentication, Encryption, Decryption
        for purpose in 0..3 {
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
                DriveKeyInfo::Key(vec![MEDIUM as u8]),
                Some(storage_flags),
                drive_operations,
            )?;

            self.batch_insert(
                PathFixedSizeKeyElement((
                    identity_key_purpose_tree,
                    &[3], //medium
                    Element::empty_tree_with_flags(Some(element_flags.clone())),
                )),
                drive_operations,
            )?;
        }
        Ok(())
    }
}

use crate::drive::defaults::{DEFAULT_HASH_160_SIZE_U8, DEFAULT_HASH_SIZE_U8};
use crate::drive::flags::{StorageFlags, SINGLE_EPOCH_FLAGS_SIZE};
use crate::drive::grove_operations::BatchInsertApplyType::StatefulBatchInsert;
use crate::drive::grove_operations::BatchInsertTreeApplyType::StatefulBatchInsertTree;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::drive::identity::IdentityRootStructure::{
    IdentityContractInfo, IdentityTreeKeyReferences, IdentityTreeKeys,
};
use crate::drive::identity::{
    identity_contract_info_root_path_vec, identity_key_location_within_identity_vec,
    identity_key_path_vec, identity_key_tree_path, identity_key_tree_path_vec, identity_path,
    identity_path_vec, identity_query_keys_full_tree_path, identity_query_keys_purpose_tree_path,
    identity_query_keys_tree_path,
};
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElement, PathKeyElementSize,
};
use crate::drive::object_size_info::PathKeyInfo::PathFixedSizeKey;
use crate::drive::object_size_info::{DriveKeyInfo, PathKeyElementInfo, PathKeyInfo};
use crate::drive::{unique_key_hashes_tree_path_vec, Drive};
use crate::error::drive::DriveError;
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

pub enum ContractApplyInfo {
    Keys(Vec<IdentityPublicKey>),
}

impl Drive {
    pub(crate) fn insert_key_to_storage_operations(
        &self,
        identity_id: [u8; 32],
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
        let identity_key_tree = identity_key_tree_path(identity_id.as_slice());

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

    /// Modification of keys is prohibited on protocol level.
    /// This method introduced ONLY to disable keys.
    pub(crate) fn replace_key_in_storage_operations(
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
        let identity_key_tree = identity_key_tree_path_vec(identity_id);

        drive_operations.push(DriveOperation::replace_for_known_path_key_element(
            identity_key_tree,
            key_id_bytes.to_vec(),
            Element::new_item_with_flags(
                serialized_identity_key,
                storage_flags.to_some_element_flags(),
            ),
        ));

        Ok(())
    }

    fn insert_key_searchable_references_operations(
        &self,
        identity_id: [u8; 32],
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

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_root_key_reference_tree(
                identity_id,
                estimated_costs_only_with_layer_info,
            );

            Self::add_estimation_costs_for_purpose_in_key_reference_tree(
                identity_id,
                estimated_costs_only_with_layer_info,
                purpose,
            );

            if matches!(purpose, Purpose::AUTHENTICATION) {
                Self::add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                    security_level,
                );
            }
        }

        // Now lets add in references so we can query keys.
        // We assume the following, the identity already has a the basic Query Tree

        if !(purpose == Purpose::AUTHENTICATION && security_level == SecurityLevel::MEDIUM) {
            // Not Medium (Medium is already pre-inserted)

            let purpose_path = identity_query_keys_purpose_tree_path(
                identity_id.as_slice(),
                purpose_vec.as_slice(),
            );

            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
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

        // Now let's set the reference
        let reference_path = identity_query_keys_full_tree_path(
            identity_id.as_slice(),
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
    pub(crate) fn insert_new_unique_key_operations(
        &self,
        identity_id: [u8; 32],
        identity_key: IdentityPublicKey,
        with_references: bool,
        storage_flags: &StorageFlags,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        drive_operations.append(&mut self.insert_reference_to_key_operations(
            identity_id,
            &identity_key,
            estimated_costs_only_with_layer_info,
            transaction,
        )?);

        let key_id_bytes = identity_key.id.encode_var_vec();

        self.insert_key_to_storage_operations(
            identity_id,
            &identity_key,
            key_id_bytes.as_slice(),
            storage_flags,
            estimated_costs_only_with_layer_info,
            drive_operations,
        )?;

        if with_references {
            if matches!(
                identity_key.purpose,
                Purpose::AUTHENTICATION | Purpose::WITHDRAW
            ) {
                self.insert_key_searchable_references_operations(
                    identity_id,
                    &identity_key,
                    key_id_bytes.as_slice(),
                    storage_flags,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    drive_operations,
                )?;
            }
        }
        Ok(())
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
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
            );
        }
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
            estimated_costs_only_with_layer_info,
            &mut batch_operations,
        )?;

        for key in keys.into_iter() {
            self.insert_new_unique_key_operations(
                identity_id,
                key,
                true,
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
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let identity_query_key_tree = identity_query_keys_tree_path(identity_id.as_slice());

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_root_key_reference_tree(
                identity_id,
                estimated_costs_only_with_layer_info,
            )
        }

        // There are 4 Purposes: Authentication, Encryption, Decryption, Withdrawal
        for purpose in Purpose::authentication_withdraw() {
            self.batch_insert_empty_tree(
                identity_query_key_tree,
                DriveKeyInfo::Key(vec![purpose as u8]),
                Some(storage_flags),
                drive_operations,
            )?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Drive::add_estimation_costs_for_purpose_in_key_reference_tree(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                    purpose,
                )
            }
        }
        // There are 4 Security Levels: Master, Critical, High, Medium
        // For the Authentication Purpose we insert every tree
        let identity_key_authentication_tree = identity_query_keys_purpose_tree_path(
            identity_id.as_slice(),
            &[Purpose::AUTHENTICATION as u8],
        );
        for security_level in SecurityLevel::full_range() {
            self.batch_insert_empty_tree(
                identity_key_authentication_tree,
                DriveKeyInfo::Key(vec![security_level as u8]),
                Some(storage_flags),
                drive_operations,
            )?;
            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Drive::add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                    security_level,
                )
            }
        }
        Ok(())
    }
}

use crate::drive::defaults::{
    DEFAULT_HASH_160_SIZE_U8, DEFAULT_HASH_SIZE_U16, DEFAULT_HASH_SIZE_U32, DEFAULT_HASH_SIZE_U8,
    ESTIMATED_NON_UNIQUE_KEY_DUPLICATES,
};
use crate::drive::identity::identity_key_path_vec;
use crate::drive::object_size_info::DriveKeyInfo::KeyRef;
use crate::drive::object_size_info::PathKeyElementInfo::PathKeyRefElement;
use crate::drive::object_size_info::PathKeyInfo::PathKeyRef;
use crate::drive::{
    non_unique_key_hashes_sub_tree_path_vec, non_unique_key_hashes_tree_path,
    non_unique_key_hashes_tree_path_vec, unique_key_hashes_tree_path_vec, Drive,
};
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation::FunctionOperation;
use crate::fee::op::{DriveOperation, FunctionOp, HashFunction};
use dpp::identity::IdentityPublicKey;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Insert a public key hash reference that contains an identity id
    /// Contrary to the name this is not a reference but an Item containing the identity
    /// identifier
    pub(crate) fn insert_reference_to_key_operations(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];
        let hash_vec = identity_key.hash()?;
        let key_hash = hash_vec.as_slice().try_into().map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution("key hash not 20 bytes"))
        })?;

        let key_len = identity_key.data.len();
        drive_operations.push(FunctionOperation(FunctionOp::new_with_byte_count(
            HashFunction::Sha256RipeMD160,
            key_len as u16,
        )));

        //todo: check if key is unique

        self.insert_unique_public_key_hash_reference_to_identity_operations(
            identity_id,
            key_hash,
            estimated_costs_only_with_layer_info,
            transaction,
        )
    }

    /// Adds the estimation costs for the insertion of a unique public key hash reference
    fn add_estimation_costs_for_insert_unique_public_key_hash_reference(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        let unique_key_hashes_path = unique_key_hashes_tree_path_vec();

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(unique_key_hashes_path),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_160_SIZE_U8,
                    DEFAULT_HASH_SIZE_U32,
                    None,
                ),
            },
        );
    }

    /// Adds the estimation costs for the insertion of a non unique
    /// public key hash reference
    fn add_estimation_costs_for_insert_non_unique_public_key_hash_reference(
        public_key_hash: [u8; 20],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        let non_unique_key_hashes_path = non_unique_key_hashes_tree_path_vec();

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(non_unique_key_hashes_path),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_160_SIZE_U8, NoSumTrees, None),
            },
        );

        let non_unique_key_hashes_sub_path =
            non_unique_key_hashes_sub_tree_path_vec(public_key_hash);

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(non_unique_key_hashes_sub_path),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(ESTIMATED_NON_UNIQUE_KEY_DUPLICATES),
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, 0, None),
            },
        );
    }

    /// Insert a unique public key hash reference that contains an identity id
    /// Contrary to the name this is not a reference but an Item containing the identity
    /// identifier
    fn insert_unique_public_key_hash_reference_to_identity_operations(
        &self,
        identity_id: [u8; 32],
        public_key_hash: [u8; 20],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];

        let already_exists = self.has_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            &mut drive_operations,
        )?;

        if already_exists {
            return Err(Error::Identity(IdentityError::UniqueKeyAlreadyExists(
                "the key already exists in the unique set",
            )));
        }

        let already_exists = self.has_non_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            &mut drive_operations,
        )?;

        if already_exists {
            return Err(Error::Identity(IdentityError::UniqueKeyAlreadyExists(
                "the key already exists in the non unique set",
            )));
        }

        let unique_key_hashes_path = unique_key_hashes_tree_path_vec();

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_insert_unique_public_key_hash_reference(
                estimated_costs_only_with_layer_info,
            );
        }

        // We insert the identity tree
        self.batch_insert::<0>(
            PathKeyRefElement((
                unique_key_hashes_path,
                public_key_hash.as_slice(),
                Element::Item(identity_id.to_vec(), None),
            )),
            &mut drive_operations,
        )?;

        Ok(drive_operations)
    }

    /// Insert a non unique public key hash reference that contains an identity id
    /// Contrary to the name this is not a reference but an Item containing the identity
    /// identifier
    fn insert_non_unique_public_key_hash_reference_to_identity_operations(
        &self,
        identity_id: [u8; 32],
        public_key_hash: [u8; 20],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];

        let mut key_already_present_in_another_identity = false;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_insert_non_unique_public_key_hash_reference(
                public_key_hash,
                estimated_costs_only_with_layer_info,
            );
        } else {
            let already_exists_in_unique = self.has_unique_public_key_hash_operations(
                public_key_hash,
                transaction,
                &mut drive_operations,
            )?;

            if already_exists_in_unique {
                return Err(Error::Identity(IdentityError::UniqueKeyAlreadyExists(
                    "the key already exists in the unique set",
                )));
            }

            key_already_present_in_another_identity = self
                .has_non_unique_public_key_hash_operations(
                    public_key_hash,
                    transaction,
                    &mut drive_operations,
                )?;

            if key_already_present_in_another_identity {
                let already_exists_for_identity = self
                    .has_non_unique_public_key_hash_already_for_identity_operations(
                        public_key_hash,
                        identity_id,
                        transaction,
                        &mut drive_operations,
                    )?;

                if already_exists_for_identity {
                    return Err(Error::Identity(IdentityError::IdentityKeyAlreadyExists(
                        "the key already exists for this user",
                    )));
                }
            }
        }

        if !key_already_present_in_another_identity {
            let non_unique_key_hashes_path = non_unique_key_hashes_tree_path();
            // We insert the parent tree
            self.batch_insert_empty_tree(
                non_unique_key_hashes_path,
                KeyRef(public_key_hash.as_slice()),
                None,
                &mut drive_operations,
            )?;
        }

        let non_unique_key_hashes_path = non_unique_key_hashes_sub_tree_path_vec(public_key_hash);

        // The value is empty here because the key already has the identity id
        self.batch_insert::<0>(
            PathKeyRefElement((
                non_unique_key_hashes_path,
                identity_id.as_slice(),
                Element::Item(vec![], None),
            )),
            &mut drive_operations,
        )?;

        Ok(drive_operations)
    }
}

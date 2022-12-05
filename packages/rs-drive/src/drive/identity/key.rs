use crate::drive::defaults::MAX_ELEMENT_SIZE;
use crate::drive::identity::{
    identity_key_location_vec, identity_key_location_within_identity_vec, identity_key_tree_path,
    identity_path, identity_path_vec, identity_query_keys_full_tree_path,
    identity_query_keys_purpose_tree_path, identity_query_keys_tree_path, IdentityRootStructure,
};
use crate::drive::object_size_info::ElementInfo::Element;
use crate::drive::object_size_info::PathKeyElementInfo::PathFixedSizeKeyElement;
use crate::drive::{key_hashes_tree_path, Drive};
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::identity::key::IdentityKey;
use grovedb::reference_path::ReferencePathType;
use grovedb::reference_path::ReferencePathType::AbsolutePathReference;
use grovedb::Element::Reference;

impl Drive {
    /// Insert a new key into an identity operations
    pub fn insert_new_key_operations(
        &self,
        identity_id: &[u8],
        identity_key: IdentityKey,
        element_flags: ElementFlags,
        verify: bool,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let serialized_identity_key = identity_key.serialize();
        let IdentityKey {
            id,
            key_type,
            purpose,
            security_level,
            readonly,
            public_key_bytes,
        } = identity_key;
        let key_hashes_tree = key_hashes_tree_path();
        let key_len = public_key_bytes.len();
        let key_hash = Sha256::digest(public_key_bytes.clone());
        drive_operations.push(FunctionOperation(FunctionOp {
            hash: HashFunction::Sha256,
            byte_count: key_len as u16,
        }));
        if verify {
            let exists = self.grove_has_raw(
                key_hashes_tree,
                key_hash.as_slice(),
                if apply { None } else { Some(MAX_ELEMENT_SIZE) }, //if you want to verify you need to know the state
                transaction,
                drive_operations,
            )?;
            if exists {
                return Err(Error::Identity(IdentityError::IdentityAlreadyExists(
                    "trying to insert a key that already exists",
                )));
            }
        }

        let identity_path = identity_path_vec(identity_id);

        // Let's first insert the hash with a reference to the identity
        self.batch_insert(
            PathFixedSizeKeyElement((
                key_hashes_tree,
                key_hash.as_slice(),
                Element::new_reference_with_max_hops_and_flags(
                    AbsolutePathReference(identity_path),
                    Some(1),
                    Some(element_flags.clone()),
                ),
            )),
            drive_operations,
        )?;

        // Now lets insert the public key
        let identity_key_tree = identity_key_tree_path(identity_id);

        let key_id_bytes = encode_u16(id)?;
        self.batch_insert(
            PathFixedSizeKeyElement((
                identity_key_tree,
                key_id_bytes.as_slice(),
                Element::new_item_with_flags(serialized_identity_key, Some(element_flags.clone())),
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

                let exists = self.grove_has_raw(
                    purpose_path,
                    &[security_level],
                    if apply { None } else { Some(MAX_ELEMENT_SIZE) },
                    transaction,
                    drive_operations,
                )?;

                if exists == false {
                    // We need to insert the security level if it doesn't yet exist
                    self.batch_insert(
                        PathFixedSizeKeyElement((
                            purpose_path,
                            &[security_level],
                            Element::empty_tree_with_flags(Some(element_flags.clone())),
                        )),
                        drive_operations,
                    )?;
                }
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
        element_flags: ElementFlags,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let identity_path = identity_path(identity_id.as_slice());
        self.batch_insert(
            PathFixedSizeKeyElement((
                identity_path,
                Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys),
                Element::empty_tree_with_flags(Some(element_flags.clone())),
            )),
            drive_operations,
        )?;

        // We create the query trees structure
        self.create_new_identity_key_query_trees_operations(
            identity_id,
            element_flags.clone(),
            drive_operations,
        )?;

        for key in keys.into_iter() {
            self.insert_new_key_operations(
                identity_id.as_slice(),
                key,
                element_flags.clone(),
                false,
                apply,
                transaction,
                drive_operations,
            )?;
        }
        Ok(())
    }

    /// This creates the key query tree structure operations and adds them to the
    /// mutable drive_operations vector
    fn create_new_identity_key_query_trees_operations(
        &self,
        identity_id: [u8; 32],
        element_flags: ElementFlags,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let identity_key_tree = identity_key_tree_path(identity_id.as_slice());

        // We need to insert the query tree
        self.batch_insert(
            PathFixedSizeKeyElement((
                identity_key_tree,
                &[],
                Element::empty_tree_with_flags(Some(element_flags.clone())),
            )),
            drive_operations,
        )?;

        let identity_query_key_tree = identity_query_keys_tree_path(identity_id.as_slice());

        // There are 3 Purposes: Authentication, Encryption, Decryption
        for purpose in 0..3 {
            self.batch_insert(
                PathFixedSizeKeyElement((
                    identity_query_key_tree,
                    &[purpose],
                    Element::empty_tree_with_flags(Some(element_flags.clone())),
                )),
                drive_operations,
            )?;
        }
        // There are 4 Security Levels: Master, Critical, High, Medium
        // For the Authentication Purpose we insert every tree
        let identity_key_authentication_tree =
            identity_query_keys_purpose_tree_path(identity_id.as_slice(), &[0]);
        for security_level in 0..4 {
            self.batch_insert(
                PathFixedSizeKeyElement((
                    identity_key_authentication_tree,
                    &[security_level],
                    Element::empty_tree_with_flags(Some(element_flags.clone())),
                )),
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

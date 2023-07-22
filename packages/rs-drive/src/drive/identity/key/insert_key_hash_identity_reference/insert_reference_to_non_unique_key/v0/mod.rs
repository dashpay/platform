use crate::drive::defaults::{
    DEFAULT_HASH_160_SIZE_U8, DEFAULT_HASH_SIZE_U32, DEFAULT_HASH_SIZE_U8,
    ESTIMATED_NON_UNIQUE_KEY_DUPLICATES,
};

use crate::drive::object_size_info::DriveKeyInfo::KeyRef;
use crate::drive::object_size_info::PathKeyElementInfo::PathKeyRefElement;

use crate::drive::{
    non_unique_key_hashes_sub_tree_path_vec, non_unique_key_hashes_tree_path,
    non_unique_key_hashes_tree_path_vec, unique_key_hashes_tree_path_vec, Drive,
};
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation::FunctionOperation;
use crate::fee::op::{FunctionOp, HashFunction, LowLevelDriveOperation};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::IdentityPublicKey;
use dpp::version::drive_versions::DriveVersion;
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
    pub(super) fn insert_reference_to_non_unique_key_operations_v0(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        let hash_vec = identity_key.hash()?;
        let key_hash = hash_vec.as_slice().try_into().map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution("key hash not 20 bytes"))
        })?;

        let key_len = identity_key.data().len();
        drive_operations.push(FunctionOperation(FunctionOp::new_with_byte_count(
            HashFunction::Sha256RipeMD160,
            key_len as u16,
        )));

        //todo: check if key is unique

        self.insert_non_unique_public_key_hash_reference_to_identity_operations(
            identity_id,
            key_hash,
            estimated_costs_only_with_layer_info,
            transaction,
            drive_version,
        )
    }
}

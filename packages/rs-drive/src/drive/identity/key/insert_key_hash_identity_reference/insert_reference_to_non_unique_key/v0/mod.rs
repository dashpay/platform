use crate::drive::Drive;
use crate::error::drive::DriveError;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation::FunctionOperation;
use crate::fees::op::{FunctionOp, HashFunction, LowLevelDriveOperation};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::IdentityPublicKey;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg};
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
        let hash_vec = identity_key.public_key_hash()?;
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

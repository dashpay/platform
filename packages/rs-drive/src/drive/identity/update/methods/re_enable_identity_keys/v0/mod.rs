use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::identity::{IdentityPublicKey, KeyID};

use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    /// Re-enables a set of identity keys for a specific identity in version 0.
    ///
    /// This method is used to reverse the disabling of specific identity keys. If keys
    /// were previously disabled for the identity identified by `identity_id`,
    /// this method can be used to re-enable them.
    ///
    /// # Parameters
    ///
    /// * `identity_id`: A unique identifier for the identity, given as a 32-byte array.
    /// * `key_ids`: A vector of `KeyID` that represents the keys to be re-enabled.
    /// * `estimated_costs_only_with_layer_info`: An optional mutable reference to a map that,
    ///   if provided, will be populated with estimated layer information about the operation.
    ///   This can be useful for gauging the impact of the operation without actually executing it.
    ///   If `None`, the real operations for re-enabling the keys are executed.
    /// * `transaction`: A transaction argument used during the re-enabling process.
    /// * `platform_version`: Represents the platform version to ensure the operations are compatible.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `LowLevelDriveOperation` which represents the operations
    /// performed during the re-enabling process, or an `Error` if the process fails.
    ///
    #[inline(always)]
    pub(super) fn re_enable_identity_keys_operations_v0(
        &self,
        identity_id: [u8; 32],
        key_ids: Vec<KeyID>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let drive_version = &platform_version.drive;
        let mut drive_operations = vec![];

        let key_ids_len = key_ids.len();

        let keys: KeyIDIdentityPublicKeyPairVec = if let Some(
            estimated_costs_only_with_layer_info,
        ) = estimated_costs_only_with_layer_info
        {
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;
            key_ids
                .into_iter()
                .map(|key_id| {
                    Ok((
                        key_id,
                        IdentityPublicKey::max_possible_size_key(key_id, platform_version)?,
                    ))
                })
                .collect::<Result<Vec<_>, ProtocolError>>()?
        } else {
            let key_request = IdentityKeysRequest {
                identity_id,
                request_type: KeyRequestType::SpecificKeys(key_ids),
                limit: Some(key_ids_len as u16),
                offset: None,
            };

            self.fetch_identity_keys_operations(
                key_request,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
        };

        if keys.len() != key_ids_len {
            // TODO Choose / add an appropriate error
            return Err(Error::Drive(DriveError::UpdatingDocumentThatDoesNotExist(
                "key to re-enable with specified ID is not found",
            )));
        }

        const RE_ENABLE_KEY_TIME_BYTE_COST: i32 = 9;

        for (_, mut key) in keys {
            key.remove_disabled_at();

            let key_id_bytes = key.id().encode_var_vec();

            self.replace_key_in_storage_operations(
                identity_id.as_slice(),
                &key,
                &key_id_bytes,
                RE_ENABLE_KEY_TIME_BYTE_COST,
                &mut drive_operations,
                drive_version,
            )?;
        }

        Ok(drive_operations)
    }
}

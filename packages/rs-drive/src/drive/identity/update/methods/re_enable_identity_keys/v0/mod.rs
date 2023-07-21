use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    pub fn re_enable_identity_keys_operations_v0(
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
                    (
                        key_id,
                        IdentityPublicKey::max_possible_size_key(key_id, platform_version),
                    )
                })
                .collect()
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
            key.disabled_at = None;

            let key_id_bytes = key.id.encode_var_vec();

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

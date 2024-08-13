use dpp::block::block_info::BlockInfo;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use grovedb::batch::KeyInfoPath;

use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
};
use dpp::fee::fee_result::FeeResult;
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::TimestampMillis;

use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;
use grovedb::reference_path::ReferencePathType;
use grovedb::reference_path::ReferencePathType::SiblingReference;
use crate::drive::identity::{identity_contract_info_group_keys_path_vec, identity_contract_info_group_path_key_purpose_vec, identity_contract_info_root_path, identity_contract_info_root_path_vec, identity_key_path_vec, identity_query_keys_tree_path_vec};
use crate::util::object_size_info::PathKeyElementInfo;

impl Drive {
    /// Disable identity keys
    pub(super) fn disable_identity_keys_v0(
        &self,
        identity_id: [u8; 32],
        keys_ids: Vec<KeyID>,
        disable_at: TimestampMillis,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.disable_identity_keys_operations_v0(
            identity_id,
            keys_ids,
            disable_at,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None, // TODO: Does disable mean delete? Check if previous_fee_versions are required in this case
        )?;

        Ok(fees)
    }

    /// Disables a set of identity keys for a given identity in version 0.
    ///
    /// This method performs operations to disable specific identity keys for the identity
    /// identified by `identity_id`. The disabling is done by marking the keys as disabled at
    /// a specified timestamp (`disable_at`).
    ///
    /// # Parameters
    ///
    /// * `identity_id`: A unique identifier for the identity. It's a 32-byte array.
    /// * `key_ids`: A vector of `KeyID` that represents the keys to be disabled.
    /// * `disable_at`: A timestamp (in milliseconds) indicating when the keys should be marked as disabled.
    /// * `estimated_costs_only_with_layer_info`: An optional mutable reference to a map that,
    ///   if provided, will be populated with estimated layer information about the operation,
    ///   rather than performing the actual disabling of keys. If `None`, the actual operations
    ///   are executed.
    /// * `transaction`: A transaction argument used for the disabling process.
    /// * `platform_version`: Represents the platform version to ensure compatibility.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `LowLevelDriveOperation` which represents the operations
    /// performed during the disabling process, or an `Error` if the operation fails.
    ///
    #[inline(always)]
    pub(super) fn disable_identity_keys_operations_v0(
        &self,
        identity_id: [u8; 32],
        key_ids: Vec<KeyID>,
        disable_at: TimestampMillis,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        let drive_version = &platform_version.drive;

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
                "key to disable with specified ID is not found",
            )));
        }

        const DISABLE_KEY_TIME_BYTE_COST: i32 = 9;

        for (_, mut key) in keys {
            key.set_disabled_at(disable_at);

            let key_id_bytes = key.id().encode_var_vec();

            self.replace_key_in_storage_operations(
                identity_id.as_slice(),
                &key,
                &key_id_bytes,
                DISABLE_KEY_TIME_BYTE_COST,
                &mut drive_operations,
                drive_version,
            )?;

            // At this point, we need to refresh reference to that Identity key that was just updated (disable is an update)

            // This is the referenced to element
            let identity_key_path = identity_key_path_vec(&identity_id, key.id());
            let identity_key_reference = Element::Reference(
                ReferencePathType::AbsolutePathReference(identity_key_path),
                Some(1), // max hops
                None,
            );

            let trust_refresh_reference = true; // todo: check if this needs to be false

            // There are two references that needs to be refreshed:
            // 1) [Root ; <identity> ; Query Keys ; Purpose ; Security Level]
            let mut index_query_keys_path = identity_query_keys_tree_path_vec(identity_id);
            index_query_keys_path.push(vec![key.purpose() as u8]);
            index_query_keys_path.push(vec![key.security_level() as u8]);

            self.batch_refresh_reference(
                index_query_keys_path,
                key_id_bytes.to_vec(),
                identity_key_reference.clone(),
                trust_refresh_reference,
                &mut drive_operations,
                drive_version,
            )?;

            if let Some(contract_info) = key.contract_bounds() {
                // 2) [Root ; <identity> ; Contract Info ; Contract Bound ; Keys]
                let mut index_contract_info_path = identity_contract_info_root_path_vec(&identity_id);
                index_contract_info_path.push(contract_info.contract_bounds_type_string().as_bytes().to_vec()); // todo: Check if contract bound type string should be used in path?
                index_contract_info_path.push(vec![key.id() as u8]);

                self.batch_refresh_reference(
                    index_contract_info_path,
                    key_id_bytes.to_vec(),
                    identity_key_reference.clone(),
                    trust_refresh_reference,
                    &mut drive_operations,
                    drive_version,
                )?;

                if let Some(document_type) = contract_info.document_type() {
                    let root_id = vec![]; // todo: really not sure about this vec![]. IdentityDataContractKeyApplyInfo has a root_id() method but not ContractBounds

                    let mut contract_id_bytes_with_document_type_name = root_id.clone();
                    contract_id_bytes_with_document_type_name.extend(document_type.as_bytes());
                    let sibling_ref_key_purpose_path = identity_contract_info_group_path_key_purpose_vec(
                        &identity_id,
                        &contract_id_bytes_with_document_type_name,
                        key.purpose(),
                    );

                    self.batch_refresh_reference(
                        sibling_ref_key_purpose_path,
                        key_id_bytes.to_vec(),
                        Element::Reference(SiblingReference(key_id_bytes.to_vec()), Some(2), None),
                        trust_refresh_reference,
                        &mut drive_operations,
                        drive_version,
                    )?;

                    let sibling_ref_group_keys_path = identity_contract_info_group_keys_path_vec(
                        &identity_id,
                        &root_id.clone(),
                    );

                    self.batch_refresh_reference(
                        sibling_ref_group_keys_path,
                        key_id_bytes.to_vec(),
                        Element::Reference(SiblingReference(key_id_bytes.to_vec()), Some(2), None),
                        trust_refresh_reference,
                        &mut drive_operations,
                        drive_version,
                    )?;
                }
            }
        }

        Ok(drive_operations)
    }
}

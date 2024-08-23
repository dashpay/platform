use crate::drive::identity::contract_info::keys::IdentityDataContractKeyApplyInfo;
use crate::drive::identity::{
    identity_contract_info_group_keys_path_vec, identity_contract_info_group_path_key_purpose_vec,
    identity_key_location_within_identity_vec,
};
use crate::drive::Drive;
use crate::error::contract::DataContractError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, Purpose};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::{SiblingReference, UpstreamRootHeightReference};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use grovedb_costs::OperationCost;
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(in crate::drive::identity::contract_info) fn refresh_potential_contract_info_key_references_v0(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(contract_bounds) = &identity_key.contract_bounds() {
            // We need to get the contract
            let contract_apply_info = IdentityDataContractKeyApplyInfo::new_from_single_key(
                identity_key.id(),
                identity_key.purpose(),
                contract_bounds,
                self,
                epoch,
                transaction,
                drive_operations,
                platform_version,
            )?;
            self.refresh_contract_info_operations_v0(
                identity_id,
                epoch,
                vec![contract_apply_info],
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            )?;
        }
        Ok(())
    }

    /// Refreshes keys for the contract info
    fn refresh_contract_info_operations_v0(
        &self,
        identity_id: [u8; 32],
        epoch: &Epoch,
        contract_infos: Vec<IdentityDataContractKeyApplyInfo>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_info(
                &identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        for contract_info in contract_infos.into_iter() {
            let root_id = contract_info.root_id();

            let contract = if estimated_costs_only_with_layer_info.is_none() {
                // we should start by fetching the contract
                let (fee, contract) = self.get_contract_with_fetch_info_and_fee(
                    root_id,
                    Some(epoch),
                    true,
                    transaction,
                    platform_version,
                )?;

                let fee = fee.ok_or(Error::Identity(
                    IdentityError::IdentityKeyDataContractNotFound,
                ))?;
                let contract = contract.ok_or(Error::Identity(
                    IdentityError::IdentityKeyDataContractNotFound,
                ))?;
                drive_operations.push(LowLevelDriveOperation::PreCalculatedFeeResult(fee));
                Some(contract)
            } else {
                drive_operations.push(LowLevelDriveOperation::CalculatedCostOperation(
                    OperationCost {
                        seek_count: 1,
                        storage_cost: Default::default(),
                        storage_loaded_bytes: 100,
                        hash_node_calls: 0,
                    },
                ));
                None
            };

            let (document_keys, contract_or_family_keys) = contract_info.keys();

            for (key_id, purpose) in contract_or_family_keys {
                if let Some(estimated_costs_only_with_layer_info) =
                    estimated_costs_only_with_layer_info
                {
                    Self::add_estimation_costs_for_contract_info_group_key_purpose(
                        &identity_id,
                        &root_id,
                        purpose,
                        estimated_costs_only_with_layer_info,
                        &platform_version.drive,
                    )?;
                }

                // we need to add a reference to the key
                let key_id_bytes = key_id.encode_var_vec();
                let key_reference =
                    identity_key_location_within_identity_vec(key_id_bytes.as_slice());

                let reference_type_path = UpstreamRootHeightReference(2, key_reference);

                // at this point we want to know if the contract is single key or multiple key
                let storage_key_requirements = contract
                    .as_ref()
                    .map(|contract| match purpose {
                        Purpose::ENCRYPTION => {
                            let encryption_storage_key_requirements = contract
                                .contract
                                .config()
                                .requires_identity_encryption_bounded_key()
                                .ok_or(Error::DataContract(
                                    DataContractError::KeyBoundsExpectedButNotPresent(
                                        "expected encryption key bounds for encryption",
                                    ),
                                ))?;
                            Ok(encryption_storage_key_requirements)
                        }
                        Purpose::DECRYPTION => {
                            let decryption_storage_key_requirements = contract
                                .contract
                                .config()
                                .requires_identity_decryption_bounded_key()
                                .ok_or(Error::DataContract(
                                    DataContractError::KeyBoundsExpectedButNotPresent(
                                        "expected encryption key bounds for decryption",
                                    ),
                                ))?;
                            Ok(decryption_storage_key_requirements)
                        }
                        _ => Err(Error::Identity(IdentityError::IdentityKeyBoundsError(
                            "purpose not available for key bounds",
                        ))),
                    })
                    .transpose()?
                    .unwrap_or(StorageKeyRequirements::MultipleReferenceToLatest);

                // if we are multiple we refresh the key under the key bytes, otherwise it is under 0

                let key = if storage_key_requirements == StorageKeyRequirements::Unique {
                    vec![]
                } else {
                    key_id_bytes.clone()
                };

                self.batch_refresh_reference(
                    identity_contract_info_group_path_key_purpose_vec(
                        &identity_id,
                        &root_id,
                        purpose,
                    ),
                    key,
                    Element::Reference(reference_type_path, Some(1), None),
                    true,
                    drive_operations,
                    &platform_version.drive,
                )?;

                if storage_key_requirements == StorageKeyRequirements::MultipleReferenceToLatest {
                    // we also refresh the sibling reference, so we can query the current key

                    let sibling_ref_type_path = SiblingReference(key_id_bytes);

                    self.batch_refresh_reference(
                        identity_contract_info_group_keys_path_vec(&identity_id, &root_id),
                        vec![],
                        Element::Reference(sibling_ref_type_path, Some(2), None),
                        true,
                        drive_operations,
                        &platform_version.drive,
                    )?;
                }
            }

            for (document_type_name, document_key_ids) in document_keys {
                // The path is the concatenation of the contract_id and the document type name
                let mut contract_id_bytes_with_document_type_name = root_id.to_vec();
                contract_id_bytes_with_document_type_name.extend(document_type_name.as_bytes());

                if let Some(estimated_costs_only_with_layer_info) =
                    estimated_costs_only_with_layer_info
                {
                    Self::add_estimation_costs_for_contract_info_group(
                        &identity_id,
                        &contract_id_bytes_with_document_type_name,
                        estimated_costs_only_with_layer_info,
                        &platform_version.drive,
                    )?;

                    Self::add_estimation_costs_for_contract_info_group_keys(
                        &identity_id,
                        &contract_id_bytes_with_document_type_name,
                        estimated_costs_only_with_layer_info,
                        &platform_version.drive,
                    )?;
                }

                for (key_id, purpose) in document_key_ids {
                    if let Some(estimated_costs_only_with_layer_info) =
                        estimated_costs_only_with_layer_info
                    {
                        Self::add_estimation_costs_for_contract_info_group_key_purpose(
                            &identity_id,
                            &contract_id_bytes_with_document_type_name,
                            purpose,
                            estimated_costs_only_with_layer_info,
                            &platform_version.drive,
                        )?;
                    }

                    // we need to add a reference to the key
                    let key_id_bytes = key_id.encode_var_vec();
                    let key_reference =
                        identity_key_location_within_identity_vec(key_id_bytes.as_slice());

                    let reference = UpstreamRootHeightReference(2, key_reference);

                    // at this point we want to know if the contract is single key or multiple key
                    let storage_key_requirements = contract
                        .as_ref()
                        .map(|contract| match purpose {
                            Purpose::ENCRYPTION => {
                                let document_type = contract
                                    .contract
                                    .document_type_for_name(document_type_name.as_str())?;
                                let encryption_storage_key_requirements = document_type
                                    .requires_identity_encryption_bounded_key()
                                    .ok_or(Error::DataContract(
                                        DataContractError::KeyBoundsExpectedButNotPresent(
                                            "expected encryption key bounds in document type",
                                        ),
                                    ))?;
                                Ok(encryption_storage_key_requirements)
                            }
                            Purpose::DECRYPTION => {
                                let document_type = contract
                                    .contract
                                    .document_type_for_name(document_type_name.as_str())?;
                                let decryption_storage_key_requirements = document_type
                                    .requires_identity_decryption_bounded_key()
                                    .ok_or(Error::DataContract(
                                        DataContractError::KeyBoundsExpectedButNotPresent(
                                            "expected encryption key bounds in document type",
                                        ),
                                    ))?;
                                Ok(decryption_storage_key_requirements)
                            }
                            _ => Err(Error::Identity(IdentityError::IdentityKeyBoundsError(
                                "purpose not available for key bounds",
                            ))),
                        })
                        .transpose()?
                        .unwrap_or(StorageKeyRequirements::MultipleReferenceToLatest);

                    let key = if storage_key_requirements == StorageKeyRequirements::Unique {
                        vec![]
                    } else {
                        key_id_bytes.clone()
                    };

                    self.batch_refresh_reference(
                        identity_contract_info_group_path_key_purpose_vec(
                            &identity_id,
                            &contract_id_bytes_with_document_type_name,
                            purpose,
                        ),
                        key,
                        Element::Reference(reference, Some(1), None),
                        true,
                        drive_operations,
                        &platform_version.drive,
                    )?;

                    if storage_key_requirements == StorageKeyRequirements::MultipleReferenceToLatest
                    {
                        // we also need to refresh the sibling reference, so we can query the current key

                        let sibling_ref_type_path = SiblingReference(key_id_bytes);

                        self.batch_refresh_reference(
                            identity_contract_info_group_path_key_purpose_vec(
                                &identity_id,
                                &contract_id_bytes_with_document_type_name,
                                purpose,
                            ),
                            vec![],
                            Element::Reference(sibling_ref_type_path, Some(2), None),
                            true,
                            drive_operations,
                            &platform_version.drive,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }
}

use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::drive::identity::contract_info::insert::DataContractApplyInfo;
use crate::drive::identity::IdentityRootStructure::IdentityContractInfo;
use crate::drive::identity::{
    identity_contract_info_group_path_vec, identity_contract_info_root_path_vec,
    identity_key_location_within_identity_vec, identity_path_vec,
};
use crate::drive::object_size_info::{PathKeyElementInfo, PathKeyInfo};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::IdentityPublicKey;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::UpstreamRootHeightReference;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_potential_contract_info_for_contract_bounded_key_v0(
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
            let contract_apply_info = DataContractApplyInfo::new_from_single_key(
                identity_key.id(),
                contract_bounds,
                self,
                epoch,
                transaction,
                drive_operations,
                platform_version,
            )?;
            self.add_contract_info_operations_v0(
                identity_id,
                vec![contract_apply_info],
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            )?;
        }
        Ok(())
    }

    /// Adds the contract info operations
    fn add_contract_info_operations_v0(
        &self,
        identity_id: [u8; 32],
        contract_infos: Vec<DataContractApplyInfo>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let identity_path = identity_path_vec(identity_id.as_slice());

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_info(
                &identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: 0,
            }
        };

        // we insert the contract root tree if it doesn't exist already
        self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
            PathKeyInfo::<0>::PathKey((identity_path, vec![IdentityContractInfo as u8])),
            None,
            apply_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        for contract_info in contract_infos.into_iter() {
            let root_id = contract_info.root_id();

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Self::add_estimation_costs_for_contract_info_group(
                    &identity_id,
                    &root_id,
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
            }

            self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
                PathKeyInfo::<0>::PathKey((
                    identity_contract_info_root_path_vec(&identity_id),
                    root_id.to_vec(),
                )),
                None,
                apply_type,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?;
            let (document_keys, contract_or_family_keys) = contract_info.keys();

            for key_id in contract_or_family_keys {
                // we need to add a reference to the key
                let key_id_bytes = key_id.encode_var_vec();
                let key_reference =
                    identity_key_location_within_identity_vec(key_id_bytes.as_slice());

                let reference_type_path = UpstreamRootHeightReference(1, key_reference);

                let ref_apply_type = if estimated_costs_only_with_layer_info.is_none() {
                    BatchInsertApplyType::StatefulBatchInsert
                } else {
                    BatchInsertApplyType::StatelessBatchInsert {
                        in_tree_using_sums: false,
                        target: QueryTargetValue(reference_type_path.serialized_size() as u32),
                    }
                };

                self.batch_insert_if_not_exists(
                    PathKeyElementInfo::<0>::PathKeyRefElement((
                        identity_contract_info_group_path_vec(&identity_id, &root_id),
                        key_id_bytes.as_slice(),
                        Element::Reference(reference_type_path, Some(1), None),
                    )),
                    ref_apply_type,
                    transaction,
                    drive_operations,
                    &platform_version.drive,
                )?;
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
                }

                self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
                    PathKeyInfo::<0>::PathKey((
                        identity_contract_info_root_path_vec(&identity_id),
                        contract_id_bytes_with_document_type_name.to_vec(),
                    )),
                    None,
                    apply_type,
                    transaction,
                    drive_operations,
                    &platform_version.drive,
                )?;
                for key_id in document_key_ids {
                    // we need to add a reference to the key
                    let key_id_bytes = key_id.encode_var_vec();
                    let key_reference =
                        identity_key_location_within_identity_vec(key_id_bytes.as_slice());

                    let reference = UpstreamRootHeightReference(1, key_reference);

                    let ref_apply_type = if estimated_costs_only_with_layer_info.is_none() {
                        BatchInsertApplyType::StatefulBatchInsert
                    } else {
                        BatchInsertApplyType::StatelessBatchInsert {
                            in_tree_using_sums: false,
                            target: QueryTargetValue(reference.serialized_size() as u32),
                        }
                    };

                    self.batch_insert_if_not_exists(
                        PathKeyElementInfo::<0>::PathKeyRefElement((
                            identity_contract_info_group_path_vec(
                                &identity_id,
                                &contract_id_bytes_with_document_type_name,
                            ),
                            key_id_bytes.as_slice(),
                            Element::Reference(reference, Some(1), None),
                        )),
                        ref_apply_type,
                        transaction,
                        drive_operations,
                        &platform_version.drive,
                    )?;
                }
            }
        }

        Ok(())
    }
}

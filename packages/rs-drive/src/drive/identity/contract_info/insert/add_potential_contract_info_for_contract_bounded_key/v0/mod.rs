use crate::drive::grove_operations::BatchInsertApplyType::StatefulBatchInsert;
use crate::drive::grove_operations::BatchInsertTreeApplyType::StatefulBatchInsertTree;
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
        // we insert the contract root tree if it doesn't exist already
        self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
            PathKeyInfo::<0>::PathKey((identity_path, vec![IdentityContractInfo as u8])),
            None,
            StatefulBatchInsertTree,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        for contract_info in contract_infos.into_iter() {
            let root_id = contract_info.root_id();
            self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
                PathKeyInfo::<0>::PathKey((
                    identity_contract_info_root_path_vec(identity_id.as_slice()),
                    root_id.to_vec(),
                )),
                None,
                StatefulBatchInsertTree,
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

                self.batch_insert_if_not_exists(
                    PathKeyElementInfo::<0>::PathKeyRefElement((
                        identity_contract_info_group_path_vec(
                            identity_id.as_slice(),
                            root_id.as_slice(),
                        ),
                        key_id_bytes.as_slice(),
                        Element::Reference(
                            UpstreamRootHeightReference(1, key_reference),
                            Some(1),
                            None,
                        ),
                    )),
                    StatefulBatchInsert,
                    transaction,
                    drive_operations,
                    &platform_version.drive,
                )?;
            }

            for (document_type, document_key_ids) in document_keys {
                self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
                    PathKeyInfo::<0>::PathKey((
                        identity_contract_info_root_path_vec(identity_id.as_slice()),
                        root_id.to_vec(),
                    )),
                    None,
                    StatefulBatchInsertTree,
                    transaction,
                    drive_operations,
                    &platform_version.drive,
                )?;
                for key_id in document_key_ids {
                    // we need to add a reference to the key
                    let key_id_bytes = key_id.encode_var_vec();
                    let key_reference =
                        identity_key_location_within_identity_vec(key_id_bytes.as_slice());

                    self.batch_insert_if_not_exists(
                        PathKeyElementInfo::<0>::PathKeyRefElement((
                            identity_contract_info_group_path_vec(
                                identity_id.as_slice(),
                                root_id.as_slice(),
                            ),
                            key_id_bytes.as_slice(),
                            Element::Reference(
                                UpstreamRootHeightReference(1, key_reference),
                                Some(1),
                                None,
                            ),
                        )),
                        StatefulBatchInsert,
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

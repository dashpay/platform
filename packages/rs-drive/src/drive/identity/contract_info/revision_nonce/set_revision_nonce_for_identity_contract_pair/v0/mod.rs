use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::drive::identity::contract_info::ContractInfoStructure::IdentityContractNonce;
use crate::drive::identity::IdentityRootStructure::IdentityContractInfo;
use crate::drive::identity::{
    identity_contract_info_group_path, identity_contract_info_group_path_key_purpose_vec,
    identity_contract_info_group_path_vec, identity_contract_info_root_path_vec,
    identity_key_location_within_identity_vec, identity_path_vec,
};
use crate::drive::object_size_info::{PathKeyElementInfo, PathKeyInfo};
use crate::drive::Drive;
use crate::error::contract::DataContractError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use dpp::identity::{IdentityPublicKey, Purpose};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::{SiblingReference, UpstreamRootHeightReference};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use grovedb_costs::OperationCost;
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    pub(in crate::drive::identity::contract_info) fn set_revision_nonce_for_identity_contract_pair_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        revision_nonce: u64,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.set_revision_nonce_for_identity_contract_pair_operations_v0(
            identity_id,
            contract_id,
            revision_nonce,
            epoch,
            estimated_costs_only_with_layer_info,
            transaction,
            drive_operations,
            platform_version,
        )
    }

    /// Sets the revision nonce for the identity contract pair
    fn set_revision_nonce_for_identity_contract_pair_operations_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        revision_nonce: u64,
        epoch: &Epoch,
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

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_info_group(
                &identity_id,
                &contract_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let identity_contract_nonce_bytes = revision_nonce.encode_var_vec();
        let identity_contract_nonce_element = Element::new_item(identity_contract_nonce_bytes);

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertApplyType::StatefulBatchInsert
        } else {
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_using_sums: false,
                target: QueryTargetValue(16), //assume that in most cases revision will be under 2^16
            }
        };

        self.batch_insert_if_changed_value(
            PathKeyElementInfo::PathFixedSizeKeyRefElement((
                identity_contract_info_group_path(&identity_id, &contract_id),
                &[IdentityContractNonce as u8],
                identity_contract_nonce_element,
            )),
            apply_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        Ok(())
    }
}

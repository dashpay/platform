use crate::drive::document::paths::{contract_document_type_path, DOCUMENT_HISTORY_TREE_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DriveKeyInfo::KeyRef;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Inserts the contract with all generation 2 trees and each retained type's history tree.
    #[inline(always)]
    pub(super) fn insert_contract_v3(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_element =
            Self::contract_element_for_insert_v1(contract, &block_info, platform_version)?;

        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.insert_contract_operations_v3(
            contract_element,
            contract,
            &block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    /// The operations for adding a contract, appended to `drive_operations`.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_contract_add_operations_v3(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_contract_operations_v3(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// Collects generation 2 writes and the per-type history trees in the same batch.
    fn insert_contract_operations_v3(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let flags = contract_element.get_flags().clone();
        let storage_flags = StorageFlags::map_some_element_flags_ref(&flags)?;
        let mut operations = vec![];
        self.insert_contract_add_operations_v2(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            &mut operations,
            transaction,
            platform_version,
        )?;
        for (name, document_type) in contract.document_types() {
            if document_type.documents_keep_history() {
                self.batch_insert_empty_tree(
                    contract_document_type_path(contract.id_ref().as_bytes(), name),
                    KeyRef(&[DOCUMENT_HISTORY_TREE_KEY]),
                    storage_flags.as_ref(),
                    &mut operations,
                    &platform_version.drive,
                )?;
            }
        }
        Ok(operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    #[test]
    fn should_create_the_history_tree_only_from_protocol_15() {
        use crate::drive::document::paths::{
            contract_document_type_path_vec, DOCUMENT_HISTORY_TREE_KEY,
        };
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::tests::json_document::json_document_to_contract;

        for (protocol, expected) in [(14, false), (15, true)] {
            let version = PlatformVersion::get(protocol).expect("protocol version");
            let drive = setup_drive_with_initial_state_structure(Some(version));
            let contract = json_document_to_contract(
                "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
                false,
                version,
            )
            .expect("keep-history contract fixture");
            drive
                .insert_contract(&contract, BlockInfo::default(), true, None, version)
                .expect("insert contract");
            let type_path = contract_document_type_path_vec(contract.id().as_slice(), "profile");
            let history_tree = drive
                .grove
                .get_raw(
                    type_path.as_slice().into(),
                    &[DOCUMENT_HISTORY_TREE_KEY],
                    None,
                    &version.drive.grove_version,
                )
                .value;
            assert_eq!(
                history_tree.is_ok(),
                expected,
                "protocol {protocol}: {history_tree:?}"
            );
        }
    }
}

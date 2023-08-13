use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE};

use crate::drive::flags::StorageFlags;
use crate::drive::{contract_documents_path, Drive};

use dpp::data_contract::DataContract;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::AllSubtrees;

use crate::drive::contract::paths::{all_contracts_global_root_path, contract_root_path};
use crate::error::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::version::drive_versions::DriveVersion;
use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_levels_up_to_contract_document_type_excluded_v0(
        contract: &DataContract,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_levels_up_to_contract(
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        let document_type_count = contract.document_types().len() as u32;

        // we only store the owner_id storage
        let storage_flags = if contract.config().can_be_deleted() {
            Some(StorageFlags::approximate_size(true, None))
        } else {
            None
        };

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_root_path(contract.id_ref().as_bytes())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, storage_flags),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_documents_path(contract.id_ref().as_bytes())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(document_type_count),
                estimated_layer_sizes: AllSubtrees(
                    ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE,
                    NoSumTrees,
                    storage_flags,
                ),
            },
        );

        Ok(())
    }
}

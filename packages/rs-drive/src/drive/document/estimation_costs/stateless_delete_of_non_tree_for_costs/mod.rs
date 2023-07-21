mod v0;

use crate::drive::document::contract_documents_keeping_history_primary_key_path_for_document_id;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::{BatchDeleteUpTreeApplyType, IsSubTree, IsSumSubTree};

use crate::drive::object_size_info::DocumentAndContractInfo;
use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;

use crate::error::drive::DriveError;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, EstimatedLayerSizes};
use intmap::IntMap;
use itertools::Itertools;
use std::collections::HashMap;

impl Drive {
    pub(crate) fn stateless_delete_of_non_tree_for_costs(
        element_estimated_sizes: EstimatedLayerSizes,
        key_info_path: &KeyInfoPath,
        is_known_to_be_subtree_with_sum: Option<(IsSubTree, IsSumSubTree)>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<BatchDeleteUpTreeApplyType, Error> {
        match platform_version
            .drive
            .methods
            .document
            .estimation_costs
            .stateless_delete_of_non_tree_for_costs
        {
            0 => Self::stateless_delete_of_non_tree_for_costs_v0(
                element_estimated_sizes,
                key_info_path,
                is_known_to_be_subtree_with_sum,
                estimated_costs_only_with_layer_info,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "Drive::stateless_delete_of_non_tree_for_costs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

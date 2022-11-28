use crate::drive::contract::all_contracts_global_root_path;
use crate::drive::defaults::DEFAULT_HASH_SIZE_U8;
use crate::drive::flags::StorageFlags;
use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerInformation::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use std::collections::HashMap;

impl Drive {
    pub(crate) fn add_estimation_costs_for_levels_up_to_contract(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLevel(0, false, AllSubtrees(1, None)),
        );

        // we then need to insert the contract layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(all_contracts_global_root_path()),
            PotentiallyAtMaxElements(AllSubtrees(
                DEFAULT_HASH_SIZE_U8,
                Some(StorageFlags::approximate_size(true, None)),
            )),
        );
    }
}

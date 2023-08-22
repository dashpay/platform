use crate::drive::defaults::CONTRACT_DOCUMENTS_PATH_HEIGHT;

use crate::drive::grove_operations::{BatchDeleteUpTreeApplyType, IsSubTree, IsSumSubTree};

use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;

use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, EstimatedLayerSizes};
use intmap::IntMap;
use itertools::Itertools;
use std::collections::HashMap;

impl Drive {
    pub(super) fn stateless_delete_of_non_tree_for_costs_v0(
        element_estimated_sizes: EstimatedLayerSizes,
        key_info_path: &KeyInfoPath,
        is_known_to_be_subtree_with_sum: Option<(IsSubTree, IsSumSubTree)>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
    ) -> Result<BatchDeleteUpTreeApplyType, Error> {
        // Keep for debugging
        // if estimated_costs_only_with_layer_info.is_some() {
        //     for (k, l) in estimated_costs_only_with_layer_info.as_ref().unwrap() {
        //         let path = k
        //             .to_path()
        //             .iter()
        //             .map(|k| hex::encode(k.as_slice()))
        //             .join("/");
        //         dbg!(path, l);
        //     }
        // }
        estimated_costs_only_with_layer_info.as_ref().map_or(
            Ok(BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum,
            }),
            |layer_info| {
                let mut layer_map = (CONTRACT_DOCUMENTS_PATH_HEIGHT..(key_info_path.len() as u16))
                    .map(|s| {
                        let subpath =
                            KeyInfoPath::from_vec(key_info_path.0[..(s as usize)].to_vec());
                        let layer_info = layer_info.get(&subpath).ok_or(Error::Fee(
                            FeeError::CorruptedEstimatedLayerInfoMissing(format!(
                                "layer info missing at path {}",
                                subpath
                                    .0
                                    .iter()
                                    .map(|k| hex::encode(k.as_slice()))
                                    .join("/")
                            )),
                        ))?;

                        Ok((s as u64, layer_info.clone()))
                    })
                    .collect::<Result<IntMap<EstimatedLayerInformation>, Error>>()?;
                // We need to update the current layer to only have 1 element that we want to delete
                let mut last_layer_information = layer_map
                    .remove((key_info_path.len() - 1) as u64)
                    .ok_or(Error::Fee(FeeError::CorruptedEstimatedLayerInfoMissing(
                        "last layer info missing".to_owned(),
                    )))?;
                last_layer_information.estimated_layer_sizes = element_estimated_sizes;
                layer_map.insert((key_info_path.len() - 1) as u64, last_layer_information);
                Ok(BatchDeleteUpTreeApplyType::StatelessBatchDelete {
                    estimated_layer_info: layer_map,
                })
            },
        )
    }
}

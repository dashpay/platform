use crate::drive::constants::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::util::estimated_layers::EstimatedLayers;

use crate::util::grove_operations::BatchDeleteUpTreeApplyType;

use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;

use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, EstimatedLayerSizes, MaybeTree};
use intmap::IntMap;
use itertools::Itertools;

impl Drive {
    /// Attempts a stateless deletion of non-tree elements for costs estimation.
    ///
    /// This function either executes a stateful batch delete or a stateless batch delete based
    /// on the presence of the `estimated_costs_only_with_layer_info` parameter.
    ///
    /// - When `estimated_costs_only_with_layer_info` is `None`, it directly performs a stateful batch delete.
    /// - When `estimated_costs_only_with_layer_info` is `Some`, it retrieves the relevant layer
    ///   information and performs a stateless batch delete. In this case, any missing layer
    ///   information results in an error.
    ///
    /// # Parameters
    /// - `element_estimated_sizes`: An estimate of the layer sizes for the element to be deleted.
    /// - `key_info_path`: The path of the key for which the deletion is to be estimated.
    /// - `is_known_to_be_subtree_with_sum`: Optional information about the subtree and sum-subtree status.
    /// - `estimated_costs_only_with_layer_info`: Optionally, a reference to the estimated costs with layer info.
    ///
    /// # Returns
    /// - `Ok(BatchDeleteUpTreeApplyType)`: The type of batch delete operation (either stateful or stateless).
    /// - `Err(Error)`: An error if there is a problem retrieving layer information.
    ///
    /// # Errors
    /// Returns an `Error::Fee(FeeError::CorruptedEstimatedLayerInfoMissing)` if the required layer
    /// information is missing in the provided estimated costs.
    ///
    /// v0 gives height `s` the description of the path's first `s` keys, a layer above the one
    /// GroveDB prices at that height (v1 corrects it from protocol version 15).
    #[inline(always)]
    pub(super) fn stateless_delete_of_non_tree_for_costs_v0(
        element_estimated_sizes: EstimatedLayerSizes,
        key_info_path: &KeyInfoPath,
        is_known_to_be_subtree_with_sum: Option<MaybeTree>,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
    ) -> Result<BatchDeleteUpTreeApplyType, Error> {
        Self::stateless_delete_of_non_tree_with_layer_paths(
            element_estimated_sizes,
            key_info_path,
            is_known_to_be_subtree_with_sum,
            estimated_costs_only_with_layer_info,
            |height| height as usize,
        )
    }

    /// v0's deletion, giving each height from `CONTRACT_DOCUMENTS_PATH_HEIGHT` up to the deleted
    /// key's the description of the path's first `layer_path_len(height)` keys, as the estimate
    /// last wrote it. The reading operation describes each of those layers just before, so the
    /// read is the same whatever else its batch describes.
    #[inline(always)]
    pub(super) fn stateless_delete_of_non_tree_with_layer_paths(
        element_estimated_sizes: EstimatedLayerSizes,
        key_info_path: &KeyInfoPath,
        is_known_to_be_subtree_with_sum: Option<MaybeTree>,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        layer_path_len: fn(u16) -> usize,
    ) -> Result<BatchDeleteUpTreeApplyType, Error> {
        // Keep for debugging
        // dbg!(&estimated_costs_only_with_layer_info);
        estimated_costs_only_with_layer_info.as_ref().map_or(
            Ok(BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum,
            }),
            |layer_info| {
                let mut layer_map = (CONTRACT_DOCUMENTS_PATH_HEIGHT..(key_info_path.len() as u16))
                    .map(|s| {
                        let subpath =
                            KeyInfoPath::from_vec(key_info_path.0[..layer_path_len(s)].to_vec());
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

                        Ok((s, *layer_info))
                    })
                    .collect::<Result<IntMap<u16, EstimatedLayerInformation>, Error>>()?;
                // We need to update the current layer to only have 1 element that we want to delete
                let mut last_layer_information = layer_map
                    .remove((key_info_path.len() - 1) as u16)
                    .ok_or(Error::Fee(FeeError::CorruptedEstimatedLayerInfoMissing(
                        "last layer info missing".to_owned(),
                    )))?;
                last_layer_information.estimated_layer_sizes = element_estimated_sizes;
                layer_map.insert((key_info_path.len() - 1) as u16, last_layer_information);
                Ok(BatchDeleteUpTreeApplyType::StatelessBatchDelete {
                    estimated_layer_info: layer_map,
                })
            },
        )
    }
}

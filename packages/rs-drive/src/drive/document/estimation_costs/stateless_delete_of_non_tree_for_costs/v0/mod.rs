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
<<<<<<< HEAD
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
=======
    /// Deletes an element in a stateless manner for computing costs.
    ///
    /// This function performs a stateless delete operation, specifically for 
    /// computing costs in GroveDB.
    ///
    /// # Parameters
    ///
    /// - `element_estimated_sizes`: Estimated sizes of the layer to be deleted.
    /// - `key_info_path`: Path information to locate the element in the tree.
    /// - `is_known_to_be_subtree_with_sum`: Specifies whether the node is known to be a subtree 
    ///   and whether it's a sum-subtree. This is an `Option` containing a tuple where
    ///   - `IsSubTree` signifies whether it's a subtree or not.
    ///   - `IsSumSubTree` signifies whether it's a sum-subtree or not.
    /// - `estimated_costs_only_with_layer_info`: Mutable reference to an optional hashmap that holds 
    ///   layer information for cost estimation.
    ///
    /// # Returns
    ///
    /// - `Ok(BatchDeleteUpTreeApplyType)`: The type of batch delete that should be applied up the tree.
    /// - `Err(Error)`: An error occurred, which is defined by the custom `Error` type.
    ///
    /// # Errors
    ///
    /// - `Error::Fee(FeeError::CorruptedEstimatedLayerInfoMissing)`: Indicates missing layer information
    ///   in `estimated_costs_only_with_layer_info`.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Assuming all types and Error variants are appropriately defined
    /// let estimated_sizes = EstimatedLayerSizes { /* fields */ };
    /// let key_info_path = &KeyInfoPath::new(/* arguments */);
    /// let subtree_info = Some((IsSubTree::Yes, IsSumSubTree::No));
    /// let mut layer_info: Option<HashMap<KeyInfoPath, EstimatedLayerInformation>> = Some(HashMap::new());
    ///
    /// let result = Drive::stateless_delete_of_non_tree_for_costs_v0(
    ///     estimated_sizes,
    ///     key_info_path,
    ///     subtree_info,
    ///     &mut layer_info
    /// );
    /// ```
>>>>>>> 6ac041d9e (feat: add docs)
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

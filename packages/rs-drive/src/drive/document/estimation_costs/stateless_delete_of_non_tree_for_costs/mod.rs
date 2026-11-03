mod v0;
mod v1;

use crate::util::estimated_layers::EstimatedLayers;
use crate::util::grove_operations::BatchDeleteUpTreeApplyType;

use crate::drive::Drive;

use crate::error::Error;

use crate::error::drive::DriveError;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerSizes, MaybeTree};

impl Drive {
    /// Tries to perform a stateless deletion of non-tree elements based on the specified platform version.
    ///
    /// This function routes the deletion request to the appropriate version of the
    /// `stateless_delete_of_non_tree_for_costs` function based on the provided `platform_version`.
    ///
    /// Version `0` gives each height the layer above the one GroveDB prices there, version `1`
    /// (protocol version 15) that layer; any other version is an error.
    ///
    /// # Parameters
    /// - `element_estimated_sizes`: An estimate of the layer sizes for the element intended for deletion.
    /// - `key_info_path`: The path of the key for which the deletion is being estimated.
    /// - `is_known_to_be_subtree_with_sum`: Optional details about the subtree and sum-subtree status.
    /// - `estimated_costs_only_with_layer_info`: Optionally, a reference to the estimated costs containing layer info.
    /// - `platform_version`: The version information to determine which version of the function to use.
    ///
    /// # Returns
    /// - `Ok(BatchDeleteUpTreeApplyType)`: The type of batch delete operation (either stateful or stateless).
    /// - `Err(Error)`: An error if an unsupported version is provided.
    ///
    /// # Errors
    /// Returns an `Error::Drive(DriveError::UnknownVersionMismatch)` if an unsupported `platform_version` is used.
    pub(crate) fn stateless_delete_of_non_tree_for_costs(
        element_estimated_sizes: EstimatedLayerSizes,
        key_info_path: &KeyInfoPath,
        is_known_to_be_subtree_with_sum: Option<MaybeTree>,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
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
            // v1 (protocol version 15): each height is given the layer GroveDB prices there.
            1 => Self::stateless_delete_of_non_tree_for_costs_v1(
                element_estimated_sizes,
                key_info_path,
                is_known_to_be_subtree_with_sum,
                estimated_costs_only_with_layer_info,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "Drive::stateless_delete_of_non_tree_for_costs".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

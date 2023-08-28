// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Implements in Drive a function which adds estimated costs to a hashmap for adding an asset lock (version 0).

use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};

use crate::drive::asset_lock::asset_lock_storage_path;
use grovedb::EstimatedSumTrees::SomeSumTrees;
use std::collections::HashMap;

impl Drive {
    /// Add estimated costs to a hashmap for adding an asset lock (version 0).
    ///
    /// This function modifies the provided hashmap, `estimated_costs_only_with_layer_info`,
    /// by inserting two sets of key-value pairs related to the estimation costs for adding an asset lock.
    ///
    /// The function assumes:
    /// - The top layer has already been constructed so that contract/documents tree are at the top.
    /// - The balance is on layer 2, so updating this will mean updating 1 sum tree and 1 normal tree.
    ///   This is why an equal weight is given to both types of trees in the estimation.
    ///
    /// # Arguments
    ///
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to a hashmap that will store
    ///   the estimated layer information associated with the key info paths.
    ///
    /// # KeyInfoPath Definitions:
    ///
    /// * First insertion: An empty key info path.
    /// * Second insertion: Uses the `asset_lock_storage_path()` function to derive the path.
    ///
    /// # Example Layer Information:
    ///
    /// * For the first insertion, it is assumed that:
    ///   - The layer is not a sum tree.
    ///   - There's an estimated level of 1 which is not a sum tree.
    ///   - There's equal weight given to sum trees and non-sum trees.
    ///
    /// * For the second insertion, it is assumed that:
    ///   - The layer is not a sum tree.
    ///   - The layer might potentially have max elements.
    ///   - Each item in this layer has a size of 36, which represents the size of an outpoint.
    ///
    /// # Notes
    ///
    /// The todo comment suggests that there may be inaccuracies in the current function logic. Ensure to verify
    /// the correctness of the provided logic and assumptions before relying on this function in production.
    pub(crate) fn add_estimation_costs_for_adding_asset_lock_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        //todo: verify (this is wrong)
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 2, updating will mean we will update 1 sum tree
        // and 1 normal tree, hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        non_sum_trees_weight: 1,
                    },
                    None,
                ),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(asset_lock_storage_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    36, //The size of an outpoint
                    0, None,
                ),
            },
        );
    }
}

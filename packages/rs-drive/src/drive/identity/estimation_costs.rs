// MIT LICENSE
//
// Copyright (c) 2022 Dash Core Group
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

// MIT LICENSE
//
// Copyright (c) 2022 Dash Core Group
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

use crate::drive::defaults::{AVERAGE_BALANCE_SIZE, DEFAULT_HASH_SIZE_U8};

use crate::drive::{identity_tree_path, unique_key_hashes_tree_path_vec, Drive};

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees, Mix};

use crate::drive::identity::{
    identity_key_tree_path_vec, identity_path_vec, identity_query_keys_purpose_tree_path,
    identity_query_keys_purpose_tree_path_vec, identity_query_keys_security_level_tree_path_vec,
    identity_query_keys_tree_path_vec,
};

use crate::drive::balances::balance_path_vec;
use dpp::identity::{Purpose, SecurityLevel};
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use std::collections::HashMap;

// we need to construct the reference from the split height of the key
// type which is at 4
// 0 represents document storage
// Then we add document id
// Then we add 0 if the document type keys history
// vec![vec![0], Vec::from(key.id)];
// 1 (vec size) + 1 (subvec size) + 1 (0) + 1 (subvec size) + 32 (key id size)
// 1 for type reference
// 1 for reference type
// 1 for root height offset
// reference path size
// 1 reference_hops options
// 1 reference_hops count
// 1 element flags option
/// size of a key reference.
pub const KEY_REFERENCE_SIZE: u32 = 42;

impl Drive {
    pub(crate) fn add_estimation_costs_for_balances(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
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

        // we then need to insert the contract layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(balance_path_vec()),
            EstimatedLayerInformation {
                is_sum_tree: true,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, AVERAGE_BALANCE_SIZE, None),
            },
        );
    }

    pub(crate) fn add_estimation_costs_for_update_revision(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we need to add the root
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(0, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // we then need to insert the root identity layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(identity_tree_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // In this layer we have
        //                   Keys
        //              /         \
        // Contract Info         Revision
        //       /                   /
        //  Negative Credit      Query Keys

        // we then need to insert the identity layer for fee refunds
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, None, 1)),
                    items_size: Some((1, 8, None, 1)),
                    references_size: None,
                },
            },
        );
    }

    pub(crate) fn add_estimation_costs_for_negative_credit(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we then need to insert the root identity layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(identity_tree_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // In this layer we have
        //                   Keys
        //              /         \
        // Contract Info         Revision
        //       /                   /
        //  Negative Credit      Query Keys

        // we then need to insert the identity layer for fee refunds
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(2, false),
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, None, 2)),
                    items_size: Some((1, 8, None, 1)),
                    references_size: None,
                },
            },
        );
    }

    pub(crate) fn add_estimation_costs_for_keys_for_identity_id(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 2, updating will mean we will update 1 sum tree
        // and 1 normal tree, hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // we then need to insert the root identity layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(identity_tree_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // we then need to insert the identity layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // we then need to insert the identity keys layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_key_tree_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(50), //we can estimate that an identity will have amount 50 keys
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: AllItems(1, 42, Some(3)),
            },
        );
    }

    pub(crate) fn add_estimation_costs_for_root_key_reference_tree(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we then need to insert the identity keys layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_query_keys_tree_path_vec(identity_id)),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(4), //we can estimate that an identity will have amount 50 keys
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );
    }

    pub(crate) fn add_estimation_costs_for_purpose_in_key_reference_tree(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        purpose: Purpose,
    ) {
        let estimated_layer_count = match purpose {
            Purpose::AUTHENTICATION => ApproximateElements(4),
            Purpose::ENCRYPTION => {
                unreachable!()
            }
            Purpose::DECRYPTION => {
                unreachable!()
            }
            Purpose::WITHDRAW => ApproximateElements(1),
        };

        let estimated_layer_sizes = match purpose {
            Purpose::AUTHENTICATION => AllSubtrees(1, NoSumTrees, None),
            Purpose::ENCRYPTION => {
                unreachable!()
            }
            Purpose::DECRYPTION => {
                unreachable!()
            }
            Purpose::WITHDRAW => AllReference(1, KEY_REFERENCE_SIZE, None),
        };
        // we then need to insert the identity keys layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_query_keys_purpose_tree_path_vec(
                identity_id.as_slice(),
                purpose,
            )),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count, // there are
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes,
            },
        );
    }

    pub(crate) fn add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        security_level: SecurityLevel,
    ) {
        // we then need to insert the identity keys layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_query_keys_security_level_tree_path_vec(
                identity_id.as_slice(),
                security_level,
            )),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(4), //we can estimate that each security level will only have 4 keys
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: AllItems(1, KEY_REFERENCE_SIZE, None),
            },
        );
    }
}

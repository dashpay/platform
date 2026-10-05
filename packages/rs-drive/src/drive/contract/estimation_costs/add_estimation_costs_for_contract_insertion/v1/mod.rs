use crate::drive::constants::{AVERAGE_NUMBER_OF_UPDATES, ESTIMATED_AVERAGE_INDEX_NAME_SIZE};
use crate::drive::contract::estimation_costs::{
    property_name_tree_type_from_flags, TreeTypeWeights,
};
use crate::drive::contract::paths::contract_keeping_history_root_path;
use crate::drive::document::paths::contract_document_type_path;
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::Drive;
use crate::util::storage_flags::StorageFlags;

use crate::error::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;

use dpp::serialization::PlatformSerializableWithPlatformVersion;

use crate::drive::votes::paths::vote_contested_resource_active_polls_contract_document_tree_path;
use crate::util::type_constants::{DEFAULT_FLOAT_SIZE, DEFAULT_FLOAT_SIZE_U8};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel};
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    /// v1 of contract-insertion cost estimation. Differs from v0 by computing
    /// the per-doctype `EstimatedSumTrees` mix instead of unconditionally
    /// asserting `NoSumTrees`.
    ///
    /// The doctype-named subtree (the layer this loop estimates) is always
    /// itself a `NormalTree` — `tree_type: TreeType::NormalTree` is unchanged
    /// from v0. What v0 got wrong is the description of its CHILDREN:
    ///
    /// - The primary-key tree at `[0]` is a `CountTree` if
    ///   `documents_countable` is set, a `ProvableCountTree` if
    ///   `range_countable` is set (see
    ///   [`DocumentTypePrimaryKeyTreeType::primary_key_tree_type`]), or a
    ///   `NormalTree` otherwise.
    /// - A top-level index whose terminator level has `range_countable = true`
    ///   is itself created as a `ProvableCountTree` (see the matching branch
    ///   in `insert_contract_v0`).
    ///
    /// Both `CountTree` and `ProvableCountTree` map to a node with a count
    /// aggregate — `NodeType::CountNode` and `NodeType::ProvableCountNode`
    /// both report `cost() == 8` (versus `NormalNode::cost() == 0`). So
    /// counting them under grovedb's single `count_trees_weight` slot is
    /// byte-accurate for the average-case fee estimate, even though
    /// `EstimatedSumTrees` doesn't expose a separate `ProvableCountTrees`
    /// bucket.
    ///
    /// For non-countable contracts (no `documentsCountable` / no
    /// `rangeCountable` anywhere) all children are normal subtrees and this
    /// method emits exactly the same `NoSumTrees` shape v0 emits — so for the
    /// pre-v12 contract surface this is a byte-identical no-op. The fee math
    /// only diverges from v0 once a doctype opts into `documentsCountable` or
    /// `rangeCountable`, which is itself a v12+ feature.
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_contract_insertion_v1(
        contract: &DataContract,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
            contract,
            estimated_costs_only_with_layer_info,
            &platform_version.drive,
        )?;

        // we only store the owner_id storage
        let storage_flags = if contract.config().can_be_deleted() || !contract.config().readonly() {
            Some(StorageFlags::approximate_size(true, None))
        } else {
            None
        };

        let document_types_with_contested_unique_indexes =
            contract.document_types_with_contested_indexes();

        if !document_types_with_contested_unique_indexes.is_empty() {
            Self::add_estimation_costs_for_contested_document_tree_levels_up_to_contract_document_type_excluded(
                contract,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;

            for document_type_name in document_types_with_contested_unique_indexes.keys() {
                estimated_costs_only_with_layer_info.insert(
                    KeyInfoPath::from_known_path(
                        vote_contested_resource_active_polls_contract_document_tree_path(
                            contract.id_ref().as_bytes(),
                            document_type_name.as_str(),
                        ),
                    ),
                    EstimatedLayerInformation {
                        tree_type: TreeType::NormalTree,
                        estimated_layer_count: ApproximateElements(2),
                        estimated_layer_sizes: AllSubtrees(
                            ESTIMATED_AVERAGE_INDEX_NAME_SIZE,
                            NoSumTrees,
                            None,
                        ),
                    },
                );
            }
        }

        for (document_type_name, document_type) in contract.document_types() {
            // Compute the child-tree-type distribution at this doctype's
            // layer. Mirror what `insert_contract_v0` actually creates:
            //
            //   - key `[0]` (the primary-key tree) → tree type from
            //     `primary_key_tree_type()` (any of the 9 variants from
            //     NormalTree through ProvableCountProvableSumTree depending
            //     on `documents_countable` / `documents_summable` /
            //     `range_countable` / `range_summable`).
            //   - each top-level index key → tree type at the index's
            //     terminator level, derived from the terminator's flags
            //     via [`property_name_tree_type_from_flags`] below.
            //     Mirror of the dispatch in
            //     `add_indices_for_index_level_for_contract_operations`.
            //
            // Each child is tallied into its matching weight slot in the
            // `SomeSumTrees` struct. `EstimatedSumTrees::estimated_size`
            // multiplies each weight by `TreeType::*.inner_node_type().cost()`
            // to compute the average-case per-node cost.
            let document_type_ref = document_type.as_ref();
            let pk_tree_type = document_type_ref.primary_key_tree_type(platform_version)?;

            let mut tree_weights = TreeTypeWeights::default();
            tree_weights.tally(pk_tree_type);

            // One root sub-level per distinct top tree (grid-qualified keys
            // for time-range first properties), matching the trees
            // `insert_contract_v0` actually creates. The map is already
            // deduped.
            let index_structure = document_type_ref.index_structure();
            for level in index_structure.sub_levels().values() {
                // A compound index ranked at its FIRST property
                // (`rankedCountable: { at }`) makes its top level the
                // grouping tree — the Count-axis indexed tree
                // `insert_contract_v0` creates through the level-aware
                // resolver — even though no index terminates there.
                let terminator_tree_type = if level.ranked_count_grouping() {
                    TreeType::ProvableCountIndexedTree
                } else {
                    level
                        .has_index_with_type()
                        .map(property_name_tree_type_from_flags)
                        .unwrap_or(TreeType::NormalTree)
                };
                tree_weights.tally(terminator_tree_type);
            }

            let estimated_sum_trees = tree_weights.to_estimated_sum_trees();

            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_document_type_path(
                    contract.id_ref().as_bytes(),
                    document_type_name.as_str(),
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(0, true),
                    estimated_layer_sizes: AllSubtrees(
                        ESTIMATED_AVERAGE_INDEX_NAME_SIZE,
                        estimated_sum_trees,
                        storage_flags,
                    ),
                },
            );
        }

        if contract.config().keeps_history() {
            // We are dealing with a sibling reference.
            // The sibling reference serialized size is going to be the encoded time size
            // (DEFAULT_FLOAT_SIZE) plus 1 byte for reference type and 1 byte for the space of
            // the encoded time
            let reference_size = DEFAULT_FLOAT_SIZE + 2;

            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_keeping_history_root_path(
                    contract.id_ref().as_bytes(),
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(AVERAGE_NUMBER_OF_UPDATES as u32),
                    estimated_layer_sizes: Mix {
                        subtrees_size: None,
                        items_size: Some((
                            DEFAULT_FLOAT_SIZE_U8,
                            contract
                                .serialize_to_bytes_with_platform_version(platform_version)?
                                .len() as u32, //todo: fix this
                            storage_flags,
                            AVERAGE_NUMBER_OF_UPDATES,
                        )),
                        references_size: Some((1, reference_size, storage_flags, 1)),
                        items_with_sum_item_size: None,
                        references_with_sum_item_size: None,
                    },
                },
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! These tests pin the per-doctype `EstimatedSumTrees` shape v1 emits
    //! against the actual tree types `insert_contract_v0` writes to grove
    //! (see the `countable_e2e_tests` module in
    //! `packages/rs-drive/src/drive/contract/insert/insert_contract/v0/mod.rs`,
    //! which reads the primary-key tree back from grove and asserts the
    //! concrete variant). The estimation has to mirror that shape; if these
    //! tests start failing, either the on-disk creation moved or the
    //! estimation did — they need to move together.
    use super::*;
    use crate::drive::document::paths::contract_document_type_path;
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::{platform_value, Value};
    use dpp::tests::utils::generate_random_identifier_struct;
    use grovedb::EstimatedLayerSizes;
    use grovedb::EstimatedSumTrees::SomeSumTrees;

    const PROTOCOL_VERSION_V12: u32 = 12;

    fn build_contract(
        documents_countable: bool,
        range_countable_index_on_color: bool,
    ) -> DataContract {
        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let mut document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "additionalProperties": false,
        });
        if documents_countable {
            document_schema.as_map_mut().unwrap().push((
                Value::Text("documentsCountable".to_string()),
                Value::Bool(true),
            ));
        }
        if range_countable_index_on_color {
            // `rangeCountable: true` on the index puts a `ProvableCountTree`
            // both at the primary-key key `[0]` AND at the `byColor` index
            // name key (per `insert_contract_v0`'s
            // `property_name_is_range_countable_terminator` branch).
            document_schema.as_map_mut().unwrap().push((
                Value::Text("indices".to_string()),
                platform_value!([{
                    "name": "byColor",
                    "properties": [{"color": "asc"}],
                    "countable": "countable",
                    "rangeCountable": true,
                }]),
            ));
        }
        let schemas = platform_value!({ "widget": document_schema });
        factory
            .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
            .expect("create contract")
            .data_contract_owned()
    }

    /// For a plain (non-countable) contract v1 must emit `NoSumTrees` — same
    /// shape as v0. Otherwise v1 would change fees for pre-v12 contract
    /// shapes that don't even have count-tree children.
    #[test]
    fn non_countable_contract_emits_no_sum_trees_same_as_v0() {
        let pv = PlatformVersion::latest();
        let contract = build_contract(false, false);
        let mut layer_info: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
        crate::drive::Drive::add_estimation_costs_for_contract_insertion_v1(
            &contract,
            &mut layer_info,
            pv,
        )
        .expect("v1 estimation");
        let key = KeyInfoPath::from_known_path(contract_document_type_path(
            contract.id_ref().as_bytes(),
            "widget",
        ));
        let layer = layer_info.get(&key).expect("layer info for widget doctype");
        assert_eq!(
            layer.tree_type,
            TreeType::NormalTree,
            "doctype parent layer is always NormalTree"
        );
        match layer.estimated_layer_sizes {
            EstimatedLayerSizes::AllSubtrees(_, NoSumTrees, _) => {}
            other => panic!(
                "non-countable contract expected NoSumTrees, got {:?}",
                other
            ),
        }
    }

    /// `documentsCountable: true` only — primary-key tree is `CountTree`,
    /// no `rangeCountable` index, so we expect a 1:1 weight split between
    /// the count-bearing primary key tree and... no other children (no
    /// indexes declared).
    #[test]
    fn documents_countable_contract_emits_some_sum_trees_with_count_weight() {
        let pv = PlatformVersion::latest();
        let contract = build_contract(true, false);
        let mut layer_info: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
        crate::drive::Drive::add_estimation_costs_for_contract_insertion_v1(
            &contract,
            &mut layer_info,
            pv,
        )
        .expect("v1 estimation");
        let key = KeyInfoPath::from_known_path(contract_document_type_path(
            contract.id_ref().as_bytes(),
            "widget",
        ));
        let layer = layer_info.get(&key).expect("layer info for widget doctype");
        match layer.estimated_layer_sizes {
            EstimatedLayerSizes::AllSubtrees(
                _,
                SomeSumTrees {
                    count_trees_weight,
                    non_sum_trees_weight,
                    sum_trees_weight,
                    big_sum_trees_weight,
                    count_sum_trees_weight,
                    ..
                },
                _,
            ) => {
                assert_eq!(
                    count_trees_weight, 1,
                    "primary-key CountTree contributes 1 count-tree child"
                );
                assert_eq!(
                    non_sum_trees_weight, 0,
                    "no indexes declared → no non-count children"
                );
                assert_eq!(sum_trees_weight, 0);
                assert_eq!(big_sum_trees_weight, 0);
                assert_eq!(count_sum_trees_weight, 0);
            }
            other => panic!(
                "documentsCountable contract expected SomeSumTrees, got {:?}",
                other
            ),
        }
    }

    /// `documentsCountable: true` (doctype level) + `rangeCountable: true`
    /// on the `byColor` index → primary-key tree is `CountTree` (from the
    /// doctype-level countable flag; no doctype-level range so it's not
    /// the *Provable* variant) AND the `byColor` index tree is a
    /// `ProvableCountTree` (from the index's `rangeCountable: true`).
    ///
    /// The v12 refactor that took advantage of grovedb #674's
    /// finer-grained weights now tallies each variant separately:
    /// `count_trees_weight` carries the CountTree primary key,
    /// `provable_count_trees_weight` carries the ProvableCountTree
    /// index. Pre-refactor both collapsed into `count_trees_weight: 2`.
    #[test]
    fn range_countable_index_contract_counts_both_pk_and_index_as_count_children() {
        let pv = PlatformVersion::latest();
        let contract = build_contract(true, true);
        let mut layer_info: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
        crate::drive::Drive::add_estimation_costs_for_contract_insertion_v1(
            &contract,
            &mut layer_info,
            pv,
        )
        .expect("v1 estimation");
        let key = KeyInfoPath::from_known_path(contract_document_type_path(
            contract.id_ref().as_bytes(),
            "widget",
        ));
        let layer = layer_info.get(&key).expect("layer info for widget doctype");
        match layer.estimated_layer_sizes {
            EstimatedLayerSizes::AllSubtrees(
                _,
                SomeSumTrees {
                    count_trees_weight,
                    provable_count_trees_weight,
                    non_sum_trees_weight,
                    ..
                },
                _,
            ) => {
                assert_eq!(
                    count_trees_weight, 1,
                    "primary-key CountTree (from documentsCountable)"
                );
                assert_eq!(
                    provable_count_trees_weight, 1,
                    "byColor ProvableCountTree (from index rangeCountable)"
                );
                assert_eq!(non_sum_trees_weight, 0, "no non-count children");
            }
            other => panic!(
                "rangeCountable contract expected SomeSumTrees, got {:?}",
                other
            ),
        }
    }

    /// Diff vs v0: for the same `documentsCountable` contract, v0 emits
    /// `NoSumTrees` (the bug) and v1 emits `SomeSumTrees { count_trees_weight: 1, ... }`.
    /// This is the smallest-possible test that pins the behavioral divergence.
    #[test]
    fn v1_differs_from_v0_only_when_count_children_present() {
        let pv = PlatformVersion::latest();

        // Non-countable: v0 and v1 must agree (byte-identical NoSumTrees).
        let plain = build_contract(false, false);
        let mut v0_layer: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
        let mut v1_layer: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
        crate::drive::Drive::add_estimation_costs_for_contract_insertion_v0(
            &plain,
            &mut v0_layer,
            pv,
        )
        .expect("v0");
        crate::drive::Drive::add_estimation_costs_for_contract_insertion_v1(
            &plain,
            &mut v1_layer,
            pv,
        )
        .expect("v1");
        let key = KeyInfoPath::from_known_path(contract_document_type_path(
            plain.id_ref().as_bytes(),
            "widget",
        ));
        assert_eq!(
            v0_layer.get(&key).map(|l| l.estimated_layer_sizes),
            v1_layer.get(&key).map(|l| l.estimated_layer_sizes),
            "v0 and v1 must produce the same shape for non-countable contracts"
        );

        // Countable: v0 still says NoSumTrees (the bug); v1 says
        // SomeSumTrees. Diverging on this case is the whole point of v1.
        let countable = build_contract(true, false);
        let mut v0_layer: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
        let mut v1_layer: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
        crate::drive::Drive::add_estimation_costs_for_contract_insertion_v0(
            &countable,
            &mut v0_layer,
            pv,
        )
        .expect("v0");
        crate::drive::Drive::add_estimation_costs_for_contract_insertion_v1(
            &countable,
            &mut v1_layer,
            pv,
        )
        .expect("v1");
        let key = KeyInfoPath::from_known_path(contract_document_type_path(
            countable.id_ref().as_bytes(),
            "widget",
        ));
        let v0_sizes = v0_layer.get(&key).unwrap().estimated_layer_sizes;
        let v1_sizes = v1_layer.get(&key).unwrap().estimated_layer_sizes;
        assert!(
            matches!(v0_sizes, EstimatedLayerSizes::AllSubtrees(_, NoSumTrees, _)),
            "v0 emits NoSumTrees (under-bills count-tree children)"
        );
        assert!(
            !matches!(v1_sizes, EstimatedLayerSizes::AllSubtrees(_, NoSumTrees, _)),
            "v1 must NOT emit NoSumTrees for countable contracts — got {:?}",
            v1_sizes
        );
    }
}

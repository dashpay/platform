//! Layer descriptions that several writes of one batch register for the same layer.
//!
//! A batch's cost estimation keeps one description per layer, and a layer's description gives
//! the level the written key sits on, which sets how many ancestors an update propagates
//! through. When two writes of one batch reach keys at different depths of the same layer, the
//! batch must be priced at the deeper one, whichever write registers its description first.

use crate::error::Error;
use crate::fees::op::DriveCost;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{
    EstimatedLayerCount, EstimatedLayerInformation, EstimatedLayerSizes, EstimatedSumTrees,
    TreeType,
};
use grovedb_merk::estimated_costs::average_case_costs::average_case_merk_propagate;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Describes the layer at `path` as `layer`, unless the batch already describes it at least as
/// deep (`estimate_levels`, which gives a description by element count a depth too: one that may
/// hold the most elements a layer can is 32 levels deep, the worst case).
pub(crate) fn insert_estimated_layer_keeping_deeper(
    estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    path: KeyInfoPath,
    layer: EstimatedLayerInformation,
) {
    if let Some(existing) = estimated_costs_only_with_layer_info.get(&path) {
        if existing.estimated_layer_count.estimate_levels()
            >= layer.estimated_layer_count.estimate_levels()
        {
            return;
        }
    }
    estimated_costs_only_with_layer_info.insert(path, layer);
}

/// What GroveDB's own estimator charges to carry a change through a layer described as `layer`
/// (`average_case_merk_propagate`: the nodes it rewrites, the bytes it loads and replaces, the
/// hashes), in credits at `platform_version`'s fees. Drive keeps no cost model of its own for a
/// description: GroveDB prices it, and Drive only compares the prices.
fn propagation_credits(
    layer: &EstimatedLayerInformation,
    platform_version: &PlatformVersion,
) -> Result<Credits, Error> {
    let propagation = average_case_merk_propagate(layer, &platform_version.drive.grove_version);
    propagation
        .value
        .map_err(|error| Error::GroveDB(Box::new(error.into())))?;
    propagation
        .cost
        .ephemeral_cost(&platform_version.fee_version)
}

/// The description a batch keeps for a layer two of its operations describe as `a` and `b`. GroveDB
/// gives every operation at the layer's path this one description, so it must cover each of
/// them: it is as deep as the deeper (`estimate_levels`, which gives a description by element
/// count a depth too), as the depth sets how many nodes every use of the description touches,
/// and at that depth it has the nodes of whichever GroveDB prices dearer
/// ([`propagation_credits`]), the shallower one being priced at the deeper one's depth. Two
/// priced alike are told apart by their contents ([`layer_key`]), so the result is the same
/// whichever is seen first.
fn costlier_layer(
    a: &EstimatedLayerInformation,
    b: &EstimatedLayerInformation,
    platform_version: &PlatformVersion,
) -> Result<EstimatedLayerInformation, Error> {
    // The operations of one batch mostly describe a shared layer alike: nothing to price.
    if a == b {
        return Ok(*a);
    }
    let at_depth_of = |layer: &EstimatedLayerInformation, deeper: &EstimatedLayerInformation| {
        EstimatedLayerInformation {
            estimated_layer_count: deeper.estimated_layer_count,
            ..*layer
        }
    };
    let (a, b) = match a
        .estimated_layer_count
        .estimate_levels()
        .cmp(&b.estimated_layer_count.estimate_levels())
    {
        Ordering::Greater => (*a, at_depth_of(b, a)),
        Ordering::Less => (at_depth_of(a, b), *b),
        Ordering::Equal => (*a, *b),
    };
    if a == b {
        return Ok(a);
    }
    let by_price =
        propagation_credits(&a, platform_version)?.cmp(&propagation_credits(&b, platform_version)?);
    let order = match by_price {
        Ordering::Equal => layer_key(&a).cmp(&layer_key(&b)),
        by_price => by_price,
    };
    Ok(if order == Ordering::Less { b } else { a })
}

/// A total order on descriptions, for two GroveDB prices alike: every field in turn, each enum
/// by the position of its variant. The matches name every variant, so a description GroveDB adds
/// cannot fall into an ordering by accident; and unlike a formatted string, the key does not
/// change with a dependency's `Debug` output.
fn layer_key(layer: &EstimatedLayerInformation) -> Vec<u64> {
    let mut key = vec![];
    tree_type_key(layer.tree_type, &mut key);
    match layer.estimated_layer_count {
        EstimatedLayerCount::PotentiallyAtMaxElements => key.push(0),
        EstimatedLayerCount::ApproximateElements(count) => key.extend([1, count as u64]),
        EstimatedLayerCount::EstimatedLevel(level, to_be_empty) => {
            key.extend([2, level as u64, to_be_empty as u64])
        }
    }
    let flags = |flags: Option<u32>| flags.map_or(0, |flags| flags as u64 + 1);
    let sized = |key: &mut Vec<u64>, entry: Option<(u8, u32, Option<u32>, u8)>| match entry {
        None => key.push(0),
        Some((key_size, value_size, flags_size, weight)) => key.extend([
            1,
            key_size as u64,
            value_size as u64,
            flags(flags_size),
            weight as u64,
        ]),
    };
    match layer.estimated_layer_sizes {
        EstimatedLayerSizes::AllSubtrees(key_size, sum_trees, flags_size) => {
            key.extend([0, key_size as u64]);
            sum_trees_key(sum_trees, &mut key);
            key.push(flags(flags_size));
        }
        EstimatedLayerSizes::AllItems(key_size, value_size, flags_size) => {
            key.extend([1, key_size as u64, value_size as u64, flags(flags_size)])
        }
        EstimatedLayerSizes::AllReference(key_size, value_size, flags_size) => {
            key.extend([2, key_size as u64, value_size as u64, flags(flags_size)])
        }
        EstimatedLayerSizes::AllItemsWithSumItem(key_size, value_size, flags_size) => {
            key.extend([3, key_size as u64, value_size as u64, flags(flags_size)])
        }
        EstimatedLayerSizes::AllReferencesWithSumItem(key_size, value_size, flags_size) => {
            key.extend([4, key_size as u64, value_size as u64, flags(flags_size)])
        }
        EstimatedLayerSizes::Mix {
            subtrees_size,
            items_size,
            references_size,
            items_with_sum_item_size,
            references_with_sum_item_size,
        } => {
            key.push(5);
            match subtrees_size {
                None => key.push(0),
                Some((key_size, sum_trees, flags_size, weight)) => {
                    key.extend([1, key_size as u64]);
                    sum_trees_key(sum_trees, &mut key);
                    key.extend([flags(flags_size), weight as u64]);
                }
            }
            sized(&mut key, items_size);
            sized(&mut key, references_size);
            sized(&mut key, items_with_sum_item_size);
            sized(&mut key, references_with_sum_item_size);
        }
    }
    key
}

fn tree_type_key(tree_type: TreeType, key: &mut Vec<u64>) {
    let (variant, parameter) = match tree_type {
        TreeType::NormalTree => (0, 0),
        TreeType::SumTree => (1, 0),
        TreeType::BigSumTree => (2, 0),
        TreeType::CountTree => (3, 0),
        TreeType::CountSumTree => (4, 0),
        TreeType::ProvableCountTree => (5, 0),
        TreeType::ProvableCountSumTree => (6, 0),
        TreeType::CommitmentTree(chunk_power) => (7, chunk_power),
        TreeType::MmrTree => (8, 0),
        TreeType::BulkAppendTree(chunk_power) => (9, chunk_power),
        TreeType::DenseAppendOnlyFixedSizeTree(height) => (10, height),
        TreeType::ProvableSumTree => (11, 0),
        TreeType::ProvableCountProvableSumTree => (12, 0),
        TreeType::ProvableSumIndexedTree => (13, 0),
        TreeType::ProvableCountIndexedTree => (14, 0),
        TreeType::ProvableCountProvableSumIndexedTree => (15, 0),
        TreeType::PrivateDocumentStore(parameter) => (16, parameter),
    };
    key.extend([variant, parameter as u64]);
}

fn sum_trees_key(sum_trees: EstimatedSumTrees, key: &mut Vec<u64>) {
    match sum_trees {
        EstimatedSumTrees::NoSumTrees => key.push(0),
        EstimatedSumTrees::SomeSumTrees {
            sum_trees_weight,
            big_sum_trees_weight,
            count_trees_weight,
            count_sum_trees_weight,
            non_sum_trees_weight,
            provable_sum_trees_weight,
            provable_count_trees_weight,
            provable_count_sum_trees_weight,
            provable_count_provable_sum_trees_weight,
        } => key.extend([
            1,
            sum_trees_weight as u64,
            big_sum_trees_weight as u64,
            count_trees_weight as u64,
            count_sum_trees_weight as u64,
            non_sum_trees_weight as u64,
            provable_sum_trees_weight as u64,
            provable_count_trees_weight as u64,
            provable_count_sum_trees_weight as u64,
            provable_count_provable_sum_trees_weight as u64,
        ]),
        EstimatedSumTrees::AllSumTrees => key.push(2),
        EstimatedSumTrees::AllBigSumTrees => key.push(3),
        EstimatedSumTrees::AllCountTrees => key.push(4),
        EstimatedSumTrees::AllCountSumTrees => key.push(5),
        EstimatedSumTrees::AllProvableSumTrees => key.push(6),
        EstimatedSumTrees::AllProvableCountTrees => key.push(7),
        EstimatedSumTrees::AllProvableCountSumTrees => key.push(8),
        EstimatedSumTrees::AllProvableCountProvableSumTrees => key.push(9),
    }
}

/// Merges `layer` into the batch's description of the layer at `path` ([`costlier_layer`]): the
/// depth of the deeper of the two and, at that depth, the nodes GroveDB prices dearer. The order
/// two operations of a batch register their descriptions of a shared layer in no longer matters,
/// and the description priced for each of them is never cheaper than its own.
pub(crate) fn merge_estimated_layer(
    estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    path: KeyInfoPath,
    layer: EstimatedLayerInformation,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let merged = match estimated_costs_only_with_layer_info.get(&path) {
        Some(existing) => costlier_layer(existing, &layer, platform_version)?,
        None => layer,
    };
    estimated_costs_only_with_layer_info.insert(path, merged);
    Ok(())
}

/// Runs `register`, an estimation that overwrites the descriptions of the layers at `paths`, and
/// puts back each description it replaced that was deeper.
pub(crate) fn register_keeping_deeper_estimated_layers(
    estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    paths: &[KeyInfoPath],
    register: impl FnOnce(&mut HashMap<KeyInfoPath, EstimatedLayerInformation>),
) {
    let before: Vec<(KeyInfoPath, EstimatedLayerInformation)> = paths
        .iter()
        .filter_map(|path| {
            estimated_costs_only_with_layer_info
                .get(path)
                .map(|layer| (path.clone(), *layer))
        })
        .collect();
    register(estimated_costs_only_with_layer_info);
    for (path, layer) in before {
        insert_estimated_layer_keeping_deeper(estimated_costs_only_with_layer_info, path, layer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb::EstimatedLayerCount::{
        ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements,
    };
    use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees};
    use grovedb::EstimatedSumTrees::{AllBigSumTrees, NoSumTrees};
    use grovedb::TreeType;

    fn layer(level: u32) -> EstimatedLayerInformation {
        EstimatedLayerInformation {
            tree_type: TreeType::NormalTree,
            estimated_layer_count: EstimatedLevel(level, false),
            estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
        }
    }

    fn level_at(
        estimation: &HashMap<KeyInfoPath, EstimatedLayerInformation>,
        path: &KeyInfoPath,
    ) -> Option<u32> {
        estimation
            .get(path)
            .and_then(|layer| match layer.estimated_layer_count {
                EstimatedLevel(level, _) => Some(level),
                _ => None,
            })
    }

    #[test]
    fn should_insert_unless_a_deeper_or_equal_level_is_described() {
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let mut estimation = HashMap::new();
        insert_estimated_layer_keeping_deeper(&mut estimation, path.clone(), layer(1));
        assert_eq!(level_at(&estimation, &path), Some(1));
        insert_estimated_layer_keeping_deeper(&mut estimation, path.clone(), layer(2));
        assert_eq!(level_at(&estimation, &path), Some(2));
        insert_estimated_layer_keeping_deeper(&mut estimation, path.clone(), layer(1));
        assert_eq!(level_at(&estimation, &path), Some(2));
        // A description by element count has a depth too: one that may hold the most elements
        // a layer can (32 levels, the worst case) is kept over a shallower level, and one of a
        // few elements gives way to a deeper level.
        let at_max = EstimatedLayerInformation {
            estimated_layer_count: PotentiallyAtMaxElements,
            ..layer(0)
        };
        estimation.insert(path.clone(), at_max);
        insert_estimated_layer_keeping_deeper(&mut estimation, path.clone(), layer(5));
        assert_eq!(estimation.get(&path), Some(&at_max));
        estimation.insert(
            path.clone(),
            EstimatedLayerInformation {
                estimated_layer_count: ApproximateElements(3),
                ..layer(0)
            },
        );
        insert_estimated_layer_keeping_deeper(&mut estimation, path.clone(), layer(5));
        assert_eq!(level_at(&estimation, &path), Some(5));
    }

    /// Two descriptions of one layer merge to the same one in either order: the deeper depth, the
    /// nodes GroveDB's propagation estimate prices dearer at that depth, then, for two it prices
    /// alike, a fixed choice.
    #[test]
    fn should_merge_two_descriptions_of_a_layer_alike_in_either_order() {
        let platform_version = PlatformVersion::latest();
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let at_depth_3 = |estimated_layer_sizes| EstimatedLayerInformation {
            estimated_layer_sizes,
            ..layer(3)
        };
        let merged = |first: EstimatedLayerInformation, second: EstimatedLayerInformation| {
            let mut estimation = HashMap::new();
            merge_estimated_layer(&mut estimation, path.clone(), first, platform_version)
                .expect("expected to merge");
            merge_estimated_layer(&mut estimation, path.clone(), second, platform_version)
                .expect("expected to merge");
            estimation[&path]
        };
        let credits = |layer: &EstimatedLayerInformation| {
            propagation_credits(layer, platform_version).expect("expected a price")
        };

        // Items of 8 bytes under 32-byte keys cost more to carry a change through than big sum
        // subtrees under the same keys, though a subtree's element is the larger: GroveDB
        // counts the item's two stored hashes.
        let items = at_depth_3(AllItems(32, 8, None));
        let subtrees = at_depth_3(AllSubtrees(32, AllBigSumTrees, None));
        assert!(credits(&items) > credits(&subtrees));
        assert_eq!(merged(items, subtrees), items);
        assert_eq!(merged(subtrees, items), items);

        // A deeper description of narrower nodes, priced below the shallower one: the merge
        // keeps the deeper depth and the dearer nodes, so neither operation is priced below its
        // own description.
        let deeper_narrow = EstimatedLayerInformation {
            estimated_layer_count: EstimatedLevel(4, false),
            ..at_depth_3(AllSubtrees(1, NoSumTrees, None))
        };
        assert!(credits(&deeper_narrow) < credits(&items));
        let deeper_items = EstimatedLayerInformation {
            estimated_layer_count: EstimatedLevel(4, false),
            ..items
        };
        assert_eq!(merged(items, deeper_narrow), deeper_items);
        assert_eq!(merged(deeper_narrow, items), deeper_items);
        assert!(credits(&deeper_items) >= credits(&items));
        assert!(credits(&deeper_items) >= credits(&deeper_narrow));

        // The deeper one's nodes are kept when they are the dearer at its depth.
        let deeper_wide = EstimatedLayerInformation {
            estimated_layer_count: EstimatedLevel(4, false),
            ..at_depth_3(AllItems(32, 64, None))
        };
        assert_eq!(merged(items, deeper_wide), deeper_wide);
        assert_eq!(merged(deeper_wide, items), deeper_wide);

        // Two different descriptions GroveDB prices alike.
        let references = at_depth_3(AllReference(32, 8, None));
        assert_ne!(items, references);
        assert_eq!(credits(&items), credits(&references));
        assert_eq!(merged(items, references), merged(references, items));
    }

    #[test]
    fn should_put_back_a_deeper_level_an_estimation_overwrote() {
        let deep = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let shallow = KeyInfoPath::from_known_path([[2u8].as_slice()]);
        let mut estimation = HashMap::from([(deep.clone(), layer(2)), (shallow.clone(), layer(0))]);
        register_keeping_deeper_estimated_layers(
            &mut estimation,
            &[deep.clone(), shallow.clone()],
            |estimation| {
                estimation.insert(deep.clone(), layer(1));
                estimation.insert(shallow.clone(), layer(1));
            },
        );
        assert_eq!(level_at(&estimation, &deep), Some(2));
        assert_eq!(level_at(&estimation, &shallow), Some(1));
    }
}

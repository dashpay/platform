//! Layer descriptions that several writes of one batch register for the same layer.
//!
//! A batch's cost estimation keeps one description per layer, and a layer's description gives
//! the level the written key sits on, which sets how many ancestors an update propagates
//! through. When two writes of one batch reach keys at different depths of the same layer, the
//! batch must be priced at the deeper one, whichever write registers its description first.
//! Which of two descriptions GroveDB prices dearer can change with the depth they are priced
//! at, so a batch keeps every description its writes give a layer until all of them are known,
//! and only then picks one.

use crate::error::drive::DriveError;
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

/// The description a batch keeps for a layer its operations describe as `candidates`. GroveDB
/// gives every operation at the layer's path this one description, so it must cover each of
/// them: it is as deep as the deepest (`estimate_levels`, which gives a description by element
/// count a depth too), as the depth sets how many nodes every use of the description touches,
/// and at that depth it has the nodes GroveDB prices dearest ([`propagation_credits`]), each
/// candidate being priced at every count that reaches that depth. Two priced alike are told
/// apart by their contents ([`layer_key`]). The pick is a maximum over the whole set, so it is
/// the same whatever order the candidates were gathered in; picking between two at a time
/// would not be, as a description priced alike or cheaper at a shallow depth can be the
/// dearer at a deeper one.
fn costliest_layer(
    candidates: &[EstimatedLayerInformation],
    platform_version: &PlatformVersion,
) -> Result<EstimatedLayerInformation, Error> {
    let Some(first) = candidates.first() else {
        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
            "a layer is described by at least one candidate",
        )));
    };
    // The operations of one batch mostly describe a shared layer alike: nothing to price.
    if candidates.iter().all(|candidate| candidate == first) {
        return Ok(*first);
    }
    let deepest = candidates
        .iter()
        .map(|candidate| candidate.estimated_layer_count.estimate_levels())
        .fold(0, u32::max);
    let mut deepest_counts: Vec<EstimatedLayerCount> = vec![];
    for candidate in candidates {
        let count = candidate.estimated_layer_count;
        if count.estimate_levels() == deepest && !deepest_counts.contains(&count) {
            deepest_counts.push(count);
        }
    }
    let mut costliest: Option<(Credits, Vec<u64>, EstimatedLayerInformation)> = None;
    for candidate in candidates {
        for count in &deepest_counts {
            let at_depth = EstimatedLayerInformation {
                estimated_layer_count: *count,
                ..*candidate
            };
            let price = propagation_credits(&at_depth, platform_version)?;
            let key = layer_key(&at_depth);
            let dearer = match &costliest {
                None => true,
                Some((costliest_price, costliest_key, _)) => {
                    (price, &key) > (*costliest_price, costliest_key)
                }
            };
            if dearer {
                costliest = Some((price, key, at_depth));
            }
        }
    }
    Ok(costliest.map_or(*first, |(_, _, layer)| layer))
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

/// Every description the operations of a batch give each layer, kept until the batch is lowered.
pub(crate) type LayerCandidates = HashMap<KeyInfoPath, Vec<EstimatedLayerInformation>>;

/// Adds the descriptions one operation of a batch gives its layers to the batch's candidates,
/// each description once.
pub(crate) fn add_estimated_layer_candidates(
    candidates: &mut LayerCandidates,
    operation_layers: HashMap<KeyInfoPath, EstimatedLayerInformation>,
) {
    for (path, layer) in operation_layers {
        let described = candidates.entry(path).or_default();
        if !described.contains(&layer) {
            described.push(layer);
        }
    }
}

/// The description a batch keeps for each layer its operations describe: the costliest of the
/// layer's candidates ([`costliest_layer`]). The order two operations of a batch register their
/// descriptions of a shared layer in does not matter, and the description priced for each of
/// them is never cheaper than its own.
pub(crate) fn costliest_estimated_layers(
    candidates: LayerCandidates,
    platform_version: &PlatformVersion,
) -> Result<HashMap<KeyInfoPath, EstimatedLayerInformation>, Error> {
    candidates
        .into_iter()
        .map(|(path, layers)| Ok((path, costliest_layer(&layers, platform_version)?)))
        .collect()
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
    use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees, Mix};
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

    /// Every order of `layers`, each from an operation of its own.
    fn orders(layers: &[EstimatedLayerInformation]) -> Vec<Vec<EstimatedLayerInformation>> {
        if layers.len() <= 1 {
            return vec![layers.to_vec()];
        }
        let mut all = vec![];
        for (index, layer) in layers.iter().enumerate() {
            let mut rest = layers.to_vec();
            rest.remove(index);
            for mut order in orders(&rest) {
                order.insert(0, *layer);
                all.push(order);
            }
        }
        all
    }

    /// The description a batch keeps when its operations describe one layer as `layers`, in
    /// that order, each from an operation of its own.
    fn kept(layers: &[EstimatedLayerInformation]) -> EstimatedLayerInformation {
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let mut candidates = LayerCandidates::new();
        for layer in layers {
            add_estimated_layer_candidates(
                &mut candidates,
                HashMap::from([(path.clone(), *layer)]),
            );
        }
        costliest_estimated_layers(candidates, PlatformVersion::latest())
            .expect("expected to pick a description")[&path]
    }

    /// The description `layers` give a layer, the same in every order they come in.
    fn kept_in_every_order(layers: &[EstimatedLayerInformation]) -> EstimatedLayerInformation {
        let kept_first = kept(layers);
        for order in orders(layers) {
            assert_eq!(kept(&order), kept_first, "order {order:?}");
        }
        kept_first
    }

    fn credits(layer: &EstimatedLayerInformation) -> Credits {
        propagation_credits(layer, PlatformVersion::latest()).expect("expected a price")
    }

    fn at_level(
        level: u32,
        estimated_layer_sizes: EstimatedLayerSizes,
    ) -> EstimatedLayerInformation {
        EstimatedLayerInformation {
            estimated_layer_count: EstimatedLevel(level, false),
            estimated_layer_sizes,
            ..layer(level)
        }
    }

    /// The deepest depth, the nodes GroveDB's propagation estimate prices dearest at that depth,
    /// then, for two it prices alike, a fixed choice; the same in every order.
    #[test]
    fn should_keep_the_costliest_description_of_a_layer_in_every_order() {
        // Items of 8 bytes under 32-byte keys cost more to carry a change through than big sum
        // subtrees under the same keys, though a subtree's element is the larger: GroveDB
        // counts the item's two stored hashes.
        let items = at_level(3, AllItems(32, 8, None));
        let subtrees = at_level(3, AllSubtrees(32, AllBigSumTrees, None));
        assert!(credits(&items) > credits(&subtrees));
        assert_eq!(kept_in_every_order(&[items, subtrees]), items);

        // A deeper description of narrower nodes, priced below the shallower one: the batch
        // keeps the deeper depth and the dearer nodes, so neither operation is priced below its
        // own description.
        let deeper_narrow = at_level(4, AllSubtrees(1, NoSumTrees, None));
        assert!(credits(&deeper_narrow) < credits(&items));
        let deeper_items = at_level(4, AllItems(32, 8, None));
        assert_eq!(kept_in_every_order(&[items, deeper_narrow]), deeper_items);
        assert!(credits(&deeper_items) >= credits(&items));
        assert!(credits(&deeper_items) >= credits(&deeper_narrow));

        // The deeper one's nodes are kept when they are the dearer at its depth.
        let deeper_wide = at_level(4, AllItems(32, 64, None));
        assert_eq!(kept_in_every_order(&[items, deeper_wide]), deeper_wide);

        // Two different descriptions GroveDB prices alike.
        let references = at_level(3, AllReference(32, 8, None));
        assert_ne!(items, references);
        assert_eq!(credits(&items), credits(&references));
        kept_in_every_order(&[items, references]);
    }

    /// At level 0 GroveDB prices every description alike, so a choice between two level 0
    /// descriptions made before a deeper one arrives would rest on the tie-break, and could keep
    /// the nodes that are the cheaper at the deeper depth. The batch keeps all three until the
    /// deepest depth is known, then the nodes dearest there, in every order.
    #[test]
    fn should_pick_among_shallow_descriptions_at_the_deepest_depth() {
        let items = at_level(0, AllItems(32, 8, None));
        let mix = at_level(
            0,
            Mix {
                subtrees_size: Some((32, AllBigSumTrees, None, 1)),
                items_size: None,
                references_size: None,
                items_with_sum_item_size: None,
                references_with_sum_item_size: None,
            },
        );
        let deep_narrow = at_level(4, AllSubtrees(1, NoSumTrees, None));
        assert_eq!(credits(&items), credits(&mix));
        let items_at_4 = at_level(4, AllItems(32, 8, None));
        let mix_at_4 = EstimatedLayerInformation {
            estimated_layer_count: EstimatedLevel(4, false),
            ..mix
        };
        assert!(credits(&items_at_4) > credits(&mix_at_4));
        assert!(credits(&items_at_4) > credits(&deep_narrow));
        assert_eq!(kept_in_every_order(&[items, mix, deep_narrow]), items_at_4);
    }

    /// Two descriptions of one depth with different counts: each candidate is priced at each
    /// count, and the batch keeps the dearest, in either order.
    #[test]
    fn should_price_every_candidate_at_every_count_of_the_deepest_depth() {
        let items = at_level(2, AllItems(32, 8, None));
        let by_count = EstimatedLayerInformation {
            estimated_layer_count: ApproximateElements(3),
            ..at_level(2, AllSubtrees(1, NoSumTrees, None))
        };
        assert_eq!(by_count.estimated_layer_count.estimate_levels(), 2);
        let kept = kept_in_every_order(&[items, by_count]);
        assert_eq!(kept.estimated_layer_sizes, AllItems(32, 8, None));
        assert!(credits(&kept) >= credits(&items));
        assert!(credits(&kept) >= credits(&by_count));
    }

    /// A layer every operation describes alike is kept as described, without pricing.
    #[test]
    fn should_keep_a_layer_described_alike_as_it_is() {
        let items = at_level(3, AllItems(32, 8, None));
        assert_eq!(kept(&[items, items, items]), items);
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

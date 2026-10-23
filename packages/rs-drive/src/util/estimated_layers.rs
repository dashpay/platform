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
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerInformation;
use grovedb_merk::estimated_costs::average_case_costs::average_case_merk_propagate;
use std::cmp::Ordering;
use std::collections::HashMap;

/// The level a description gives its layer, `None` for one described by its element count.
fn estimated_level(layer: &EstimatedLayerInformation) -> Option<u32> {
    match layer.estimated_layer_count {
        EstimatedLevel(level, _) => Some(level),
        _ => None,
    }
}

/// Describes the layer at `path` as `layer`, unless the batch already describes it at a level at
/// least as deep.
pub(crate) fn insert_estimated_layer_keeping_deeper(
    estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    path: KeyInfoPath,
    layer: EstimatedLayerInformation,
) {
    if let Some(existing) = estimated_costs_only_with_layer_info.get(&path) {
        if let (Some(existing_level), Some(level)) =
            (estimated_level(existing), estimated_level(&layer))
        {
            if existing_level >= level {
                return;
            }
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

/// Orders two descriptions of one layer: the deeper first (`estimate_levels`, which gives a
/// description by element count a depth too), as the depth sets how many nodes every use of the
/// description touches; then at one depth the one GroveDB prices dearer
/// ([`propagation_credits`]); then, for two priced alike, by their contents, so the order is
/// total and the same whichever is seen first.
fn compare_layer_cost(
    a: &EstimatedLayerInformation,
    b: &EstimatedLayerInformation,
    platform_version: &PlatformVersion,
) -> Result<Ordering, Error> {
    // The operations of one batch mostly describe a shared layer alike: nothing to price.
    if a == b {
        return Ok(Ordering::Equal);
    }
    let by_depth = a
        .estimated_layer_count
        .estimate_levels()
        .cmp(&b.estimated_layer_count.estimate_levels());
    if by_depth != Ordering::Equal {
        return Ok(by_depth);
    }
    // Priced only when the depths tie, so a pricing error of a description the depth already
    // ranks cannot fail the comparison.
    let by_price =
        propagation_credits(a, platform_version)?.cmp(&propagation_credits(b, platform_version)?);
    if by_price != Ordering::Equal {
        return Ok(by_price);
    }
    Ok(format!("{a:?}").cmp(&format!("{b:?}")))
}

/// Merges `layer` into the batch's description of the layer at `path`, keeping the costlier of
/// the two ([`compare_layer_cost`]). The order two operations of a batch register their
/// descriptions of a shared layer in no longer matters: the batch's estimate keeps the deeper,
/// and of two at one depth the one GroveDB prices dearer.
pub(crate) fn merge_estimated_layer(
    estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    path: KeyInfoPath,
    layer: EstimatedLayerInformation,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    if let Some(existing) = estimated_costs_only_with_layer_info.get(&path) {
        if compare_layer_cost(existing, &layer, platform_version)? != Ordering::Less {
            return Ok(());
        }
    }
    estimated_costs_only_with_layer_info.insert(path, layer);
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
    use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
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
        estimation.get(path).and_then(estimated_level)
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
        // A description by element count is replaced as before.
        estimation.insert(
            path.clone(),
            EstimatedLayerInformation {
                estimated_layer_count: PotentiallyAtMaxElements,
                ..layer(0)
            },
        );
        insert_estimated_layer_keeping_deeper(&mut estimation, path.clone(), layer(1));
        assert_eq!(level_at(&estimation, &path), Some(1));
    }

    /// Two descriptions of one layer merge to the same one in either order: the deeper, then at
    /// one depth the one GroveDB's propagation estimate prices dearer, then, for two it prices
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

        // A deeper description wins whatever its nodes.
        let deeper_narrow = EstimatedLayerInformation {
            estimated_layer_count: EstimatedLevel(4, false),
            ..at_depth_3(AllSubtrees(1, NoSumTrees, None))
        };
        assert!(credits(&deeper_narrow) < credits(&items));
        assert_eq!(merged(items, deeper_narrow), deeper_narrow);
        assert_eq!(merged(deeper_narrow, items), deeper_narrow);

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

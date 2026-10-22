//! Layer descriptions that several writes of one batch register for the same layer.
//!
//! A batch's cost estimation keeps one description per layer, and a layer's description gives
//! the level the written key sits on, which sets how many ancestors an update propagates
//! through. When two writes of one batch reach keys at different depths of the same layer, the
//! batch must be priced at the deeper one, whichever write registers its description first.

use crate::error::Error;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes;
use grovedb_version::version::GroveVersion;
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

/// The bytes the estimation prices one node of a layer described as `layer` with: its average key
/// and its value with feature and flags, the larger of each part of a mixed layer.
fn node_bytes(
    layer: &EstimatedLayerInformation,
    grove_version: &GroveVersion,
) -> Result<u32, Error> {
    let key = match &layer.estimated_layer_sizes {
        EstimatedLayerSizes::AllSubtrees(key, _, _)
        | EstimatedLayerSizes::AllItems(key, _, _)
        | EstimatedLayerSizes::AllReference(key, _, _)
        | EstimatedLayerSizes::AllItemsWithSumItem(key, _, _)
        | EstimatedLayerSizes::AllReferencesWithSumItem(key, _, _) => *key,
        EstimatedLayerSizes::Mix {
            subtrees_size,
            items_size,
            references_size,
            items_with_sum_item_size,
            references_with_sum_item_size,
        } => [
            subtrees_size.map(|(key, ..)| key),
            items_size.map(|(key, ..)| key),
            references_size.map(|(key, ..)| key),
            items_with_sum_item_size.map(|(key, ..)| key),
            references_with_sum_item_size.map(|(key, ..)| key),
        ]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or_default(),
    };
    let value = layer
        .estimated_layer_sizes
        .value_with_feature_and_flags_size(grove_version)
        .map_err(|error| Error::GroveDB(Box::new(error.into())))?;
    Ok(u32::from(key) + value)
}

/// Orders two descriptions of one layer by what they price an update through it at: the
/// deeper first (`estimate_levels`, which gives a description by element count a depth too),
/// then at one depth the larger node, then, for two that price alike, by their contents, so
/// the order is total and the same whichever is seen first.
fn compare_layer_cost(
    a: &EstimatedLayerInformation,
    b: &EstimatedLayerInformation,
    grove_version: &GroveVersion,
) -> Result<Ordering, Error> {
    Ok(a.estimated_layer_count
        .estimate_levels()
        .cmp(&b.estimated_layer_count.estimate_levels())
        .then(node_bytes(a, grove_version)?.cmp(&node_bytes(b, grove_version)?))
        .then_with(|| format!("{a:?}").cmp(&format!("{b:?}"))))
}

/// Merges `layer` into the batch's description of the layer at `path`, keeping the costlier of
/// the two ([`compare_layer_cost`]). The order two operations of a batch register their
/// descriptions of a shared layer in no longer matters: the batch's estimate keeps the deeper,
/// and of two at one depth the one pricing a larger node.
pub(crate) fn merge_estimated_layer(
    estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    path: KeyInfoPath,
    layer: EstimatedLayerInformation,
    grove_version: &GroveVersion,
) -> Result<(), Error> {
    if let Some(existing) = estimated_costs_only_with_layer_info.get(&path) {
        if compare_layer_cost(existing, &layer, grove_version)? != Ordering::Less {
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
    use grovedb::EstimatedLayerSizes::AllSubtrees;
    use grovedb::EstimatedSumTrees::NoSumTrees;
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

    /// Two descriptions of one layer at one depth that price a node differently merge to the
    /// costlier, in either order; a deeper one wins whatever its node; and two that price alike
    /// merge to the same one in either order.
    #[test]
    fn should_merge_two_descriptions_of_a_layer_alike_in_either_order() {
        let grove_version = GroveVersion::latest();
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let wide_key = EstimatedLayerInformation {
            estimated_layer_sizes: AllSubtrees(17, NoSumTrees, None),
            ..layer(3)
        };
        let narrow_key = EstimatedLayerInformation {
            estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            ..layer(3)
        };
        let merged = |first: EstimatedLayerInformation, second: EstimatedLayerInformation| {
            let mut estimation = HashMap::new();
            merge_estimated_layer(&mut estimation, path.clone(), first, grove_version)
                .expect("expected to merge");
            merge_estimated_layer(&mut estimation, path.clone(), second, grove_version)
                .expect("expected to merge");
            estimation[&path]
        };
        assert_eq!(merged(wide_key, narrow_key), wide_key);
        assert_eq!(merged(narrow_key, wide_key), wide_key);

        let deeper_narrow = EstimatedLayerInformation {
            estimated_layer_count: EstimatedLevel(4, false),
            ..narrow_key
        };
        assert_eq!(merged(wide_key, deeper_narrow), deeper_narrow);
        assert_eq!(merged(deeper_narrow, wide_key), deeper_narrow);

        let flagged = EstimatedLayerInformation {
            estimated_layer_sizes: AllSubtrees(1, NoSumTrees, Some(16)),
            ..layer(3)
        };
        assert_eq!(merged(narrow_key, flagged), merged(flagged, narrow_key));
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

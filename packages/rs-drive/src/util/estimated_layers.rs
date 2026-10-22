//! Layer descriptions that several writes of one batch register for the same layer.
//!
//! A batch's cost estimation keeps one description per layer, and a layer's description gives
//! the level the written key sits on, which sets how many ancestors an update propagates
//! through. When two writes of one batch reach keys at different depths of the same layer, the
//! batch must be priced at the deeper one, whichever write registers its description first.

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerInformation;
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

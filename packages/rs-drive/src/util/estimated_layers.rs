//! The layer descriptions a cost estimate hands GroveDB ([`EstimatedLayers`]).
//!
//! GroveDB prices every operation at a layer's path with one description, and a layer's
//! description gives the level the written key sits on, which sets how many ancestors an update
//! propagates through. When two writes of one estimate reach keys at different depths of the
//! same layer, the estimate must be priced at the deeper one, whichever write registers its
//! description first; a write may be a nested estimator of an operation (a token mint describes
//! the root for its supply and then for its balance) as well as an operation of its own. Which
//! of two descriptions GroveDB prices dearer can change with the depth they are priced at, so an
//! estimate that is resolved waits until all of them are known, and only then picks one
//! ([`costliest_layer`]).

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::DriveCost;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::{
    EstimatedLayerCount, EstimatedLayerInformation, EstimatedLayerSizes, EstimatedSumTrees,
    TreeType,
};
use grovedb_merk::estimated_costs::average_case_costs::average_case_merk_propagate;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

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

/// Whether `layer` describes subtrees: GroveDB prices a tree operation at a layer's path (a
/// subtree's new root key, a subtree's deletion) with the subtree sizes, and
/// `layered_flags_size` is GroveDB's own check that a description carries them.
fn describes_subtrees(layer: &EstimatedLayerInformation) -> bool {
    layer.estimated_layer_sizes.layered_flags_size().is_ok()
}

/// The description a resolved estimate keeps for a layer its writes describe as `candidates`
/// (distinct). GroveDB gives every operation at the layer's path this one description, so it
/// must cover each of them: it is as deep as the deepest (`estimate_levels`, which gives a
/// description by element count a depth too), as the depth sets how many nodes every use of the
/// description touches, and at that depth it has the nodes GroveDB prices dearest
/// ([`propagation_credits`]), each candidate being priced at every count that reaches that depth.
/// The kept description can so combine one candidate's nodes with another's depth. When a write
/// describes the layer as holding subtrees ([`describes_subtrees`]), the nodes are picked among
/// the descriptions that do, as a tree operation there needs the subtree sizes. No two writes
/// of a protocol version 15 batch disagree on whether a layer holds subtrees, so this choice
/// changes nothing there; it keeps an estimate from failing should they. Two priced
/// alike are told apart by their contents ([`layer_key`]). The pick is a maximum over the whole
/// set, so it is the same whatever order the candidates were gathered in; picking between two
/// at a time would not be, as a description priced alike or cheaper at a shallow depth can be
/// the dearer at a deeper one.
fn costliest_layer(
    candidates: &[EstimatedLayerInformation],
    platform_version: &PlatformVersion,
) -> Result<EstimatedLayerInformation, Error> {
    if let [only] = candidates {
        return Ok(*only);
    }
    let no_candidate = || {
        Error::Drive(DriveError::CorruptedCodeExecution(
            "a layer is described by at least one candidate",
        ))
    };
    let deepest = candidates
        .iter()
        .map(|candidate| candidate.estimated_layer_count.estimate_levels())
        .max()
        .ok_or_else(no_candidate)?;
    let mut deepest_counts: Vec<EstimatedLayerCount> = vec![];
    for count in candidates
        .iter()
        .map(|candidate| candidate.estimated_layer_count)
        .filter(|count| count.estimate_levels() == deepest)
    {
        if !deepest_counts.contains(&count) {
            deepest_counts.push(count);
        }
    }
    let any_subtrees = candidates.iter().any(describes_subtrees);
    let priced = candidates
        .iter()
        .filter(|candidate| !any_subtrees || describes_subtrees(candidate))
        .flat_map(|candidate| {
            deepest_counts
                .iter()
                .map(move |count| EstimatedLayerInformation {
                    estimated_layer_count: *count,
                    ..*candidate
                })
        })
        .map(|at_depth| {
            Ok((
                propagation_credits(&at_depth, platform_version)?,
                layer_key(&at_depth),
                at_depth,
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    priced
        .into_iter()
        .max_by(|(price, key, _), (other_price, other_key, _)| {
            (price, key).cmp(&(other_price, other_key))
        })
        .map(|(_, _, layer)| layer)
        .ok_or_else(no_candidate)
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

/// Records `layer` among the descriptions of `path` the estimate did not keep, each once.
fn keep_other_description(
    other_descriptions: &mut HashMap<KeyInfoPath, Vec<EstimatedLayerInformation>>,
    path: &KeyInfoPath,
    layer: EstimatedLayerInformation,
) {
    let others = other_descriptions.entry(path.clone()).or_default();
    if !others.contains(&layer) {
        others.push(layer);
    }
}

/// The layer descriptions a cost estimate hands GroveDB, one per layer path.
///
/// Each write describes the layer at a path, and the description written last is the one
/// GroveDB prices ([`EstimatedLayers::into_layers`]) and the one the estimate's own reads see
/// ([`EstimatedLayers::get`]); [`EstimatedLayers::insert_if_absent`] writes only a path nothing
/// described yet. Every estimate before protocol version 15 is priced that way.
///
/// For a path described more than one way, the estimate also keeps the descriptions it did not
/// keep: those a later write replaced, and those `insert_if_absent` did not write. Only
/// `apply_drive_operations` 2 (protocol version 15) resolves an estimate
/// ([`EstimatedLayers::resolve`]): once the whole batch is described, each such path is
/// described by the costliest of all its descriptions ([`costliest_layer`]), so the estimate does
/// not depend on the order the writes came in, and no write is priced at a shallower depth than
/// its own description.
///
/// A read during lowering (`stateless_delete_of_non_tree_for_costs`) sees the description
/// written last, not the resolved one, and GroveDB prices what it reads at once. A reader so
/// reads only layers its own operation described just before reading them; the read is then the
/// same in every order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EstimatedLayers {
    /// The last description written for each path.
    last_written: HashMap<KeyInfoPath, EstimatedLayerInformation>,
    /// For a path described more than one way, every other description a write gave it.
    other_descriptions: HashMap<KeyInfoPath, Vec<EstimatedLayerInformation>>,
}

impl EstimatedLayers {
    /// An estimate describing no layer yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Describes the layer at `path` as `layer`, replacing the last description written for it.
    pub fn insert(&mut self, path: KeyInfoPath, layer: EstimatedLayerInformation) {
        match self.last_written.entry(path) {
            Entry::Occupied(mut written) => {
                let replaced = written.insert(layer);
                if replaced != layer {
                    keep_other_description(&mut self.other_descriptions, written.key(), replaced);
                }
            }
            Entry::Vacant(unwritten) => {
                unwritten.insert(layer);
            }
        }
    }

    /// Describes the layer at `path` as `layer` unless a description of it was written; the
    /// estimate keeps `layer` among the path's other descriptions either way.
    pub fn insert_if_absent(&mut self, path: KeyInfoPath, layer: EstimatedLayerInformation) {
        match self.last_written.entry(path) {
            Entry::Occupied(written) => {
                if *written.get() != layer {
                    keep_other_description(&mut self.other_descriptions, written.key(), layer);
                }
            }
            Entry::Vacant(unwritten) => {
                unwritten.insert(layer);
            }
        }
    }

    /// Keeps, for the described layer at `path`, its last description at `level` among the
    /// path's other descriptions, so a resolved estimate describes the layer at least that deep.
    /// Does nothing for a path no write described.
    pub(crate) fn keep_at_level(&mut self, path: &KeyInfoPath, level: u32) {
        if let Some(written) = self.last_written.get(path) {
            let at_level = EstimatedLayerInformation {
                estimated_layer_count: EstimatedLevel(level, false),
                ..*written
            };
            if at_level != *written {
                keep_other_description(&mut self.other_descriptions, path, at_level);
            }
        }
    }

    /// Describes the layer at `path` as `layer` unless the description written last for it is
    /// at least as deep (`estimate_levels`, which gives a description by element count a depth
    /// too: one that may hold the most elements a layer can is 32 levels deep, the worst case);
    /// the estimate keeps the description it does not write among the path's other
    /// descriptions either way.
    pub(crate) fn insert_keeping_deeper(
        &mut self,
        path: KeyInfoPath,
        layer: EstimatedLayerInformation,
    ) {
        match self.last_written.entry(path) {
            Entry::Occupied(mut written) => {
                let kept = if written.get().estimated_layer_count.estimate_levels()
                    >= layer.estimated_layer_count.estimate_levels()
                {
                    layer
                } else {
                    written.insert(layer)
                };
                if kept != *written.get() {
                    keep_other_description(&mut self.other_descriptions, written.key(), kept);
                }
            }
            Entry::Vacant(unwritten) => {
                unwritten.insert(layer);
            }
        }
    }

    /// The last description written for the layer at `path`.
    pub fn get(&self, path: &KeyInfoPath) -> Option<&EstimatedLayerInformation> {
        self.last_written.get(path)
    }

    /// How many paths are described.
    pub fn len(&self) -> usize {
        self.last_written.len()
    }

    /// Whether no path is described.
    pub fn is_empty(&self) -> bool {
        self.last_written.is_empty()
    }

    /// The estimate with each path described more than one way described by the costliest of
    /// its descriptions ([`costliest_layer`]); a path described one way keeps its description.
    pub(crate) fn resolve(mut self, platform_version: &PlatformVersion) -> Result<Self, Error> {
        for (path, mut descriptions) in std::mem::take(&mut self.other_descriptions) {
            let written = self.last_written.get_mut(&path).ok_or(Error::Drive(
                DriveError::CorruptedCodeExecution(
                    "a path with other descriptions has a description written",
                ),
            ))?;
            if !descriptions.contains(written) {
                descriptions.push(*written);
            }
            *written = costliest_layer(&descriptions, platform_version)?;
        }
        Ok(self)
    }

    /// The descriptions GroveDB prices the estimate with: the last written for each path, or,
    /// once [`EstimatedLayers::resolve`]d, the costliest.
    pub(crate) fn into_layers(self) -> HashMap<KeyInfoPath, EstimatedLayerInformation> {
        self.last_written
    }
}

/// Describes the layer at `path` as `layer`, unless the estimate already describes it at least as
/// deep ([`EstimatedLayers::insert_keeping_deeper`]).
pub(crate) fn insert_estimated_layer_keeping_deeper(
    estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    path: KeyInfoPath,
    layer: EstimatedLayerInformation,
) {
    estimated_costs_only_with_layer_info.insert_keeping_deeper(path, layer);
}

/// Runs `register`, an estimation that overwrites the descriptions of the layers at `paths`, and
/// puts back each description it replaced that was deeper.
pub(crate) fn register_keeping_deeper_estimated_layers(
    estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    paths: &[KeyInfoPath],
    register: impl FnOnce(&mut EstimatedLayers),
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
    use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
    use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees, Mix};
    use grovedb::EstimatedSumTrees::{AllBigSumTrees, NoSumTrees};
    use grovedb::TreeType;

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

    /// The description a resolved estimate keeps for a layer its writes describe as `layers`,
    /// in that order.
    fn kept(layers: &[EstimatedLayerInformation]) -> EstimatedLayerInformation {
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let mut estimate = EstimatedLayers::new();
        for layer in layers {
            estimate.insert(path.clone(), *layer);
        }
        *estimate
            .resolve(PlatformVersion::latest())
            .expect("expected to pick a description")
            .get(&path)
            .expect("expected the layer to be described")
    }

    /// The description `layers` give a layer, the same in every order they come in.
    fn kept_in_every_order(layers: &[EstimatedLayerInformation]) -> EstimatedLayerInformation {
        let mut every_order = orders(layers).into_iter();
        let kept_first = kept(&every_order.next().expect("expected an order"));
        for order in every_order {
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
            tree_type: TreeType::NormalTree,
            estimated_layer_count: EstimatedLevel(level, false),
            estimated_layer_sizes,
        }
    }

    /// The deepest depth, the nodes GroveDB's propagation estimate prices dearest at that depth,
    /// then, for two it prices alike, a fixed choice; the same in every order.
    #[test]
    fn should_keep_the_costliest_description_of_a_layer_in_every_order() {
        // Items under 32-byte keys cost more to carry a change through than items of the same
        // size under 1-byte keys.
        let items = at_level(3, AllItems(32, 8, None));
        let narrow_items = at_level(3, AllItems(1, 8, None));
        assert!(credits(&items) > credits(&narrow_items));
        assert_eq!(kept_in_every_order(&[items, narrow_items]), items);

        // A deeper description of narrower nodes, priced below the shallower one: the batch
        // keeps the deeper depth and the dearer nodes, a description neither write gave, so
        // neither write is priced below its own description.
        let deeper_narrow = at_level(4, AllItems(1, 1, None));
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
    /// descriptions made before a deeper one arrives would rest on the tie-break (which keeps
    /// the references), and could keep the nodes that are the cheaper at the deeper depth. The
    /// batch keeps all three until the deepest depth is known, then the nodes dearest there, in
    /// every order.
    #[test]
    fn should_pick_among_shallow_descriptions_at_the_deepest_depth() {
        let items = at_level(0, AllItems(32, 8, None));
        let references = at_level(0, AllReference(1, 8, None));
        let deep_narrow = at_level(4, AllItems(1, 1, None));
        assert_eq!(credits(&items), credits(&references));
        let items_at_4 = at_level(4, AllItems(32, 8, None));
        assert!(credits(&items_at_4) > credits(&at_level(4, AllReference(1, 8, None))));
        assert!(credits(&items_at_4) > credits(&deep_narrow));
        assert_eq!(
            kept_in_every_order(&[items, references, deep_narrow]),
            items_at_4
        );
    }

    /// Items of 8 bytes under 32-byte keys cost more to carry a change through than big sum
    /// subtrees under the same keys (GroveDB counts the item's two stored hashes), but a write
    /// that describes the layer as holding subtrees keeps a description of subtrees: GroveDB
    /// prices a tree operation there with the subtree sizes, and refuses a description of items
    /// alone. The depth is still the deepest any write gives, and a `Mix` holding subtrees is a
    /// description of subtrees.
    #[test]
    fn should_describe_a_layer_holding_subtrees_with_subtrees() {
        let items = at_level(3, AllItems(32, 8, None));
        let subtrees = at_level(2, AllSubtrees(32, AllBigSumTrees, None));
        let subtrees_at_3 = at_level(3, AllSubtrees(32, AllBigSumTrees, None));
        assert!(credits(&items) > credits(&subtrees_at_3));
        assert_eq!(kept_in_every_order(&[items, subtrees]), subtrees_at_3);

        let mix = at_level(
            1,
            Mix {
                subtrees_size: Some((1, NoSumTrees, None, 1)),
                items_size: Some((1, 8, None, 1)),
                references_size: None,
                items_with_sum_item_size: None,
                references_with_sum_item_size: None,
            },
        );
        let mix_at_3 = EstimatedLayerInformation {
            estimated_layer_count: EstimatedLevel(3, false),
            ..mix
        };
        assert_eq!(kept_in_every_order(&[items, mix]), mix_at_3);
    }

    /// Two descriptions of one depth with different counts: each candidate is priced at each
    /// count, and the batch keeps the dearest, in either order.
    #[test]
    fn should_price_every_candidate_at_every_count_of_the_deepest_depth() {
        let items = at_level(2, AllItems(32, 8, None));
        let by_count = EstimatedLayerInformation {
            estimated_layer_count: ApproximateElements(3),
            ..at_level(2, AllItems(1, 8, None))
        };
        assert_eq!(by_count.estimated_layer_count.estimate_levels(), 2);
        let kept = kept_in_every_order(&[items, by_count]);
        assert_eq!(kept.estimated_layer_sizes, AllItems(32, 8, None));
        assert!(credits(&kept) >= credits(&items));
        assert!(credits(&kept) >= credits(&by_count));
    }

    /// A layer every write describes alike is kept as described.
    #[test]
    fn should_keep_a_layer_described_alike_as_it_is() {
        let items = at_level(3, AllItems(32, 8, None));
        assert_eq!(kept(&[items, items, items]), items);
    }

    /// GroveDB prices the description written last unless the estimate is resolved, as every
    /// estimate before `apply_drive_operations` 2 is priced; `insert_if_absent` writes only an
    /// undescribed path. Resolving picks the costliest of every description the writes gave,
    /// the one `insert_if_absent` did not write included.
    #[test]
    fn should_keep_the_last_description_written_until_resolved() {
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let deep = at_level(3, AllItems(32, 8, None));
        let shallow = at_level(1, AllItems(1, 8, None));
        let mut estimate = EstimatedLayers::new();
        estimate.insert(path.clone(), shallow);
        estimate.insert_if_absent(path.clone(), deep);
        assert_eq!(estimate.get(&path), Some(&shallow));
        assert_eq!(
            estimate.clone().into_layers(),
            HashMap::from([(path.clone(), shallow)])
        );
        assert_eq!(
            estimate
                .resolve(PlatformVersion::latest())
                .expect("expected to resolve")
                .into_layers(),
            HashMap::from([(path, deep)])
        );
    }

    /// A described layer kept at a deeper level is resolved at that level, with the nodes of
    /// its description; an undescribed layer stays undescribed.
    #[test]
    fn should_resolve_a_layer_kept_at_a_level_at_least_that_deep() {
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let undescribed = KeyInfoPath::from_known_path([[2u8].as_slice()]);
        let mut estimate = EstimatedLayers::new();
        estimate.insert(path.clone(), at_level(3, AllSubtrees(1, NoSumTrees, None)));
        estimate.keep_at_level(&path, 4);
        estimate.keep_at_level(&undescribed, 4);
        let resolved = estimate
            .resolve(PlatformVersion::latest())
            .expect("expected to resolve");
        assert_eq!(
            resolved.get(&path),
            Some(&at_level(4, AllSubtrees(1, NoSumTrees, None)))
        );
        assert_eq!(resolved.get(&undescribed), None);
    }

    fn layer(level: u32) -> EstimatedLayerInformation {
        at_level(level, AllSubtrees(1, NoSumTrees, None))
    }

    fn level_at(estimation: &EstimatedLayers, path: &KeyInfoPath) -> Option<u32> {
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
        let mut estimation = EstimatedLayers::new();
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

    #[test]
    fn should_put_back_a_deeper_level_an_estimation_overwrote() {
        let deep = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let shallow = KeyInfoPath::from_known_path([[2u8].as_slice()]);
        let mut estimation = EstimatedLayers::new();
        estimation.insert(deep.clone(), layer(2));
        estimation.insert(shallow.clone(), layer(0));
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

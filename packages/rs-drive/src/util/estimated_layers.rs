//! The layer descriptions a cost estimate hands GroveDB ([`EstimatedLayers`]).
//!
//! GroveDB prices every operation at a layer's path with one description, and a layer's
//! description gives the level the written key sits on, which sets how many ancestors an update
//! propagates through. When two writes of one estimate reach keys at different depths of the
//! same layer, the estimate must be priced at the deeper one, whichever write registers its
//! description first; a write may be a nested estimator of an operation (a token mint describes
//! the root for its supply and then for its balance) as well as an operation of its own. Which
//! of two descriptions GroveDB prices dearer can change with the depth they are priced at, so an
//! estimate that keeps every description waits until all of them are known, and only then picks
//! one.

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
/// subtree's new root key, a subtree's deletion) with the subtree sizes, which a description of
/// items or references alone lacks (`layered_flags_size` refuses it).
fn describes_subtrees(layer: &EstimatedLayerInformation) -> bool {
    matches!(
        layer.estimated_layer_sizes,
        EstimatedLayerSizes::AllSubtrees(..)
            | EstimatedLayerSizes::Mix {
                subtrees_size: Some(_),
                ..
            }
    )
}

/// The description a batch keeps for a layer its operations describe as `candidates`. GroveDB
/// gives every operation at the layer's path this one description, so it must cover each of
/// them: it is as deep as the deepest (`estimate_levels`, which gives a description by element
/// count a depth too), as the depth sets how many nodes every use of the description touches,
/// and at that depth it has the nodes GroveDB prices dearest ([`propagation_credits`]), each
/// candidate being priced at every count that reaches that depth. When a write describes the
/// layer as holding subtrees ([`describes_subtrees`]), the nodes are picked among the
/// descriptions that do, as a tree operation there needs the subtree sizes: a write describing
/// the same layer as items alone (the total system credits item in `Misc`, beside the token
/// supplies tree) is then priced with a subtree description, as generation 1 priced it whenever
/// the subtree description came last. Two priced alike are told apart by their contents
/// ([`layer_key`]). The pick is a maximum over the whole set, so it is the same whatever order
/// the candidates were gathered in; picking between two at a time would not be, as a
/// description priced alike or cheaper at a shallow depth can be the dearer at a deeper one.
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
    let any_subtrees = candidates.iter().any(describes_subtrees);
    let mut costliest: Option<(Credits, Vec<u64>, EstimatedLayerInformation)> = None;
    for candidate in candidates
        .iter()
        .filter(|candidate| !any_subtrees || describes_subtrees(candidate))
    {
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

/// Every description an estimate's writes give each layer, each description once.
type LayerCandidates = HashMap<KeyInfoPath, Vec<EstimatedLayerInformation>>;

/// The description an estimate keeps for each layer its writes describe: the costliest of the
/// layer's candidates ([`costliest_layer`]). The order the writes register their descriptions of
/// a shared layer in does not matter.
fn costliest_estimated_layers(
    candidates: LayerCandidates,
    platform_version: &PlatformVersion,
) -> Result<HashMap<KeyInfoPath, EstimatedLayerInformation>, Error> {
    candidates
        .into_iter()
        .map(|(path, layers)| Ok((path, costliest_layer(&layers, platform_version)?)))
        .collect()
}

/// The layer descriptions a cost estimate hands GroveDB, one per layer path.
///
/// [`EstimatedLayers::new`] keeps, for each path, the last description written: every estimate
/// relies on that except `apply_drive_operations` 2 (protocol version 15). That generation
/// builds the estimate of a batch with [`EstimatedLayers::keeping_every_description`], which also
/// keeps every description any write gives a path, the nested estimators of one operation
/// included, and [`EstimatedLayers::resolve`] then describes each path by the costliest of them.
/// No write of the batch is priced at a shallower depth than its own description, and the
/// order the writes were described in does not matter. The descriptions reach GroveDB through
/// [`EstimatedLayers::into_layers`], which refuses an estimate that was not resolved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EstimatedLayers {
    /// The last description written for each path, which the estimate's own reads see.
    last_written: HashMap<KeyInfoPath, EstimatedLayerInformation>,
    /// Every description written for each path, when the estimate keeps them.
    every_description: Option<LayerCandidates>,
}

impl EstimatedLayers {
    /// An estimate that keeps the last description written for each path.
    pub fn new() -> Self {
        Self::default()
    }

    /// An estimate that keeps every description written for each path until
    /// [`EstimatedLayers::resolve`] picks the costliest.
    pub(crate) fn keeping_every_description() -> Self {
        Self {
            last_written: HashMap::new(),
            every_description: Some(LayerCandidates::new()),
        }
    }

    fn keep(&mut self, path: &KeyInfoPath, layer: EstimatedLayerInformation) {
        if let Some(every_description) = self.every_description.as_mut() {
            let described = every_description.entry(path.clone()).or_default();
            if !described.contains(&layer) {
                described.push(layer);
            }
        }
    }

    /// Describes the layer at `path` as `layer`, replacing the last description written for it,
    /// and returns that one.
    pub fn insert(
        &mut self,
        path: KeyInfoPath,
        layer: EstimatedLayerInformation,
    ) -> Option<EstimatedLayerInformation> {
        self.keep(&path, layer);
        self.last_written.insert(path, layer)
    }

    /// Describes the layer at `path` as `layer` unless a description of it was written; an
    /// estimate keeping every description keeps this one either way.
    pub fn insert_if_absent(&mut self, path: KeyInfoPath, layer: EstimatedLayerInformation) {
        self.keep(&path, layer);
        self.last_written.entry(path).or_insert(layer);
    }

    /// Describes the layer at `path` as `layer` unless the description written last for it is
    /// at least as deep (`estimate_levels`, which gives a description by element count a depth
    /// too: one that may hold the most elements a layer can is 32 levels deep, the worst case);
    /// an estimate keeping every description keeps this one either way.
    pub(crate) fn insert_keeping_deeper(
        &mut self,
        path: KeyInfoPath,
        layer: EstimatedLayerInformation,
    ) {
        self.keep(&path, layer);
        let deeper_written = self.last_written.get(&path).is_some_and(|existing| {
            existing.estimated_layer_count.estimate_levels()
                >= layer.estimated_layer_count.estimate_levels()
        });
        if !deeper_written {
            self.last_written.insert(path, layer);
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

    /// The estimate GroveDB prices: for an estimate keeping every description, each path
    /// described by the costliest of its descriptions; any other estimate unchanged.
    pub(crate) fn resolve(self, platform_version: &PlatformVersion) -> Result<Self, Error> {
        match self.every_description {
            None => Ok(self),
            Some(every_description) => Ok(Self {
                last_written: costliest_estimated_layers(every_description, platform_version)?,
                every_description: None,
            }),
        }
    }

    /// The descriptions GroveDB prices the estimate with. An estimate keeping every description
    /// is refused until [`EstimatedLayers::resolve`] picked the costliest of them, so none is
    /// priced with the last description written instead.
    pub(crate) fn into_layers(
        self,
    ) -> Result<HashMap<KeyInfoPath, EstimatedLayerInformation>, Error> {
        match self.every_description {
            None => Ok(self.last_written),
            Some(_) => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "an estimate keeping every layer description is resolved before GroveDB prices it",
            ))),
        }
    }
}

impl From<HashMap<KeyInfoPath, EstimatedLayerInformation>> for EstimatedLayers {
    fn from(last_written: HashMap<KeyInfoPath, EstimatedLayerInformation>) -> Self {
        Self {
            last_written,
            every_description: None,
        }
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

    /// The description an estimate keeping every description resolves a layer to when its
    /// writes describe it as `layers`, in that order.
    fn kept(layers: &[EstimatedLayerInformation]) -> EstimatedLayerInformation {
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let mut estimate = EstimatedLayers::keeping_every_description();
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
        // Items under 32-byte keys cost more to carry a change through than items of the same
        // size under 1-byte keys.
        let items = at_level(3, AllItems(32, 8, None));
        let narrow_items = at_level(3, AllItems(1, 8, None));
        assert!(credits(&items) > credits(&narrow_items));
        assert_eq!(kept_in_every_order(&[items, narrow_items]), items);

        // A deeper description of narrower nodes, priced below the shallower one: the batch
        // keeps the deeper depth and the dearer nodes, so neither operation is priced below its
        // own description.
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

    /// A layer every operation describes alike is kept as described, without pricing.
    #[test]
    fn should_keep_a_layer_described_alike_as_it_is() {
        let items = at_level(3, AllItems(32, 8, None));
        assert_eq!(kept(&[items, items, items]), items);
    }

    /// The default estimate keeps the last description written for a path, as every estimate
    /// before `apply_drive_operations` 2 did, and resolving it changes nothing.
    #[test]
    fn should_keep_the_last_description_written_by_default() {
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let deep = at_level(3, AllItems(32, 8, None));
        let shallow = at_level(1, AllSubtrees(1, NoSumTrees, None));
        let mut estimate = EstimatedLayers::new();
        assert_eq!(estimate.insert(path.clone(), deep), None);
        assert_eq!(estimate.insert(path.clone(), shallow), Some(deep));
        estimate.insert_if_absent(path.clone(), deep);
        let resolved = estimate
            .clone()
            .resolve(PlatformVersion::latest())
            .expect("expected to resolve");
        assert_eq!(resolved, estimate);
        assert_eq!(
            estimate.into_layers().expect("expected the layers"),
            HashMap::from([(path, shallow)])
        );
    }

    /// An estimate keeping every description reads the last one written, keeps a description
    /// `insert_if_absent` would not write, and reaches GroveDB only once resolved, as the
    /// costliest of them.
    #[test]
    fn should_keep_every_description_until_resolved() {
        let path = KeyInfoPath::from_known_path([[1u8].as_slice()]);
        let deep = at_level(3, AllItems(32, 8, None));
        let shallow = at_level(1, AllItems(1, 8, None));
        let mut estimate = EstimatedLayers::keeping_every_description();
        estimate.insert(path.clone(), shallow);
        estimate.insert_if_absent(path.clone(), deep);
        assert_eq!(estimate.get(&path), Some(&shallow));
        assert!(matches!(
            estimate.clone().into_layers(),
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ));
        let layers = estimate
            .resolve(PlatformVersion::latest())
            .expect("expected to resolve")
            .into_layers()
            .expect("expected the layers");
        assert_eq!(layers, HashMap::from([(path, deep)]));
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
        let mut estimation = EstimatedLayers::from(HashMap::from([
            (deep.clone(), layer(2)),
            (shallow.clone(), layer(0)),
        ]));
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

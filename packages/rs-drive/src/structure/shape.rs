use crate::drive::Drive;
use crate::structure::export::{LayerShape, ShapeNode};
use crate::structure::{KeySpec, NodeId, StructureNode};
use dpp::version::PlatformVersion;
use grovedb::operations::proof::{GroveDBProof, LayerProof, MerkOnlyLayerProof, ProofBytes};
use grovedb::{MerkProofDecoder, PathQuery, Query, SizedQuery};
use grovedb_merk::proofs::tree::{execute, Tree};
use std::collections::BTreeMap;

/// The exact Merk binary tree of every layer of `drive` whose place and keys
/// are fixed: the layer's path holds fixed keys only, and so do its children.
///
/// GroveDB does not expose the links between the nodes of a Merk, but a proof
/// does: the proof of a query for everything in a layer lists every node and
/// how they connect, so replaying it rebuilds the tree.
pub(super) fn layer_shapes(
    drive: &Drive,
    root: &StructureNode,
    origin: &str,
    platform_version: &PlatformVersion,
) -> BTreeMap<NodeId, LayerShape> {
    let mut shapes = BTreeMap::new();
    let mut layers: Vec<(Vec<Vec<u8>>, &StructureNode)> = vec![(vec![], root)];
    while let Some((path, node)) = layers.pop() {
        let all_fixed = node
            .children
            .iter()
            .all(|child| matches!(child.key, KeySpec::Fixed { .. }));
        if all_fixed && node.children.len() > 1 {
            if let Some(tree) = layer_tree(drive, &path, platform_version) {
                shapes.insert(
                    node.id.clone(),
                    LayerShape {
                        origin: origin.to_string(),
                        tree: shape_of(&tree),
                        states: vec![],
                    },
                );
            }
        }
        for child in &node.children {
            let holds_merk = child
                .kinds
                .iter()
                .any(|kind| kind.is_tree() && !kind.is_opaque());
            if let (Some(key), true) = (child.fixed_key_bytes(), holds_merk) {
                let mut child_path = path.clone();
                child_path.push(key.to_vec());
                layers.push((child_path, child));
            }
        }
    }
    shapes
}

/// The exact Merk binary tree of the layer at `path`
pub(super) fn shape_at(
    drive: &Drive,
    path: &[Vec<u8>],
    platform_version: &PlatformVersion,
) -> Option<ShapeNode> {
    layer_tree(drive, path, platform_version).map(|tree| shape_of(&tree))
}

/// The Merk of the layer at `path`, rebuilt from a proof of everything in it
fn layer_tree(drive: &Drive, path: &[Vec<u8>], platform_version: &PlatformVersion) -> Option<Tree> {
    let path_query = PathQuery::new(
        path.to_vec(),
        SizedQuery::new(Query::new_range_full(), None, None),
    );
    let proof = drive
        .grove
        .prove_query_non_serialized(&path_query, None, &platform_version.drive.grove_version)
        .unwrap()
        .expect("expected to prove the layer");

    let merk_proof = match proof {
        GroveDBProof::V0(proof) => merk_only_layer(&proof.root_layer, path)?,
        GroveDBProof::V1(proof) => layer(&proof.root_layer, path)?,
    };
    let tree = execute(MerkProofDecoder::new(&merk_proof), false, |_| Ok(()))
        .unwrap()
        .expect("expected to replay the layer's proof");
    Some(tree)
}

fn merk_only_layer(proof: &MerkOnlyLayerProof, path: &[Vec<u8>]) -> Option<Vec<u8>> {
    match path.split_first() {
        None => Some(proof.merk_proof.clone()),
        Some((key, rest)) => merk_only_layer(proof.lower_layers.get(key)?, rest),
    }
}

fn layer(proof: &LayerProof, path: &[Vec<u8>]) -> Option<Vec<u8>> {
    match path.split_first() {
        None => match &proof.merk_proof {
            ProofBytes::Merk(bytes) => Some(bytes.clone()),
            _ => None,
        },
        Some((key, rest)) => layer(proof.lower_layers.get(key)?, rest),
    }
}

fn shape_of(tree: &Tree) -> ShapeNode {
    ShapeNode {
        key: tree
            .key()
            .expect("expected every node of a full layer proof to carry its key")
            .to_vec(),
        left: tree
            .left
            .as_ref()
            .map(|child| Box::new(shape_of(&child.tree))),
        right: tree
            .right
            .as_ref()
            .map(|child| Box::new(shape_of(&child.tree))),
    }
}

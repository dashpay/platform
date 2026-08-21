//! Merk search-path structure extracted from locally generated GroveDB proofs.
//!
//! The node generates a proof for a single key with
//! [`Drive::grove_get_proved_path_query`](crate::drive::Drive) against its own
//! committed state and immediately decodes it here to learn the *structure* of
//! the terminal merk layer: how many levels the search path for that key
//! traverses, and whether the key is present. No cryptographic verification is
//! performed — the proof never leaves the node that produced it.
//!
//! The op-stream semantics mirror `merk::proofs::tree::execute`:
//! `Push`/`PushInverted` push a node, `Parent`/`ParentInverted` pop the parent
//! then the child, `Child`/`ChildInverted` pop the child then the parent; in
//! every case the child is attached one level below the parent. Only depths
//! matter here, so left/right orientation is ignored.

use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::operations::proof::{GroveDBProof, LayerProof, ProofBytes};
use grovedb::{MerkProofDecoder, MerkProofNode, MerkProofOp};

/// Structure information about a single key's search path inside one merk
/// layer, read from a locally generated proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SingleKeyProofLevels {
    /// Whether the key is present in the tree.
    pub present: bool,
    /// The number of merk levels the operation on this key touches: for a
    /// present key, the number of nodes on the root→key path (a replace); for
    /// an absent key, the number of nodes on the root→boundary path plus one
    /// (an insert hangs the new node below the deepest absence boundary).
    pub levels: u8,
}

/// The key-bearing nodes of a partially assembled proof subtree, recorded as
/// `(depth from the subtree root, is the target key)`.
struct SubtreeKeyedNodes {
    keyed: Vec<(u8, bool)>,
}

/// Decodes a locally generated GroveDB proof and returns the search-path
/// structure for `key` in the merk layer at `path`.
///
/// `proof_bytes` must be the exact output of a single-key
/// `grove_get_proved_path_query` over `path`/`key` on this node — the decode
/// is canonical (trailing bytes rejected) and only `GroveDBProof::V1`
/// envelopes with a plain merk terminal layer are supported.
pub(crate) fn single_key_proof_levels(
    proof_bytes: &[u8],
    path: &[&[u8]],
    key: &[u8],
) -> Result<SingleKeyProofLevels, Error> {
    // The same bincode configuration the prover writes out.
    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<{ 256 * 1024 * 1024 }>();
    let (proof, consumed): (GroveDBProof, usize) = bincode::decode_from_slice(proof_bytes, config)
        .map_err(|e| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "unable to decode local grovedb proof: {e}"
            )))
        })?;
    if consumed != proof_bytes.len() {
        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
            "local grovedb proof has {} trailing bytes",
            proof_bytes.len() - consumed
        ))));
    }
    let GroveDBProof::V1(v1) = proof else {
        return Err(Error::Drive(DriveError::CorruptedDriveState(
            "local grovedb proof is not a V1 envelope".to_string(),
        )));
    };
    let mut layer: &LayerProof = &v1.root_layer;
    for segment in path {
        match layer.lower_layers.get(*segment) {
            Some(lower_layer) => layer = lower_layer,
            None => {
                // The prover does not descend into an empty subtree: for a
                // locally generated single-key proof over the queried path, a
                // missing lower layer means the terminal tree is empty, so
                // the operation lands at its root.
                return Ok(SingleKeyProofLevels {
                    present: false,
                    levels: 1,
                });
            }
        }
    }
    let ProofBytes::Merk(merk_bytes) = &layer.merk_proof else {
        return Err(Error::Drive(DriveError::CorruptedDriveState(
            "local grovedb proof terminal layer is not a merk proof".to_string(),
        )));
    };
    merk_single_key_levels(merk_bytes, key)
}

/// Runs the merk proof op stream through a depth-only reconstruction and
/// derives [`SingleKeyProofLevels`] for `key`.
fn merk_single_key_levels(
    merk_proof_bytes: &[u8],
    key: &[u8],
) -> Result<SingleKeyProofLevels, Error> {
    let mut stack: Vec<SubtreeKeyedNodes> = Vec::new();
    for op in MerkProofDecoder::new(merk_proof_bytes) {
        let op = op.map_err(|e| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "unable to decode local merk proof op: {e}"
            )))
        })?;
        match op {
            MerkProofOp::Push(node) | MerkProofOp::PushInverted(node) => {
                stack.push(subtree_from_node(node, key)?);
            }
            MerkProofOp::Parent | MerkProofOp::ParentInverted => {
                let parent = pop_subtree(&mut stack)?;
                let child = pop_subtree(&mut stack)?;
                stack.push(attach_child(parent, child)?);
            }
            MerkProofOp::Child | MerkProofOp::ChildInverted => {
                let child = pop_subtree(&mut stack)?;
                let parent = pop_subtree(&mut stack)?;
                stack.push(attach_child(parent, child)?);
            }
        }
    }

    if stack.is_empty() {
        // An empty tree: the operation lands at the root.
        return Ok(SingleKeyProofLevels {
            present: false,
            levels: 1,
        });
    }
    if stack.len() != 1 {
        return Err(Error::Drive(DriveError::CorruptedDriveState(
            "local merk proof op stream did not assemble into a single tree".to_string(),
        )));
    }
    let keyed = stack.pop().expect("checked non-empty above").keyed;

    if let Some((depth, _)) = keyed.iter().find(|(_, is_target)| *is_target) {
        // Present: a replace touches every node on the root→key path.
        Ok(SingleKeyProofLevels {
            present: true,
            levels: depth.saturating_add(1),
        })
    } else {
        // Absent: an insert hangs the new node below the deepest boundary.
        let levels = match keyed.iter().map(|(depth, _)| *depth).max() {
            Some(boundary_depth) => boundary_depth.saturating_add(2),
            None => 1,
        };
        Ok(SingleKeyProofLevels {
            present: false,
            levels,
        })
    }
}

fn pop_subtree(stack: &mut Vec<SubtreeKeyedNodes>) -> Result<SubtreeKeyedNodes, Error> {
    stack.pop().ok_or_else(|| {
        Error::Drive(DriveError::CorruptedDriveState(
            "local merk proof op stream underflowed its stack".to_string(),
        ))
    })
}

fn attach_child(
    mut parent: SubtreeKeyedNodes,
    child: SubtreeKeyedNodes,
) -> Result<SubtreeKeyedNodes, Error> {
    for (depth, is_target) in child.keyed {
        let bumped = depth.checked_add(1).ok_or_else(|| {
            Error::Drive(DriveError::CorruptedDriveState(
                "local merk proof path depth overflowed".to_string(),
            ))
        })?;
        parent.keyed.push((bumped, is_target));
    }
    Ok(parent)
}

fn subtree_from_node(node: MerkProofNode, target: &[u8]) -> Result<SubtreeKeyedNodes, Error> {
    let keyed = match node {
        MerkProofNode::Hash(_) | MerkProofNode::KVHash(_) | MerkProofNode::KVHashCount(_, _) => {
            vec![]
        }
        MerkProofNode::KV(key, _)
        | MerkProofNode::KVValueHash(key, _, _)
        | MerkProofNode::KVValueHashFeatureType(key, _, _, _)
        | MerkProofNode::KVRefValueHash(key, _, _)
        | MerkProofNode::KVCount(key, _, _)
        | MerkProofNode::KVRefValueHashCount(key, _, _, _)
        | MerkProofNode::KVValueHashFeatureTypeWithChildHash(key, _, _, _, _) => {
            vec![(0, key.as_slice() == target)]
        }
        MerkProofNode::KVDigest(key, _) | MerkProofNode::KVDigestCount(key, _, _) => {
            vec![(0, key.as_slice() == target)]
        }
        other => {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "unexpected node type in local single-key merk proof: {other:?}"
            ))));
        }
    };
    Ok(SubtreeKeyedNodes { keyed })
}

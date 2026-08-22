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
use grovedb::operations::proof::{GroveDBProof, LayerProof, MerkOnlyLayerProof, ProofBytes};
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
    // A missing lower layer in either envelope means the prover did not
    // descend into an empty subtree: for a locally generated single-key
    // proof over the queried path, the terminal tree is empty and the
    // operation lands at its root.
    let empty_terminal = SingleKeyProofLevels {
        present: false,
        levels: 1,
    };
    let merk_bytes: &[u8] = match &proof {
        // Current envelope (GROVE_V3+, protocol v12+).
        GroveDBProof::V1(v1) => {
            let mut layer: &LayerProof = &v1.root_layer;
            for segment in path {
                match layer.lower_layers.get(*segment) {
                    Some(lower_layer) => layer = lower_layer,
                    None => return Ok(empty_terminal),
                }
            }
            let ProofBytes::Merk(bytes) = &layer.merk_proof else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "local grovedb proof terminal layer is not a merk proof".to_string(),
                )));
            };
            bytes
        }
        // Legacy merk-only envelope, still produced by GROVE_V2's prove
        // path — selected by protocol v11 through DRIVE_VERSION_V6.
        GroveDBProof::V0(v0) => {
            let mut layer: &MerkOnlyLayerProof = &v0.root_layer;
            for segment in path {
                match layer.lower_layers.get(*segment) {
                    Some(lower_layer) => layer = lower_layer,
                    None => return Ok(empty_terminal),
                }
            }
            &layer.merk_proof
        }
    };
    merk_single_key_levels(merk_bytes, key)
}

/// Decodes a merk proof byte stream and derives [`SingleKeyProofLevels`]
/// for `key`.
fn merk_single_key_levels(
    merk_proof_bytes: &[u8],
    key: &[u8],
) -> Result<SingleKeyProofLevels, Error> {
    let mut ops = Vec::new();
    for op in MerkProofDecoder::new(merk_proof_bytes) {
        ops.push(op.map_err(|e| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "unable to decode local merk proof op: {e}"
            )))
        })?);
    }
    single_key_levels_from_ops(ops, key)
}

/// Runs an already-decoded merk proof op stream through a depth-only
/// reconstruction and derives [`SingleKeyProofLevels`] for `key`.
fn single_key_levels_from_ops(
    ops: impl IntoIterator<Item = MerkProofOp>,
    key: &[u8],
) -> Result<SingleKeyProofLevels, Error> {
    let mut stack: Vec<SubtreeKeyedNodes> = Vec::new();
    for op in ops {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::Drive;
    use crate::util::batch::drive_op_batch::AddressFundsOperationType;
    use crate::util::batch::DriveOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;

    // ---------------------------------------------------------------
    // Synthetic op streams: pin the two subtle rules of the
    // reconstruction — the Parent/Child pop order and the absent-key
    // "+2" rule — plus the failure modes, with exact values.
    // ---------------------------------------------------------------

    fn kv(key: &[u8]) -> MerkProofNode {
        MerkProofNode::KV(key.to_vec(), vec![0xAA])
    }

    #[test]
    fn test_empty_op_stream_is_an_empty_tree() {
        let levels = single_key_levels_from_ops([], b"x").expect("empty stream");
        assert_eq!(
            levels,
            SingleKeyProofLevels {
                present: false,
                levels: 1
            }
        );
    }

    #[test]
    fn test_single_leaf_present_and_absent() {
        let present =
            single_key_levels_from_ops([MerkProofOp::Push(kv(b"a"))], b"a").expect("present");
        assert_eq!(
            present,
            SingleKeyProofLevels {
                present: true,
                levels: 1
            }
        );

        // The absent key hangs below the single boundary node at depth 0.
        let absent =
            single_key_levels_from_ops([MerkProofOp::Push(kv(b"a"))], b"x").expect("absent");
        assert_eq!(
            absent,
            SingleKeyProofLevels {
                present: false,
                levels: 2
            }
        );
    }

    #[test]
    fn test_parent_attaches_the_previously_pushed_child() {
        // Push(child a), Push(parent), Parent: pops the parent first, then
        // the child — a must land one level below the KVHash root.
        let ops = [
            MerkProofOp::Push(kv(b"a")),
            MerkProofOp::Push(MerkProofNode::KVHash([0u8; 32])),
            MerkProofOp::Parent,
            MerkProofOp::Push(MerkProofNode::Hash([1u8; 32])),
            MerkProofOp::Child,
        ];
        let levels = single_key_levels_from_ops(ops, b"a").expect("present");
        assert_eq!(
            levels,
            SingleKeyProofLevels {
                present: true,
                levels: 2
            }
        );
    }

    #[test]
    fn test_child_attaches_the_top_of_stack_below_the_parent() {
        // Push(parent), Push(child b), Child: pops the child first — b must
        // land one level below the KVHash root.
        let ops = [
            MerkProofOp::Push(MerkProofNode::KVHash([0u8; 32])),
            MerkProofOp::Push(kv(b"b")),
            MerkProofOp::Child,
        ];
        let levels = single_key_levels_from_ops(ops, b"b").expect("present");
        assert_eq!(
            levels,
            SingleKeyProofLevels {
                present: true,
                levels: 2
            }
        );
    }

    #[test]
    fn test_absence_boundary_via_digest_uses_the_plus_two_rule() {
        // Root is a keyless KVHash; the only key-bearing node is a KVDigest
        // boundary at depth 1 — an absent key hangs below it: 1 + 2 = 3.
        let ops = [
            MerkProofOp::Push(MerkProofNode::KVDigest(b"a".to_vec(), [2u8; 32])),
            MerkProofOp::Push(MerkProofNode::KVHash([0u8; 32])),
            MerkProofOp::Parent,
        ];
        let levels = single_key_levels_from_ops(ops, b"x").expect("absent");
        assert_eq!(
            levels,
            SingleKeyProofLevels {
                present: false,
                levels: 3
            }
        );
    }

    #[test]
    fn test_stack_underflow_and_leftover_subtrees_are_errors() {
        for underflow in [
            vec![MerkProofOp::Parent],
            vec![MerkProofOp::Child],
            vec![MerkProofOp::Push(kv(b"a")), MerkProofOp::Parent],
        ] {
            assert!(
                single_key_levels_from_ops(underflow.clone(), b"a").is_err(),
                "stack underflow must be rejected: {underflow:?}"
            );
        }

        let leftover = [MerkProofOp::Push(kv(b"a")), MerkProofOp::Push(kv(b"b"))];
        assert!(
            single_key_levels_from_ops(leftover, b"a").is_err(),
            "an op stream leaving two subtrees must be rejected"
        );
    }

    // ---------------------------------------------------------------
    // Real generated proofs over known AVL shapes: keys are inserted
    // one batch at a time in an order that never triggers a rotation,
    // so every depth is derivable by hand. Levels are exact, covering
    // present keys on both sides at several depths and absences below
    // the minimum, in interior gaps, and above the maximum.
    // ---------------------------------------------------------------

    fn address(n: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([n; 20])
    }

    fn seed_balance(drive: &Drive, n: u8, platform_version: &PlatformVersion) {
        drive
            .apply_drive_operations(
                vec![DriveOperation::AddressFundsOperation(
                    AddressFundsOperationType::SetBalanceToAddress {
                        address: address(n),
                        nonce: 0,
                        balance: 1_000,
                    },
                )],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("seed balance");
    }

    fn levels_for(
        drive: &Drive,
        n: u8,
        platform_version: &PlatformVersion,
    ) -> SingleKeyProofLevels {
        let queried = address(n);
        let query = Drive::balance_for_clear_address_query(&queried);
        let proof = drive
            .grove_get_proved_path_query(&query, None, &mut vec![], &platform_version.drive)
            .expect("prove");
        let path = Drive::clear_addresses_path();
        let segments: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
        single_key_proof_levels(&proof, &segments, queried.to_bytes().as_slice())
            .expect("decode levels")
    }

    #[test]
    fn test_exact_levels_on_known_avl_shapes() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);

        let expect = |n: u8, present: bool, levels: u8, what: &str| {
            let decoded = levels_for(&drive, n, platform_version);
            assert_eq!(
                decoded,
                SingleKeyProofLevels { present, levels },
                "{what}: key {n}"
            );
        };

        // Empty tree: any key lands at the root.
        expect(40, false, 1, "empty tree");

        // {40}: a single root.
        seed_balance(&drive, 40, platform_version);
        expect(40, true, 1, "single node, present root");
        expect(10, false, 2, "single node, absent below");
        expect(70, false, 2, "single node, absent above");

        // {20, 40, 60}: inserting 20 then 60 hangs them under 40 with no
        // rotation — root 40 at level 1, both children at level 2.
        seed_balance(&drive, 20, platform_version);
        seed_balance(&drive, 60, platform_version);
        expect(40, true, 1, "three nodes, root");
        expect(20, true, 2, "three nodes, left child");
        expect(60, true, 2, "three nodes, right child");
        expect(10, false, 3, "three nodes, absent below min");
        expect(30, false, 3, "three nodes, absent in left gap");
        expect(50, false, 3, "three nodes, absent in right gap");
        expect(70, false, 3, "three nodes, absent above max");

        // {10..70}: the four leaves slot under 20 and 60 with no rotation,
        // giving the complete three-level tree.
        for n in [10u8, 30, 50, 70] {
            seed_balance(&drive, n, platform_version);
        }
        expect(40, true, 1, "seven nodes, root");
        expect(20, true, 2, "seven nodes, left inner");
        expect(60, true, 2, "seven nodes, right inner");
        for n in [10u8, 30, 50, 70] {
            expect(n, true, 3, "seven nodes, leaf");
        }
        expect(5, false, 4, "seven nodes, absent below min");
        for n in [15u8, 25, 35, 45, 55, 65] {
            expect(n, false, 4, "seven nodes, absent in interior gap");
        }
        expect(75, false, 4, "seven nodes, absent above max");
    }
}

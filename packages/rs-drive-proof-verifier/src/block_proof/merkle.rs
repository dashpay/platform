//! Tenderdash's Merkle tree (RFC 6962), the tree behind a block's hash, its `data_hash`
//! and its `results_hash`.

use super::BlockProofError;
use sha2::{Digest, Sha256};

const LEAF_PREFIX: [u8; 1] = [0];
const INNER_PREFIX: [u8; 1] = [1];
const HASH_SIZE: usize = 32;

fn leaf_hash(leaf: &[u8]) -> [u8; 32] {
    Sha256::new()
        .chain_update(LEAF_PREFIX)
        .chain_update(leaf)
        .finalize()
        .into()
}

fn inner_hash(left: &[u8], right: &[u8]) -> [u8; 32] {
    Sha256::new()
        .chain_update(INNER_PREFIX)
        .chain_update(left)
        .chain_update(right)
        .finalize()
        .into()
}

/// The largest power of two below `length`, which must be at least 2.
fn split_point(length: usize) -> usize {
    1 << (usize::BITS - 1 - (length - 1).leading_zeros())
}

/// The root of the tree over `leaves`.
pub fn merkle_root<T: AsRef<[u8]>>(leaves: &[T]) -> [u8; 32] {
    match leaves {
        [] => Sha256::digest([]).into(),
        [leaf] => leaf_hash(leaf.as_ref()),
        _ => {
            let (left, right) = leaves.split_at(split_point(leaves.len()));
            inner_hash(&merkle_root(left), &merkle_root(right))
        }
    }
}

/// The tree over some leaves, with the paths of the selected ones: what a node needs to
/// prove those leaves are under the root.
#[derive(Debug)]
pub struct MerkleTree {
    /// The root of the tree.
    pub root: [u8; 32],
    /// For each selected leaf, the hashes from its sibling up to a child of the root.
    pub paths: Vec<Vec<Vec<u8>>>,
}

impl MerkleTree {
    /// Build the tree over `leaves`, keeping paths only for the positions in `selected`,
    /// which must be ascending and within `leaves`.
    pub fn new<T: AsRef<[u8]>>(leaves: &[T], selected: &[usize]) -> Self {
        Self::build(leaves, 0, selected)
    }

    fn build<T: AsRef<[u8]>>(leaves: &[T], first: usize, selected: &[usize]) -> Self {
        if selected.is_empty() {
            return Self {
                root: merkle_root(leaves),
                paths: Vec::new(),
            };
        }
        if leaves.len() < 2 {
            return Self {
                root: merkle_root(leaves),
                paths: vec![Vec::new(); selected.len()],
            };
        }

        let split = split_point(leaves.len());
        let (left_leaves, right_leaves) = leaves.split_at(split);
        let (left_selected, right_selected) =
            selected.split_at(selected.partition_point(|&position| position < first + split));
        let left = Self::build(left_leaves, first, left_selected);
        let right = Self::build(right_leaves, first + split, right_selected);

        let root = inner_hash(&left.root, &right.root);
        let paths = left
            .paths
            .into_iter()
            .map(|mut path| {
                path.push(right.root.to_vec());
                path
            })
            .chain(right.paths.into_iter().map(|mut path| {
                path.push(left.root.to_vec());
                path
            }))
            .collect();
        Self { root, paths }
    }
}

/// Follow `path` up from the leaf at `index` of a tree of `total` leaves, returning the
/// root it leads to. `path` runs from the leaf's sibling up to a child of the root.
///
/// The position decides the order each pair is hashed in, so a path leads to the tree's
/// root only from the position it was built for. `total` is the caller's claim: a path
/// can also fit a tree of another size, at another position.
pub fn root_from_path(
    index: u32,
    total: u32,
    leaf: &[u8],
    path: &[Vec<u8>],
) -> Result<[u8; 32], BlockProofError> {
    if index >= total {
        return Err(BlockProofError::IndexOutOfRange { index, total });
    }

    // Walk down from the root to the leaf, noting at each level which side the leaf is on
    let (mut position, mut size) = (index as usize, total as usize);
    let mut leaf_is_on_the_left = Vec::new();
    while size > 1 {
        let split = split_point(size);
        if position < split {
            leaf_is_on_the_left.push(true);
            size = split;
        } else {
            leaf_is_on_the_left.push(false);
            position -= split;
            size -= split;
        }
    }
    let malformed = path.len() != leaf_is_on_the_left.len()
        || path.iter().any(|sibling| sibling.len() != HASH_SIZE);
    if malformed {
        return Err(BlockProofError::MalformedPath { index, total });
    }

    // Then hash back up, the path giving the other side at each level
    let root = path.iter().zip(leaf_is_on_the_left.iter().rev()).fold(
        leaf_hash(leaf),
        |hash, (sibling, &on_the_left)| {
            if on_the_left {
                inner_hash(&hash, sibling)
            } else {
                inner_hash(sibling, &hash)
            }
        },
    );
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `merkle.ProofsFromByteSlices` of Tenderdash's `crypto/merkle` over `leaf-0`..`leaf-6`.
    const TENDERDASH_ROOT: &str =
        "0b007fb915eb9b2a146f54b1c86ec53b664f8e455b7660b0b6ee13edc0d921c0";
    const TENDERDASH_PATHS: [&[&str]; 7] = [
        &[
            "3145c409f259b7c53e32036090ff76751025a2498ba9823ef718cac50b4e616f",
            "bd45ff28796704d88bdac51b1df553fda59837b616d6d1cb2114dbc3b087ff69",
            "8eae6bd3b3a07f1f75ee72a531629e6eb31e42e62f760e47de52a53c3641ef23",
        ],
        &[
            "305df59f9590c3c9ac63d2b2743c388e3792449078cebf7fb3dbe6471643b2b7",
            "bd45ff28796704d88bdac51b1df553fda59837b616d6d1cb2114dbc3b087ff69",
            "8eae6bd3b3a07f1f75ee72a531629e6eb31e42e62f760e47de52a53c3641ef23",
        ],
        &[
            "f76836325aec5699d8d71f8e42e9d47c5c29b08059ba296384f7ca40ad3a40ae",
            "60a53eed0de87a90c8e59427c59c46253c33a76a09502a51801300927b7e6bdc",
            "8eae6bd3b3a07f1f75ee72a531629e6eb31e42e62f760e47de52a53c3641ef23",
        ],
        &[
            "fca89f57c9f8c8eb4047a7ff9d333acf9e0f3384b20b255bceab0f216dcca267",
            "60a53eed0de87a90c8e59427c59c46253c33a76a09502a51801300927b7e6bdc",
            "8eae6bd3b3a07f1f75ee72a531629e6eb31e42e62f760e47de52a53c3641ef23",
        ],
        &[
            "8f1593cb92f429d9340b9bbc1f0bb122adf8026c42a4a42142e2168931727236",
            "676f3782f5b3a5fb4370ed49572cedc523f4a66322269c85f2af0509d17b0a4d",
            "bdd1c5ff55b19cb6b0e7c761bf9a6ccaa27fbbfc07b74f1fabb6e911a0bd2ab3",
        ],
        &[
            "ea9fc1a1b6e191b460d0d6306e3e870c173f39330f13cda1b70cfc72bdc398ba",
            "676f3782f5b3a5fb4370ed49572cedc523f4a66322269c85f2af0509d17b0a4d",
            "bdd1c5ff55b19cb6b0e7c761bf9a6ccaa27fbbfc07b74f1fabb6e911a0bd2ab3",
        ],
        &[
            "985bb5d36b927800876871da925a7e82abe83a9ddba5882920a007a55ea2b376",
            "bdd1c5ff55b19cb6b0e7c761bf9a6ccaa27fbbfc07b74f1fabb6e911a0bd2ab3",
        ],
    ];

    fn tenderdash_leaves() -> Vec<Vec<u8>> {
        (0..7)
            .map(|index| format!("leaf-{index}").into_bytes())
            .collect()
    }

    fn numbered_leaves(total: usize) -> Vec<Vec<u8>> {
        (0..total)
            .map(|index| index.to_be_bytes().to_vec())
            .collect()
    }

    fn hex_path(path: &[Vec<u8>]) -> Vec<String> {
        path.iter().map(hex::encode).collect()
    }

    #[test]
    fn should_build_the_same_tree_as_tenderdash() {
        let leaves = tenderdash_leaves();

        let tree = MerkleTree::new(&leaves, &(0..leaves.len()).collect::<Vec<_>>());

        assert_eq!(hex::encode(tree.root), TENDERDASH_ROOT);
        for (path, expected) in tree.paths.iter().zip(TENDERDASH_PATHS) {
            assert_eq!(hex_path(path), expected);
        }
    }

    #[test]
    fn should_follow_the_paths_tenderdash_builds_to_its_root() {
        let leaves = tenderdash_leaves();

        for (index, (leaf, path)) in leaves.iter().zip(TENDERDASH_PATHS).enumerate() {
            let path = path
                .iter()
                .map(|hash| hex::decode(hash).unwrap())
                .collect::<Vec<_>>();

            let root = root_from_path(index as u32, 7, leaf, &path).unwrap();

            assert_eq!(hex::encode(root), TENDERDASH_ROOT, "leaf {index}");
        }
    }

    #[test]
    fn should_hash_edge_trees_like_rfc_6962() {
        assert_eq!(
            hex::encode(merkle_root::<Vec<u8>>(&[])),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex::encode(merkle_root(&[Vec::new()])),
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
        );
        assert_eq!(
            hex::encode(merkle_root(&[b"L123456".to_vec()])),
            "395aa064aa4c29f7010acfe3f25db9485bbd4b91897b6ad7ad547639252b4d56"
        );
        assert_eq!(
            hex::encode(inner_hash(b"N123", b"N456")),
            "aa217fe888e47007fa15edab33c2b492a722cb106c64667fc2b044444de66bbb"
        );
    }

    #[test]
    fn should_compute_the_same_root_without_building_paths() {
        for total in 0..=33usize {
            let leaves = numbered_leaves(total);
            let all = (0..total).collect::<Vec<_>>();

            assert_eq!(merkle_root(&leaves), MerkleTree::new(&leaves, &all).root);
        }
    }

    #[test]
    fn should_build_only_the_selected_paths() {
        let leaves = tenderdash_leaves();

        for selected in [vec![], vec![0], vec![6], vec![1, 4], vec![2, 3, 5]] {
            let tree = MerkleTree::new(&leaves, &selected);

            assert_eq!(hex::encode(tree.root), TENDERDASH_ROOT);
            assert_eq!(tree.paths.len(), selected.len());
            for (path, position) in tree.paths.iter().zip(&selected) {
                assert_eq!(hex_path(path), TENDERDASH_PATHS[*position]);
            }
        }
    }

    #[test]
    fn should_give_every_leaf_a_path_to_the_root() {
        for total in 1..=33usize {
            let leaves = numbered_leaves(total);
            let tree = MerkleTree::new(&leaves, &(0..total).collect::<Vec<_>>());

            for (index, leaf) in leaves.iter().enumerate() {
                let root =
                    root_from_path(index as u32, total as u32, leaf, &tree.paths[index]).unwrap();
                assert_eq!(root, tree.root, "leaf {index} of {total}");
            }
        }
    }

    #[test]
    fn should_not_reach_the_root_from_another_position() {
        for total in 2..=33usize {
            let leaves = numbered_leaves(total);
            let tree = MerkleTree::new(&leaves, &(0..total).collect::<Vec<_>>());

            for (index, leaf) in leaves.iter().enumerate() {
                for claimed in (0..total).filter(|&claimed| claimed != index) {
                    let root =
                        root_from_path(claimed as u32, total as u32, leaf, &tree.paths[index]);
                    assert_ne!(root.ok(), Some(tree.root), "leaf {index} as {claimed}");
                }
            }
        }
    }

    #[test]
    fn should_not_reach_the_root_from_another_leaf() {
        let leaves = tenderdash_leaves();
        let tree = MerkleTree::new(&leaves, &[3]);

        let root = root_from_path(3, 7, b"leaf-4", &tree.paths[0]).unwrap();

        assert_ne!(root, tree.root);
    }

    #[test]
    fn should_refuse_a_position_outside_the_tree() {
        assert_eq!(
            root_from_path(7, 7, b"leaf", &[]),
            Err(BlockProofError::IndexOutOfRange { index: 7, total: 7 })
        );
        assert_eq!(
            root_from_path(0, 0, b"leaf", &[]),
            Err(BlockProofError::IndexOutOfRange { index: 0, total: 0 })
        );
    }

    #[test]
    fn should_refuse_a_path_of_the_wrong_shape() {
        let leaves = tenderdash_leaves();
        let tree = MerkleTree::new(&leaves, &[2]);
        let path = &tree.paths[0];
        let malformed = Err(BlockProofError::MalformedPath { index: 2, total: 7 });

        // too short, too long, and a sibling that is not a hash
        assert_eq!(root_from_path(2, 7, b"leaf-2", &path[1..]), malformed);
        let mut longer = path.clone();
        longer.push(vec![0; 32]);
        assert_eq!(root_from_path(2, 7, b"leaf-2", &longer), malformed);
        let mut truncated = path.clone();
        truncated[0].pop();
        assert_eq!(root_from_path(2, 7, b"leaf-2", &truncated), malformed);

        // a single leaf is its own root and has no path
        assert_eq!(
            root_from_path(0, 1, b"leaf", &[vec![0; 32]]),
            Err(BlockProofError::MalformedPath { index: 0, total: 1 })
        );
        assert_eq!(root_from_path(0, 1, b"leaf", &[]), Ok(leaf_hash(b"leaf")));
    }
}

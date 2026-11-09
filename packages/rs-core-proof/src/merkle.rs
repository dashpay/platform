//! Core's positional Merkle paths (`ProofMerklePath`).
use crate::wire::{sha256d, Hash, Reader, Result};
use crate::Error;

/// Most leaves a path may claim; Core uses the same bound.
const MAX_LEAVES: u32 = 100_000;
/// Most siblings a path may carry, enough for [`MAX_LEAVES`].
const MAX_DEPTH: usize = 17;

/// The parent of two sibling nodes.
fn node(left: &Hash, right: &Hash) -> Hash {
    let mut pair = [0; 64];
    pair[..32].copy_from_slice(left);
    pair[32..].copy_from_slice(right);
    sha256d(&pair)
}

/// The siblings from a leaf up to a Merkle root, one per level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerklePath {
    /// Position of the leaf.
    pub index: u32,
    /// Number of leaves in the tree.
    pub count: u32,
    /// One sibling per level. Where a level has an odd width and the leaf is
    /// last, the sibling is the node itself, as in Bitcoin's Merkle trees.
    pub siblings: Vec<Hash>,
}

impl MerklePath {
    /// Check that `leaf` sits at `index` of a `count`-leaf tree with `root`.
    ///
    /// A sibling equal to the node is accepted only where the tree duplicates
    /// that node, so a path cannot use Bitcoin's duplicate-leaf ambiguity to
    /// prove a leaf at a second position.
    pub fn verify(&self, mut leaf: Hash, root: &Hash) -> Result<()> {
        if self.count == 0 || self.count > MAX_LEAVES || self.index >= self.count {
            return Err(Error("Merkle position"));
        }
        let mut width = self.count;
        let mut index = self.index;
        let mut siblings = self.siblings.iter();
        while width > 1 {
            let sibling = siblings.next().ok_or(Error("short Merkle path"))?;
            let duplicated = index ^ 1 >= width;
            if duplicated != (*sibling == leaf) {
                return Err(Error("mutated Merkle path"));
            }
            leaf = if index & 1 == 0 {
                node(&leaf, sibling)
            } else {
                node(sibling, &leaf)
            };
            index /= 2;
            width = width.div_ceil(2);
        }
        if siblings.next().is_some() || leaf != *root {
            return Err(Error("Merkle root/path"));
        }
        Ok(())
    }

    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let index = r.u32()?;
        let count = r.u32()?;
        let depth = r.u8()? as usize;
        if depth > MAX_DEPTH {
            return Err(Error("Merkle depth limit"));
        }
        let siblings = (0..depth).map(|_| r.array()).collect::<Result<_>>()?;
        Ok(Self {
            index,
            count,
            siblings,
        })
    }

    pub(crate) fn encode(&self, out: &mut Vec<u8>) -> Result<()> {
        if self.siblings.len() > MAX_DEPTH {
            return Err(Error("Merkle depth limit"));
        }
        out.extend(self.index.to_le_bytes());
        out.extend(self.count.to_le_bytes());
        out.push(self.siblings.len() as u8);
        for sibling in &self.siblings {
            out.extend(sibling);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_accept_duplicated_last_leaf_only_at_its_own_position() {
        let (a, b, c) = ([1; 32], [2; 32], [3; 32]);
        let root = node(&node(&a, &b), &node(&c, &c));
        let last = MerklePath {
            index: 2,
            count: 3,
            siblings: vec![c, node(&a, &b)],
        };
        assert_eq!(last.verify(c, &root), Ok(()));
        // The same leaf cannot be proven at the phantom fourth position.
        let phantom = MerklePath {
            index: 3,
            count: 3,
            siblings: vec![c, node(&a, &b)],
        };
        assert!(phantom.verify(c, &root).is_err());
        // A four-leaf claim over the same root is rejected because the
        // sibling equals the node where the tree does not duplicate.
        let four = MerklePath {
            index: 2,
            count: 4,
            siblings: vec![c, node(&a, &b)],
        };
        assert!(four.verify(c, &root).is_err());
    }

    #[test]
    fn should_reject_paths_with_extra_or_missing_siblings() {
        let (a, b) = ([1; 32], [2; 32]);
        let root = node(&a, &b);
        let path = |siblings| MerklePath {
            index: 0,
            count: 2,
            siblings,
        };
        assert_eq!(path(vec![b]).verify(a, &root), Ok(()));
        assert!(path(vec![]).verify(a, &root).is_err());
        assert!(path(vec![b, b]).verify(a, &root).is_err());
        let single = MerklePath {
            index: 0,
            count: 1,
            siblings: vec![],
        };
        assert_eq!(single.verify(a, &a), Ok(()));
    }

    #[test]
    fn should_reject_out_of_range_positions() {
        let path = |index, count| MerklePath {
            index,
            count,
            siblings: vec![],
        };
        assert!(path(0, 0).verify([0; 32], &[0; 32]).is_err());
        assert!(path(1, 1).verify([0; 32], &[0; 32]).is_err());
        assert!(path(0, MAX_LEAVES + 1).verify([0; 32], &[0; 32]).is_err());
    }
}

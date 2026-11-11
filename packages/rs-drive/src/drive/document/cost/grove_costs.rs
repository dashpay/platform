//! The storage bytes GroveDB charges for one new element, restated from
//! grovedb-merk, whose formulas are compiled only with its RocksDB storage
//! (`minimal`), so the `verify` build can price an insert too. The tests pin
//! every restated number to grovedb's own functions.
//!
//! A new element costs its key (the 32-byte subtree prefix plus the key, with
//! a length varint) and its value: the serialized element (or, for trees and
//! sum items, a fixed size standing for it), the value and node hashes, the
//! node's aggregate feature, and the link its parent keeps to it.

use grovedb_merk::tree_type::TreeType;
use integer_encoding::VarInt;

const HASH_LENGTH: u32 = 32;

/// The aggregate a Merk node carries, set by the tree the node lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Normal,
    Sum,
    BigSum,
    Count,
    CountSum,
    ProvableCount,
    ProvableCountSum,
    ProvableSum,
    ProvableCountProvableSum,
}

impl NodeKind {
    /// The kind of the nodes of a tree of `tree_type` (`TreeType::inner_node_type`).
    pub(crate) fn of_tree(tree_type: TreeType) -> Self {
        match tree_type {
            TreeType::NormalTree
            | TreeType::CommitmentTree(_)
            | TreeType::MmrTree
            | TreeType::BulkAppendTree(_)
            | TreeType::DenseAppendOnlyFixedSizeTree(_)
            | TreeType::PrivateDocumentStore(_) => NodeKind::Normal,
            TreeType::SumTree => NodeKind::Sum,
            TreeType::BigSumTree => NodeKind::BigSum,
            TreeType::CountTree => NodeKind::Count,
            TreeType::CountSumTree => NodeKind::CountSum,
            TreeType::ProvableCountTree | TreeType::ProvableCountIndexedTree => {
                NodeKind::ProvableCount
            }
            TreeType::ProvableCountSumTree => NodeKind::ProvableCountSum,
            TreeType::ProvableSumTree | TreeType::ProvableSumIndexedTree => NodeKind::ProvableSum,
            TreeType::ProvableCountProvableSumTree
            | TreeType::ProvableCountProvableSumIndexedTree => NodeKind::ProvableCountProvableSum,
        }
    }

    /// The feature bytes a node stores (`NodeType::feature_len`).
    fn feature_len(self) -> u32 {
        match self {
            NodeKind::Normal => 1,
            NodeKind::Sum | NodeKind::Count | NodeKind::ProvableCount | NodeKind::ProvableSum => 9,
            NodeKind::BigSum
            | NodeKind::CountSum
            | NodeKind::ProvableCountSum
            | NodeKind::ProvableCountProvableSum => 17,
        }
    }

    /// The aggregate bytes a parent's link to the node carries (`NodeType::cost`).
    fn link_aggregate_len(self) -> u32 {
        self.feature_len() - 1
    }
}

fn varint_len(value: u32) -> u32 {
    value.required_space() as u32
}

/// The link a parent node keeps to a child keyed by `key_len` bytes
/// (`Link::encoded_link_size`).
fn link_len(key_len: u32, node: NodeKind) -> u32 {
    key_len + HASH_LENGTH + 4 + node.link_aggregate_len()
}

/// The key bytes of a new node (`KV::node_key_byte_cost_size`).
pub(crate) fn key_bytes(key_len: u32) -> u32 {
    HASH_LENGTH + key_len + varint_len(key_len + HASH_LENGTH)
}

/// The value bytes of a node whose value hash it pays for itself: an item,
/// a reference or a sum item (`KV::node_value_byte_cost_size`).
fn node_value_bytes(key_len: u32, raw_value_len: u32, node: NodeKind) -> u32 {
    let value_size = raw_value_len + 2 * HASH_LENGTH + node.feature_len();
    value_size + varint_len(value_size) + link_len(key_len, node)
}

/// The value bytes of a tree element, whose value hash the root of its own
/// Merk pays for (`KV::layered_value_byte_cost_size_for_key_and_value_lengths`).
fn layered_value_bytes(key_len: u32, value_len: u32, node: NodeKind) -> u32 {
    value_len + node.feature_len() + HASH_LENGTH + 2 + link_len(key_len, node)
}

/// The fixed size standing for a tree element of `tree_type`
/// (grovedb-merk `tree_type::costs`).
fn tree_cost_size(tree_type: TreeType) -> u32 {
    match tree_type {
        TreeType::NormalTree => 3,
        TreeType::SumTree
        | TreeType::CountTree
        | TreeType::ProvableCountTree
        | TreeType::ProvableSumTree => 12,
        TreeType::BigSumTree => 19,
        TreeType::CountSumTree
        | TreeType::ProvableCountSumTree
        | TreeType::ProvableCountProvableSumTree => 21,
        TreeType::ProvableCountIndexedTree | TreeType::ProvableSumIndexedTree => 13,
        TreeType::ProvableCountProvableSumIndexedTree => 28,
        TreeType::CommitmentTree(_) | TreeType::BulkAppendTree(_) => 12,
        TreeType::MmrTree => 11,
        TreeType::DenseAppendOnlyFixedSizeTree(_) => 6,
        TreeType::PrivateDocumentStore(_) => 17,
    }
}

/// The bytes a flags field of `flags_len` bytes adds to a tree or sum item.
fn flags_bytes(flags_len: Option<u32>) -> u32 {
    flags_len.map_or(0, |len| len + varint_len(len))
}

/// The element GroveDB prices, as far as its storage cost goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PricedElement {
    /// An empty tree of `tree_type`, `wrapped` in a zero-contribution
    /// wrapper or not, with flags of `flags_len` bytes.
    Tree {
        tree_type: TreeType,
        wrapped: bool,
        flags_len: Option<u32>,
    },
    /// An item or reference (with or without a sum) whose serialized element
    /// is `serialized_len` bytes.
    Serialized { serialized_len: u32 },
    /// An item that also carries a sum, holding `item_len` bytes, with flags
    /// of `flags_len` bytes.
    ItemWithSumItem {
        item_len: u32,
        flags_len: Option<u32>,
    },
    /// A sum item (a `summableOffCountIndex` index's counter), charged its
    /// fixed size whatever its value, with flags of `flags_len` bytes.
    SumItem { flags_len: Option<u32> },
}

/// The storage bytes GroveDB adds for a new `element` under a key of
/// `key_len` bytes in a tree whose nodes are `node` nodes.
pub(crate) fn new_element_bytes(key_len: u32, element: PricedElement, node: NodeKind) -> u32 {
    let value = match element {
        PricedElement::Tree {
            tree_type,
            wrapped,
            flags_len,
        } => layered_value_bytes(
            key_len,
            tree_cost_size(tree_type) + flags_bytes(flags_len) + u32::from(wrapped),
            node,
        ),
        PricedElement::Serialized { serialized_len } => {
            node_value_bytes(key_len, serialized_len, node)
        }
        PricedElement::ItemWithSumItem {
            item_len,
            flags_len,
        } => node_value_bytes(
            key_len,
            item_len + varint_len(item_len) + 11 + flags_bytes(flags_len),
            node,
        ),
        PricedElement::SumItem { flags_len } => {
            node_value_bytes(key_len, 11 + flags_bytes(flags_len), node)
        }
    };
    key_bytes(key_len) + value
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use grovedb::Element;
    use grovedb_merk::element::costs::ElementCostExtensions;
    use grovedb_merk::tree::kv::KV;
    use grovedb_version::version::GroveVersion;

    const TREE_TYPES: [TreeType; 12] = [
        TreeType::NormalTree,
        TreeType::SumTree,
        TreeType::BigSumTree,
        TreeType::CountTree,
        TreeType::CountSumTree,
        TreeType::ProvableCountTree,
        TreeType::ProvableCountSumTree,
        TreeType::ProvableSumTree,
        TreeType::ProvableCountProvableSumTree,
        TreeType::ProvableSumIndexedTree,
        TreeType::ProvableCountIndexedTree,
        TreeType::ProvableCountProvableSumIndexedTree,
    ];

    #[test]
    fn should_give_each_tree_the_node_kind_grovedb_gives_it() {
        for tree_type in TREE_TYPES {
            let grovedb = tree_type.inner_node_type();
            let ours = NodeKind::of_tree(tree_type);
            assert_eq!(ours.feature_len(), grovedb.feature_len(), "{tree_type:?}");
            assert_eq!(ours.link_aggregate_len(), grovedb.cost(), "{tree_type:?}");
        }
    }

    #[test]
    fn should_price_keys_and_values_as_grovedb_does() {
        for tree_type in TREE_TYPES {
            let node = NodeKind::of_tree(tree_type);
            let grovedb_node = tree_type.inner_node_type();
            for key_len in [0u32, 1, 8, 32, 95, 96, 200, 255] {
                assert_eq!(key_bytes(key_len), KV::node_key_byte_cost_size(key_len));
                for raw in [0u32, 10, 63, 64, 100, 127, 128, 1000, 16_500] {
                    assert_eq!(
                        node_value_bytes(key_len, raw, node),
                        KV::node_value_byte_cost_size(key_len, raw, grovedb_node),
                        "{tree_type:?} key {key_len} raw {raw}"
                    );
                    assert_eq!(
                        layered_value_bytes(key_len, raw, node),
                        KV::layered_value_byte_cost_size_for_key_and_value_lengths(
                            key_len,
                            raw,
                            grovedb_node
                        ),
                        "{tree_type:?} key {key_len} raw {raw}"
                    );
                }
            }
        }
    }

    #[test]
    fn should_price_tree_elements_as_grovedb_does() {
        let grove_version = GroveVersion::latest();
        let flags = Some(vec![7u8; 35]);
        for parent in TREE_TYPES {
            for key in [vec![1u8; 1], vec![2u8; 32], vec![3u8; 60]] {
                for (tree_type, element) in [
                    (
                        TreeType::NormalTree,
                        Element::empty_tree_with_flags(flags.clone()),
                    ),
                    (
                        TreeType::SumTree,
                        Element::empty_sum_tree_with_flags(flags.clone()),
                    ),
                    (
                        TreeType::CountTree,
                        Element::empty_count_tree_with_flags(flags.clone()),
                    ),
                    (
                        TreeType::CountSumTree,
                        Element::empty_count_sum_tree_with_flags(flags.clone()),
                    ),
                    (
                        TreeType::ProvableCountTree,
                        Element::empty_provable_count_tree_with_flags(flags.clone()),
                    ),
                    (TreeType::NormalTree, Element::empty_tree_with_flags(None)),
                ] {
                    let serialized = element.serialize(grove_version).expect("serialize");
                    let grovedb = Element::specialized_costs_for_key_value(
                        &key,
                        &serialized,
                        parent.inner_node_type(),
                        grove_version,
                    )
                    .expect("cost");
                    let flags_len = match &element {
                        Element::Tree(_, f)
                        | Element::SumTree(_, _, f)
                        | Element::CountTree(_, _, f)
                        | Element::CountSumTree(_, _, _, f)
                        | Element::ProvableCountTree(_, _, f) => f.as_ref().map(|f| f.len() as u32),
                        _ => unreachable!(),
                    };
                    let ours = new_element_bytes(
                        key.len() as u32,
                        PricedElement::Tree {
                            tree_type,
                            wrapped: false,
                            flags_len,
                        },
                        NodeKind::of_tree(parent),
                    ) - key_bytes(key.len() as u32);
                    assert_eq!(ours, grovedb, "{tree_type:?} in {parent:?}");

                    let wrapped = Element::NonCounted(Box::new(element.clone()))
                        .serialize(grove_version)
                        .expect("serialize");
                    let grovedb_wrapped = Element::specialized_costs_for_key_value(
                        &key,
                        &wrapped,
                        parent.inner_node_type(),
                        grove_version,
                    )
                    .expect("cost");
                    let ours_wrapped = new_element_bytes(
                        key.len() as u32,
                        PricedElement::Tree {
                            tree_type,
                            wrapped: true,
                            flags_len,
                        },
                        NodeKind::of_tree(parent),
                    ) - key_bytes(key.len() as u32);
                    assert_eq!(ours_wrapped, grovedb_wrapped, "wrapped {tree_type:?}");
                }
            }
        }
    }

    #[test]
    fn should_price_items_and_sum_items_as_grovedb_does() {
        let grove_version = GroveVersion::latest();
        for parent in TREE_TYPES {
            let node = NodeKind::of_tree(parent);
            for len in [0usize, 5, 100, 300, 5000] {
                let item = Element::new_item_with_flags(vec![1; len], Some(vec![2; 35]));
                let serialized = item.serialize(grove_version).expect("serialize");
                let grovedb = Element::specialized_costs_for_key_value(
                    &[9; 32],
                    &serialized,
                    parent.inner_node_type(),
                    grove_version,
                )
                .expect("cost");
                assert_eq!(
                    new_element_bytes(
                        32,
                        PricedElement::Serialized {
                            serialized_len: serialized.len() as u32
                        },
                        node
                    ) - key_bytes(32),
                    grovedb
                );

                let with_sum =
                    Element::new_item_with_sum_item_with_flags(vec![1; len], 42, Some(vec![2; 35]));
                let serialized = with_sum.serialize(grove_version).expect("serialize");
                let grovedb = Element::specialized_costs_for_key_value(
                    &[9; 32],
                    &serialized,
                    parent.inner_node_type(),
                    grove_version,
                )
                .expect("cost");
                assert_eq!(
                    new_element_bytes(
                        32,
                        PricedElement::ItemWithSumItem {
                            item_len: len as u32,
                            flags_len: Some(35)
                        },
                        node
                    ) - key_bytes(32),
                    grovedb
                );
            }
        }
    }
}

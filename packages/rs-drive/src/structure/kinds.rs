use grovedb::Element;
use serde::Serialize;

/// The kind of a GroveDB element, without its contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum ElementKind {
    /// An ordinary value
    Item,
    /// A reference to another element by path
    Reference,
    /// A subtree
    Tree,
    /// A signed integer summed by a parent sum tree
    SumItem,
    /// A subtree summing its sum items
    SumTree,
    /// A sum tree with an i128 total
    BigSumTree,
    /// A subtree counting its elements
    CountTree,
    /// A subtree counting and summing
    CountSumTree,
    /// A count tree whose count is part of the hashed state
    ProvableCountTree,
    /// A value that also carries a sum
    ItemWithSumItem,
    /// A count sum tree whose count is part of the hashed state
    ProvableCountSumTree,
    /// An Orchard style note commitment tree
    CommitmentTree,
    /// A Merkle mountain range
    MmrTree,
    /// A bulk append tree
    BulkAppendTree,
    /// A dense append only tree of fixed size
    DenseAppendOnlyFixedSizeTree,
    /// A reference that also carries a sum
    ReferenceWithSumItem,
    /// A sum tree whose sum is part of the hashed state
    ProvableSumTree,
    /// A count sum tree whose count and sum are part of the hashed state
    ProvableCountProvableSumTree,
    /// A provable sum tree with a secondary index
    ProvableSumIndexedTree,
    /// A provable count tree with a secondary index
    ProvableCountIndexedTree,
    /// A provable count and sum tree with secondary indexes
    ProvableCountProvableSumIndexedTree,
    /// A private document store
    PrivateDocumentStore,
    /// A reference kept in sync with its target
    BidirectionalReference,
    /// An item that records the references pointing at it
    ItemWithBackwardsReferences,
    /// A sum item that records the references pointing at it
    SumItemWithBackwardsReferences,
    /// An item with a sum that records the references pointing at it
    ItemWithSumItemWithBackwardsReferences,
}

impl ElementKind {
    /// Every kind, in declaration order
    pub const ALL: [ElementKind; 26] = [
        ElementKind::Item,
        ElementKind::Reference,
        ElementKind::Tree,
        ElementKind::SumItem,
        ElementKind::SumTree,
        ElementKind::BigSumTree,
        ElementKind::CountTree,
        ElementKind::CountSumTree,
        ElementKind::ProvableCountTree,
        ElementKind::ItemWithSumItem,
        ElementKind::ProvableCountSumTree,
        ElementKind::CommitmentTree,
        ElementKind::MmrTree,
        ElementKind::BulkAppendTree,
        ElementKind::DenseAppendOnlyFixedSizeTree,
        ElementKind::ReferenceWithSumItem,
        ElementKind::ProvableSumTree,
        ElementKind::ProvableCountProvableSumTree,
        ElementKind::ProvableSumIndexedTree,
        ElementKind::ProvableCountIndexedTree,
        ElementKind::ProvableCountProvableSumIndexedTree,
        ElementKind::PrivateDocumentStore,
        ElementKind::BidirectionalReference,
        ElementKind::ItemWithBackwardsReferences,
        ElementKind::SumItemWithBackwardsReferences,
        ElementKind::ItemWithSumItemWithBackwardsReferences,
    ];

    /// The kind of an element. Looks through the `NonCounted`, `NotSummed`
    /// and `NotCountedOrSummed` wrappers.
    ///
    /// The match has no wildcard arm on purpose: a GroveDB upgrade that adds
    /// an element variant must decide here how the structure describes it.
    pub fn of(element: &Element) -> ElementKind {
        match element {
            Element::Item(..) => ElementKind::Item,
            Element::Reference(..) => ElementKind::Reference,
            Element::Tree(..) => ElementKind::Tree,
            Element::SumItem(..) => ElementKind::SumItem,
            Element::SumTree(..) => ElementKind::SumTree,
            Element::BigSumTree(..) => ElementKind::BigSumTree,
            Element::CountTree(..) => ElementKind::CountTree,
            Element::CountSumTree(..) => ElementKind::CountSumTree,
            Element::ProvableCountTree(..) => ElementKind::ProvableCountTree,
            Element::ItemWithSumItem(..) => ElementKind::ItemWithSumItem,
            Element::ProvableCountSumTree(..) => ElementKind::ProvableCountSumTree,
            Element::CommitmentTree(..) => ElementKind::CommitmentTree,
            Element::MmrTree(..) => ElementKind::MmrTree,
            Element::BulkAppendTree(..) => ElementKind::BulkAppendTree,
            Element::DenseAppendOnlyFixedSizeTree(..) => ElementKind::DenseAppendOnlyFixedSizeTree,
            Element::NonCounted(inner)
            | Element::NotSummed(inner)
            | Element::NotCountedOrSummed(inner) => ElementKind::of(inner),
            Element::ReferenceWithSumItem(..) => ElementKind::ReferenceWithSumItem,
            Element::ProvableSumTree(..) => ElementKind::ProvableSumTree,
            Element::ProvableCountProvableSumTree(..) => ElementKind::ProvableCountProvableSumTree,
            Element::ProvableSumIndexedTree(..) => ElementKind::ProvableSumIndexedTree,
            Element::ProvableCountIndexedTree(..) => ElementKind::ProvableCountIndexedTree,
            Element::ProvableCountProvableSumIndexedTree(..) => {
                ElementKind::ProvableCountProvableSumIndexedTree
            }
            Element::PrivateDocumentStore(..) => ElementKind::PrivateDocumentStore,
            Element::BidirectionalReference(..) => ElementKind::BidirectionalReference,
            Element::ItemWithBackwardsReferences(..) => ElementKind::ItemWithBackwardsReferences,
            Element::SumItemWithBackwardsReferences(..) => {
                ElementKind::SumItemWithBackwardsReferences
            }
            Element::ItemWithSumItemWithBackwardsReferences(..) => {
                ElementKind::ItemWithSumItemWithBackwardsReferences
            }
        }
    }

    /// Whether elements of this kind hold a layer below them
    pub fn is_tree(&self) -> bool {
        match self {
            ElementKind::Tree
            | ElementKind::SumTree
            | ElementKind::BigSumTree
            | ElementKind::CountTree
            | ElementKind::CountSumTree
            | ElementKind::ProvableCountTree
            | ElementKind::ProvableCountSumTree
            | ElementKind::CommitmentTree
            | ElementKind::MmrTree
            | ElementKind::BulkAppendTree
            | ElementKind::DenseAppendOnlyFixedSizeTree
            | ElementKind::ProvableSumTree
            | ElementKind::ProvableCountProvableSumTree
            | ElementKind::ProvableSumIndexedTree
            | ElementKind::ProvableCountIndexedTree
            | ElementKind::ProvableCountProvableSumIndexedTree
            | ElementKind::PrivateDocumentStore => true,
            ElementKind::Item
            | ElementKind::Reference
            | ElementKind::SumItem
            | ElementKind::ItemWithSumItem
            | ElementKind::ReferenceWithSumItem
            | ElementKind::BidirectionalReference
            | ElementKind::ItemWithBackwardsReferences
            | ElementKind::SumItemWithBackwardsReferences
            | ElementKind::ItemWithSumItemWithBackwardsReferences => false,
        }
    }

    /// Whether the layer below is something other than a Merk of elements,
    /// so it cannot be walked. Mirrors `Element::uses_non_merk_data_storage`.
    pub fn is_opaque(&self) -> bool {
        matches!(
            self,
            ElementKind::CommitmentTree
                | ElementKind::MmrTree
                | ElementKind::BulkAppendTree
                | ElementKind::DenseAppendOnlyFixedSizeTree
                | ElementKind::PrivateDocumentStore
        )
    }

    /// Whether the element points at another element
    pub fn is_reference(&self) -> bool {
        matches!(
            self,
            ElementKind::Reference
                | ElementKind::ReferenceWithSumItem
                | ElementKind::BidirectionalReference
        )
    }
}

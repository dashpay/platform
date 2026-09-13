//! Typed specialized collections: the manifest slot for native tree families
//! that ordinary document schemas do not expose.
//!
//! Only the declaration shape is specified here. Operation sets, limits, the
//! privacy model and the native adapters are separate capability work; every
//! kind is [`CapabilityStatus::PendingNative`](super::CapabilityStatus).
//! There is no raw path, raw element or database handle anywhere in this
//! model: a typed collection is a declared capability with a key and element
//! type, not an escape hatch.

use core::fmt;

use super::entry::ValueType;
use super::DeclarationOrigin;
use crate::identity::CollectionName;

/// The native tree family behind a typed collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypedCollectionKind {
    /// Sum tree.
    Sum,
    /// Big sum tree (128-bit totals).
    BigSum,
    /// Count tree.
    Count,
    /// Count and sum tree.
    CountSum,
    /// Provable sum tree (range sums with proofs).
    ProvableSum,
    /// Provable count tree (range counts with proofs).
    ProvableCount,
    /// Ranked tree.
    Ranked,
    /// Append-only (Merkle mountain range) collection.
    Append,
    /// Commitment collection.
    Commitment,
}

impl TypedCollectionKind {
    /// Every kind in the catalogue.
    pub const ALL: &'static [TypedCollectionKind] = &[
        TypedCollectionKind::Sum,
        TypedCollectionKind::BigSum,
        TypedCollectionKind::Count,
        TypedCollectionKind::CountSum,
        TypedCollectionKind::ProvableSum,
        TypedCollectionKind::ProvableCount,
        TypedCollectionKind::Ranked,
        TypedCollectionKind::Append,
        TypedCollectionKind::Commitment,
    ];
}

impl fmt::Display for TypedCollectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TypedCollectionKind::Sum => "sum",
            TypedCollectionKind::BigSum => "big sum",
            TypedCollectionKind::Count => "count",
            TypedCollectionKind::CountSum => "count and sum",
            TypedCollectionKind::ProvableSum => "provable sum",
            TypedCollectionKind::ProvableCount => "provable count",
            TypedCollectionKind::Ranked => "ranked",
            TypedCollectionKind::Append => "append",
            TypedCollectionKind::Commitment => "commitment",
        })
    }
}

/// A typed specialized collection declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedCollectionSpec {
    /// Where the spec came from.
    pub origin: DeclarationOrigin,
    /// The collection's identity, in the same namespace as document
    /// collections.
    pub id: CollectionName,
    /// The tree family.
    pub kind: TypedCollectionKind,
    /// The key type.
    pub key: ValueType,
    /// The element type.
    pub element: ValueType,
    /// Upper bound on the number of elements, when declared.
    pub max_elements: Option<u64>,
}

impl TypedCollectionSpec {
    /// A builder-declared typed collection.
    pub fn new(
        id: CollectionName,
        kind: TypedCollectionKind,
        key: ValueType,
        element: ValueType,
    ) -> Self {
        TypedCollectionSpec {
            origin: DeclarationOrigin::Builder,
            id,
            kind,
            key,
            element,
            max_elements: None,
        }
    }

    /// Records the origin.
    pub fn with_origin(mut self, origin: DeclarationOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Bounds the element count.
    pub fn max_elements(mut self, max_elements: u64) -> Self {
        self.max_elements = Some(max_elements);
        self
    }
}

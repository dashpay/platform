use crate::data_contract::document_type::action_fees::DocumentActionFees;
use std::collections::BTreeSet;

/// Trait providing getters for DocumentTypeV2-specific fields.
pub trait DocumentTypeV2Getters {
    /// Returns whether documents of this type are countable.
    /// When true, the primary key tree uses a CountTree enabling O(1) total document count queries.
    fn documents_countable(&self) -> bool;

    /// Returns whether this document type supports range countable.
    /// When true, the primary key tree uses a ProvableCountTree.
    /// Implies documents_countable = true.
    fn range_countable(&self) -> bool;

    /// Returns the name of the integer property whose values are summed into
    /// the primary-key tree's running aggregate, or `None` if this document
    /// type doesn't opt into sum-tree behavior. When `Some`, the primary-key
    /// tree is a `SumTree` (or `ProvableSumTree` if [`Self::range_summable`]
    /// is also true). The doctype-level total-sum fast path reads the root
    /// aggregate in **O(1)**; per-key range sums via `AggregateSumOnRange`
    /// require [`Self::range_summable`] = true and run in **O(log n)** over
    /// the in-range merk descent — both surfaced through the `GetDocumentsSum`
    /// endpoint.
    fn documents_summable(&self) -> Option<&str>;

    /// Returns whether this document type supports range summable. When
    /// true, the primary-key sum tree is a `ProvableSumTree` (per-node
    /// aggregated sums). Implies [`Self::documents_summable`] is `Some`.
    fn range_summable(&self) -> bool;

    /// Returns whether this document type is **indexOnly**: documents are
    /// never written to primary storage — the index entries are the rows,
    /// each terminating in an `Item` keyed by the index's `terminal`
    /// property. Only what is in the indexes exists and is recoverable.
    fn index_only(&self) -> bool;

    /// The top-level properties frozen at document creation on a mutable
    /// document type (the `immutable` keyword, protocol version 14). A
    /// replace that changes, adds or removes any of them is rejected with
    /// `DocumentImmutablePropertyChangedError`. Empty on document types
    /// that predate the keyword and on types whose documents are not
    /// mutable, where every property is already immutable.
    fn immutable_fields(&self) -> &BTreeSet<String>;

    /// The subset of [`Self::immutable_fields`] a replace may still set while
    /// the stored document has no value for them (the
    /// `immutableAllowSetting` keyword, protocol version 14). Once present
    /// they are frozen like the rest of the list. Always a subset of
    /// [`Self::immutable_fields`]; empty on document types that predate the
    /// keyword.
    fn immutable_fields_allow_setting(&self) -> &BTreeSet<String>;

    /// The fixed fees in credits this document type charges for actions on its documents
    /// (the `actionFees` keyword, protocol version 14). `None` on document types that
    /// declare none and on those that predate the keyword.
    fn action_fees(&self) -> Option<&DocumentActionFees>;
}

/// Trait providing setters for DocumentTypeV2-specific fields.
pub trait DocumentTypeV2Setters {
    /// Sets whether documents of this type are countable.
    fn set_documents_countable(&mut self, countable: bool);

    /// Sets whether this document type supports range countable.
    fn set_range_countable(&mut self, range_countable: bool);

    /// Sets the integer property whose values feed the primary-key sum
    /// tree. Pass `None` to disable sum-tree behavior; setting `None`
    /// also clears `range_summable` (preserving the invariant
    /// "range_summable implies documents_summable.is_some()").
    fn set_documents_summable(&mut self, property: Option<String>);

    /// Sets whether this document type supports range summable.
    /// Setting `true` requires [`Self::documents_summable`] to already
    /// be set to `Some(_)`; setters MAY enforce this by panicking or by
    /// silently no-op'ing — refer to the impl docs.
    fn set_range_summable(&mut self, range_summable: bool);
}

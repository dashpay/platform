use crate::data_contract::config::moderation::SettledDeletionRule;
use crate::data_contract::document_type::action_fees::DocumentActionFees;
use crate::data_contract::document_type::index::DerivedIndexProperty;
use crate::data_contract::document_type::property::{
    DocumentPropertyReferenceTarget, DocumentReferenceKind, GeneratedFrom,
};
use crate::data_contract::document_type::property_constraints::PropertyConstraint;
use std::collections::{BTreeMap, BTreeSet};

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
    /// On an indexOnly type, the top-level properties stored in every entry's
    /// value after the row commitment (`entryPayload`), in name order; empty
    /// elsewhere.
    fn entry_payload(&self) -> &BTreeSet<String>;

    /// Whether the documents of this type leave state only when a `refersTo` with
    /// `consume` deletes them (`canBeDeleted: "onlyWhenConsumed"`, protocol version 14):
    /// their owner can not delete one (`documents_can_be_deleted` is false), a create
    /// consuming it can. False on document types that predate the value.
    fn documents_deleted_only_when_consumed(&self) -> bool;

    /// Returns whether the contract's moderators may delete documents of this
    /// type (`moderatorAbilities.delete`, protocol version 14). Independent of
    /// `documents_can_be_deleted`, which rules what a document's own owner may
    /// do. False on document types that predate the keyword.
    fn documents_can_be_deleted_by_moderators(&self) -> bool;

    /// For how many seconds after a document's last modification (`$updatedAt`,
    /// or `$createdAt` on a type that carries no `$updatedAt`)
    /// the moderators may still delete it (`moderatorAbilities.deleteWithin`,
    /// protocol version 14). `None` means no limit, and is what every document
    /// type that predates the keyword answers.
    fn documents_can_be_deleted_by_moderators_for(&self) -> Option<u32>;

    /// Whether a moderator's deletion of a document of this type leaves a removal record
    /// under the contract (`moderatorAbilities.deleteKeepsRecord`, protocol version 14): the
    /// type then has a removal records tree, and a deleted document can be restored. False on
    /// a type whose documents moderators can not delete, and on those that predate the keyword.
    fn moderator_deletions_keep_records(&self) -> bool;

    /// Whether the owner of a document of this type a moderator deletes is refunded its
    /// storage (`moderatorAbilities.deleteRefundsOwner`, protocol version 14). False, the owner
    /// forfeiting it, when left out, on a type whose documents moderators can not delete, and
    /// on those that predate the keyword.
    fn moderator_deletions_refund_owner(&self) -> bool;

    /// Who must approve a moderator's deletion of a document of this type once it is settled,
    /// past its `moderatorAbilities.deleteWithin` window (`moderatorAbilities.deleteSettled`,
    /// protocol version 14). `None` when no moderator deletes a settled document, and on every
    /// type that predates the keyword.
    fn moderator_settled_deletion(&self) -> Option<SettledDeletionRule>;

    /// The property paths whose values a moderator's removal record of a document of this
    /// type keeps, copied from the document as it was deleted
    /// (`moderatorAbilities.deleteKeepsFields`, protocol version 14): what of it stays public
    /// once it is gone. Empty on a type whose deletions keep no record or list none, and on
    /// those that predate the keyword.
    fn moderator_deletion_kept_fields(&self) -> &BTreeSet<String>;

    /// The top-level properties only the contract's moderators write
    /// (`moderatorAbilities.changeFields`, protocol version 14): a moderator
    /// changes them with a `ContractUserModeration` transition, and a document's
    /// owner sets, changes or removes them in a create or a replace only when it
    /// moderates the contract. Empty on document types that list none and on
    /// those that predate the keyword.
    fn moderator_changeable_fields(&self) -> &BTreeSet<String>;

    /// How many seconds after its creation (`$createdAt`) the platform deletes each
    /// document of the type (the `ttl` keyword, protocol version 14). `None` means the
    /// documents live until someone deletes them, and is what every document type that
    /// predates the keyword answers.
    fn documents_ttl_seconds(&self) -> Option<u32>;

    /// Whether a document of the type can stop existing once written: its owner may delete
    /// it (`canBeDeleted`), a create consuming it may (`canBeDeleted: "onlyWhenConsumed"`),
    /// the contract's moderators may (`moderatorAbilities.delete`), or the platform deletes
    /// it when its `ttl` passes. A `permanentDocument` reference and a
    /// list element reference may only target a type for which this is false; which of the
    /// other two kinds may target one for which it is true is
    /// [`Self::document_reference_kind`]'s answer.
    fn documents_can_disappear(&self) -> bool;

    /// The one kind of document reference that may target the type, from what
    /// can make its documents leave state: `permanentDocument` when nothing can
    /// ([`Self::documents_can_disappear`] is false), `moderatedDocument` when
    /// only the contract's moderators can and every removal leaves a record (its
    /// owner can not delete one, no create consumes one, no `ttl` expires one, and
    /// `moderatorAbilities.deleteKeepsRecord` holds), `deletableDocument`
    /// otherwise. None of what it reads can change on a contract update, so the
    /// answer holds for good.
    fn document_reference_kind(&self) -> DocumentReferenceKind;

    /// The top-level properties the `immutable` keyword (protocol version 14)
    /// lists by name on a mutable document type: frozen at document creation.
    /// A replace that changes, adds or removes any of them is rejected with
    /// `DocumentImmutablePropertyChangedError`. Empty on document types that
    /// predate the keyword and on types whose documents are not mutable, where
    /// every property is already immutable. The properties it lists with a
    /// condition are [`Self::immutable_field_conditions`].
    fn immutable_fields(&self) -> &BTreeSet<String>;

    /// The dotted paths of the properties that declare `distinctFrom`
    /// (protocol version 14), in schema order. Empty on generations that
    /// predate the keyword.
    fn distinct_from_fields(&self) -> &[String];

    /// The dotted path of every property that declares `generatedFrom`
    /// (protocol version 14) with its declaration, in schema order, so a
    /// document write visits only them. Empty on generations that predate the
    /// keyword.
    fn generated_from_fields(&self) -> &[(String, GeneratedFrom)];

    /// The top-level properties the `immutable` keyword lists with a
    /// condition, each with it: a replace that changes, adds or removes one
    /// while its condition holds is rejected with
    /// `DocumentImmutablePropertyChangedError`. The condition is judged on the
    /// document the replace writes, reading the stored one through `$old.`.
    /// Disjoint from [`Self::immutable_fields`]; empty on document types that
    /// predate the keyword.
    fn immutable_field_conditions(&self) -> &BTreeMap<String, PropertyConstraint>;

    /// The `retractedWhen` condition: a replace whose written document meets
    /// it is a retraction, the one replace a banned or suspended owner may
    /// still make on a moderated contract. Judged as an
    /// [`Self::immutable_field_conditions`] condition is. `None` on document
    /// types that declare none and on those that predate the keyword.
    fn retracted_when(&self) -> Option<&PropertyConstraint>;

    /// The fixed fees in credits this document type charges for actions on its documents
    /// (the `actionFees` keyword, protocol version 14). `None` on document types that
    /// declare none and on those that predate the keyword.
    fn action_fees(&self) -> Option<&DocumentActionFees>;

    /// The `refersTo` declaration whose value is the document's `$ownerId`, the
    /// writer (the `ownerRefersTo` keyword, protocol version 14): consensus
    /// checks it with the writer's id as the value when a document is created,
    /// and on a replace under the rules of its target. `None` on document types
    /// that declare none and on those that predate the keyword. Enumerated with
    /// the property references by `DocumentTypeRef::reference_declarations`.
    fn owner_reference(&self) -> Option<&DocumentPropertyReferenceTarget>;

    /// The `refersTo` declaration whose value is the document's `$creatorId`,
    /// its creator (the `creatorRefersTo` keyword, protocol version 14),
    /// checked with the creator's id as the value when a document is created,
    /// and on a replace under the rules of its target. `None` on document types
    /// that declare none and on those that predate the keyword.
    fn creator_reference(&self) -> Option<&DocumentPropertyReferenceTarget>;

    /// The rules every created or replaced document must meet, by name, in the
    /// order they are checked (the `propertyConstraints` keyword, protocol version
    /// 14). Empty on document types that declare none and on those that predate
    /// the keyword.
    fn property_constraints(&self) -> &BTreeMap<String, PropertyConstraint>;

    /// The index properties whose values are read from the document a
    /// reference of the type points at (`"<reference property>.<field>"`,
    /// protocol version 14), by their names in the indexes. Empty on document
    /// types that declare none and on those that predate them.
    fn derived_index_properties(&self) -> &BTreeMap<String, DerivedIndexProperty>;
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

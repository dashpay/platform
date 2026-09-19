//! Collection, index, typed collection and rule manifests.

use alloc::vec::Vec;

use crate::declare::{
    ActionScope, BoundedKeyRequirement, CollectionKind, ContestedSpec, Countability, FieldSpec,
    IndexOnlySpec, Ranking, RuleKind, SecurityLevel, Store, TimeRangeSpec, TokenCostSpec,
    TradeMode, TypedCollectionKind, ValueType, WritePolicy,
};
use crate::identity::{CollectionName, IndexName, PropertyName, PropertyPath, RuleName};

/// One index, sugar expanded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexManifest {
    /// The index's identity within its collection.
    pub name: IndexName,
    /// Indexed property paths in order, all ascending.
    pub properties: Vec<PropertyPath>,
    /// Unique index.
    pub unique: bool,
    /// Null values are searchable.
    pub null_searchable: bool,
    /// Contested parameters, field matches sorted by property.
    pub contested: Option<ContestedSpec>,
    /// Count fast path.
    pub count: Countability,
    /// Range counts.
    pub range_count: bool,
    /// Integer property summed at the index.
    pub sum: Option<PropertyName>,
    /// Range sums.
    pub range_sum: bool,
    /// Ranking axes.
    pub ranked: Ranking,
    /// Time range bucketing.
    pub time_range: Option<TimeRangeSpec>,
    /// Index-only options.
    pub index_only: Option<IndexOnlySpec>,
}

/// One document collection or singleton, sugar expanded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionManifest {
    /// The collection's identity and native document type name.
    pub name: CollectionName,
    /// Documents or singleton.
    pub kind: CollectionKind,
    /// Author-declared schema revision.
    pub schema_revision: u32,
    /// Who may create documents.
    pub write: WritePolicy,
    /// Documents may be replaced.
    pub mutable: bool,
    /// Documents may be deleted.
    pub deletable: bool,
    /// Every revision is kept.
    pub keep_history: bool,
    /// Transfer revisions are kept.
    pub keep_transfer_history: bool,
    /// Purchase revisions are kept.
    pub keep_purchase_history: bool,
    /// Price-update revisions are kept.
    pub keep_pricing_history: bool,
    /// Documents may be transferred.
    pub transferable: bool,
    /// Marketplace mode.
    pub trade: TradeMode,
    /// Signature security level required to write.
    pub security_level: SecurityLevel,
    /// Identity encryption bounded key requirement.
    pub encryption_key: Option<BoundedKeyRequirement>,
    /// Identity decryption bounded key requirement.
    pub decryption_key: Option<BoundedKeyRequirement>,
    /// Count tree on the primary key.
    pub count: bool,
    /// Provable count on the primary key.
    pub range_count: bool,
    /// Integer property summed on the primary key.
    pub sum: Option<PropertyName>,
    /// Provable sum on the primary key.
    pub range_sum: bool,
    /// Documents live only in their indexes.
    pub index_only: bool,
    /// Required system properties, sorted.
    pub requires: Vec<PropertyPath>,
    /// Token prices, sorted by action.
    pub token_costs: Vec<TokenCostSpec>,
    /// Document store.
    pub store: Store,
    /// Stored fields, sorted by position at every level.
    pub fields: Vec<FieldSpec>,
    /// Indexes, sorted by name.
    pub indexes: Vec<IndexManifest>,
}

impl CollectionManifest {
    /// Looks an index up by name.
    pub fn index(&self, name: &str) -> Option<&IndexManifest> {
        self.indexes
            .iter()
            .find(|index| index.name.as_str() == name)
    }

    /// Looks a top-level field up by name.
    pub fn field(&self, name: &str) -> Option<&FieldSpec> {
        self.fields.iter().find(|field| field.name.as_str() == name)
    }
}

/// One typed specialized collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedCollectionManifest {
    /// The collection's identity.
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

/// One rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleManifest {
    /// The guarded collection.
    pub collection: CollectionName,
    /// The rule's name within the collection.
    pub name: RuleName,
    /// The guarded actions, sorted and deduplicated.
    pub actions: Vec<ActionScope>,
    /// How the rule decides.
    pub kind: RuleKind,
}

//! Document collections and singletons: the native document type switches,
//! the stored fields and the indexes.

use alloc::string::String;
use alloc::vec::Vec;

use super::field::{FieldSpec, FieldType, ReferenceTarget};
use super::index::IndexSpec;
use super::rule::ActionScope;
use super::DeclarationOrigin;
use crate::identity::{CollectionName, PropertyName};

/// Whether a collection holds many documents or one reserved record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CollectionKind {
    /// Ordinary documents addressed by document id.
    Documents,
    /// One native document under a reserved key: configuration, not a
    /// separate storage engine.
    Singleton,
}

/// Who may create documents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WritePolicy {
    /// Anyone (native creation restriction 0).
    #[default]
    Any,
    /// The contract owner only (native creation restriction 1).
    Owner,
    /// The contract's own code only. Pending native support; derives the
    /// `ContractWrites` capability requirement.
    Contract,
}

/// Marketplace trade mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TradeMode {
    /// No marketplace.
    #[default]
    None,
    /// Direct purchase at the listed price.
    DirectPurchase,
}

/// Signature security level required to write.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityLevel {
    /// Critical.
    Critical,
    /// High, the default.
    #[default]
    High,
    /// Medium.
    Medium,
}

/// Identity bounded key requirement (native storage key requirements).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundedKeyRequirement {
    /// One non-replaceable key.
    Unique,
    /// Several keys.
    Multiple,
    /// Several keys with a reference to the latest.
    MultipleReferenceToLatest,
}

/// Document store kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Store {
    /// Ordinary documents, readable by anyone with a proof.
    #[default]
    Public,
    /// Private document store. Catalogued, interface disabled: private Rust
    /// fields and access control are not encryption, and the store's
    /// encryption, key control, query visibility and proof behaviour are
    /// specified separately before this can be enabled.
    Private,
}

/// What happens to the tokens a priced action charges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenCostEffect {
    /// Transferred to the contract owner, the default.
    #[default]
    TransferToContractOwner,
    /// Burned.
    Burn,
}

/// Who pays the gas of a priced action.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GasPaidBy {
    /// The document owner, the default.
    #[default]
    DocumentOwner,
    /// The contract owner.
    ContractOwner,
    /// The contract owner when it can pay, else the document owner.
    PreferContractOwner,
}

/// A token price on one document action.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TokenCost {
    /// The token contract; the declaring contract when absent.
    pub contract: Option<[u8; 32]>,
    /// Token position in the token contract.
    pub token_position: u16,
    /// Amount charged.
    pub amount: u64,
    /// What happens to the tokens.
    pub effect: TokenCostEffect,
    /// Who pays gas.
    pub gas_paid_by: GasPaidBy,
}

impl TokenCost {
    /// A cost in the declaring contract's token with default effect and payer.
    pub fn new(token_position: u16, amount: u64) -> Self {
        TokenCost {
            contract: None,
            token_position,
            amount,
            effect: TokenCostEffect::default(),
            gas_paid_by: GasPaidBy::default(),
        }
    }

    /// Prices in another contract's token.
    pub fn contract(mut self, contract: [u8; 32]) -> Self {
        self.contract = Some(contract);
        self
    }

    /// Sets the effect.
    pub fn effect(mut self, effect: TokenCostEffect) -> Self {
        self.effect = effect;
        self
    }

    /// Sets the gas payer.
    pub fn gas_paid_by(mut self, gas_paid_by: GasPaidBy) -> Self {
        self.gas_paid_by = gas_paid_by;
        self
    }
}

/// A token cost bound to an action. One per action.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TokenCostSpec {
    /// The priced action.
    pub action: ActionScope,
    /// The price.
    pub cost: TokenCost,
}

/// A document collection or singleton.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionSpec {
    /// Where the spec came from.
    pub origin: DeclarationOrigin,
    /// The collection's identity and native document type name.
    pub name: CollectionName,
    /// Documents or singleton.
    pub kind: CollectionKind,
    /// Author-declared schema revision, at least 1. Recorded for compatibility
    /// reports; provisional meaning.
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
    /// Count tree on the primary key. `None` when the author said nothing, so
    /// that average sugar may promote it; an explicit `Some(false)` next to
    /// `average` is a conflict, as it is natively.
    pub count: Option<bool>,
    /// Provable count on the primary key; explicitness as for `count`.
    pub range_count: Option<bool>,
    /// Integer property summed on the primary key.
    pub sum: Option<PropertyName>,
    /// Provable sum on the primary key; explicitness as for `count`.
    pub range_sum: Option<bool>,
    /// Sugar for `count` plus `sum`; expanded by the validator, never stored in
    /// the manifest.
    pub average: Option<PropertyName>,
    /// Sugar for `range_count` plus `range_sum`; expanded by the validator.
    pub range_average: bool,
    /// Documents live only in their indexes.
    pub index_only: bool,
    /// Token prices per action.
    pub token_costs: Vec<TokenCostSpec>,
    /// Document store.
    pub store: Store,
    /// The Rust field marked `#[document_id]`; required on document
    /// collections, forbidden on singletons. A Rust name, never part of the
    /// manifest.
    pub document_id_field: Option<String>,
    /// Stored fields.
    pub fields: Vec<FieldSpec>,
    /// Indexes.
    pub indexes: Vec<IndexSpec>,
}

impl CollectionSpec {
    fn with_kind(name: CollectionName, kind: CollectionKind) -> Self {
        CollectionSpec {
            origin: DeclarationOrigin::Builder,
            name,
            kind,
            schema_revision: 1,
            write: WritePolicy::default(),
            mutable: true,
            deletable: true,
            keep_history: false,
            keep_transfer_history: false,
            keep_purchase_history: false,
            keep_pricing_history: false,
            transferable: false,
            trade: TradeMode::default(),
            security_level: SecurityLevel::default(),
            encryption_key: None,
            decryption_key: None,
            count: None,
            range_count: None,
            sum: None,
            range_sum: None,
            average: None,
            range_average: false,
            index_only: false,
            token_costs: Vec::new(),
            store: Store::default(),
            document_id_field: None,
            fields: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// A document collection with default switches.
    pub fn documents(name: CollectionName) -> Self {
        Self::with_kind(name, CollectionKind::Documents)
    }

    /// A singleton with default switches.
    pub fn singleton(name: CollectionName) -> Self {
        Self::with_kind(name, CollectionKind::Singleton)
    }

    /// Records the origin.
    pub fn with_origin(mut self, origin: DeclarationOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Sets the schema revision.
    pub fn schema_revision(mut self, revision: u32) -> Self {
        self.schema_revision = revision;
        self
    }

    /// Sets who may create documents.
    pub fn write(mut self, write: WritePolicy) -> Self {
        self.write = write;
        self
    }

    /// Sets whether documents may be replaced.
    pub fn mutable(mut self, mutable: bool) -> Self {
        self.mutable = mutable;
        self
    }

    /// Sets whether documents may be deleted.
    pub fn deletable(mut self, deletable: bool) -> Self {
        self.deletable = deletable;
        self
    }

    /// Keeps every revision.
    pub fn keep_history(mut self, keep: bool) -> Self {
        self.keep_history = keep;
        self
    }

    /// Keeps transfer revisions.
    pub fn keep_transfer_history(mut self, keep: bool) -> Self {
        self.keep_transfer_history = keep;
        self
    }

    /// Keeps purchase revisions.
    pub fn keep_purchase_history(mut self, keep: bool) -> Self {
        self.keep_purchase_history = keep;
        self
    }

    /// Keeps price-update revisions.
    pub fn keep_pricing_history(mut self, keep: bool) -> Self {
        self.keep_pricing_history = keep;
        self
    }

    /// Sets whether documents may be transferred.
    pub fn transferable(mut self, transferable: bool) -> Self {
        self.transferable = transferable;
        self
    }

    /// Sets the marketplace mode.
    pub fn trade(mut self, trade: TradeMode) -> Self {
        self.trade = trade;
        self
    }

    /// Sets the signature security level.
    pub fn security_level(mut self, level: SecurityLevel) -> Self {
        self.security_level = level;
        self
    }

    /// Sets the encryption key requirement.
    pub fn encryption_key(mut self, requirement: BoundedKeyRequirement) -> Self {
        self.encryption_key = Some(requirement);
        self
    }

    /// Sets the decryption key requirement.
    pub fn decryption_key(mut self, requirement: BoundedKeyRequirement) -> Self {
        self.decryption_key = Some(requirement);
        self
    }

    /// Counts documents on the primary key.
    pub fn count(mut self, count: bool) -> Self {
        self.count = Some(count);
        self
    }

    /// Provable counts on the primary key.
    pub fn range_count(mut self, range_count: bool) -> Self {
        self.range_count = Some(range_count);
        self
    }

    /// Sums a property on the primary key.
    pub fn sum(mut self, property: PropertyName) -> Self {
        self.sum = Some(property);
        self
    }

    /// Provable sums on the primary key.
    pub fn range_sum(mut self, range_sum: bool) -> Self {
        self.range_sum = Some(range_sum);
        self
    }

    /// Average sugar: `count` plus `sum` on the property.
    pub fn average(mut self, property: PropertyName) -> Self {
        self.average = Some(property);
        self
    }

    /// Range average sugar: `range_count` plus `range_sum`.
    pub fn range_average(mut self, range_average: bool) -> Self {
        self.range_average = range_average;
        self
    }

    /// Stores documents only in their indexes.
    pub fn index_only(mut self, index_only: bool) -> Self {
        self.index_only = index_only;
        self
    }

    /// Sets the document store.
    pub fn store(mut self, store: Store) -> Self {
        self.store = store;
        self
    }

    /// Names the Rust field carrying the document id.
    pub fn document_id_field(mut self, field: impl Into<String>) -> Self {
        self.document_id_field = Some(field.into());
        self
    }

    /// Adds a stored field.
    pub fn field(mut self, field: FieldSpec) -> Self {
        self.fields.push(field);
        self
    }

    /// Adds an index.
    pub fn index(mut self, index: IndexSpec) -> Self {
        self.indexes.push(index);
        self
    }

    /// Prices an action.
    pub fn token_cost(mut self, action: ActionScope, cost: TokenCost) -> Self {
        self.token_costs.push(TokenCostSpec { action, cost });
        self
    }

    /// Whether two specs describe the same collection apart from origin,
    /// indexes and the order of order-insensitive members (fields at every
    /// nesting level, token costs, reference agreements). Used to tell a
    /// restatement or extension from a conflict.
    pub fn same_shape_ignoring_indexes(&self, other: &CollectionSpec) -> bool {
        let mut left = self.normalized();
        let mut right = other.normalized();
        left.origin = right.origin;
        left.indexes = Vec::new();
        right.indexes = Vec::new();
        left == right
    }

    /// A copy with every order-insensitive member in canonical order: fields
    /// by position at every nesting level, token costs by action, reference
    /// agreements by referring property.
    pub fn normalized(&self) -> CollectionSpec {
        let mut normalized = self.clone();
        normalize_fields(&mut normalized.fields);
        normalized
            .token_costs
            .sort_by(|a, b| a.action.cmp(&b.action));
        normalized
    }
}

fn normalize_fields(fields: &mut [FieldSpec]) {
    for field in fields.iter_mut() {
        match &mut field.ty {
            FieldType::Object(nested) => normalize_fields(nested),
            FieldType::Reference(ReferenceTarget::PermanentDocument { agreement, .. }) => {
                agreement.sort();
            }
            _ => {}
        }
    }
    fields.sort_by(|a, b| a.position.cmp(&b.position).then(a.name.cmp(&b.name)));
}

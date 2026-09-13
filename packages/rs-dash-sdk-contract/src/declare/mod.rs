//! The typed declaration model.
//!
//! A [`ContractDeclaration`] gathers everything a contract package declares:
//! WASM modules and the interfaces between them, document collections with
//! their fields and indexes, typed specialized collections, entries, rules,
//! explicitly required capabilities and the receipt policy. Attributes and
//! typed builders both produce these specs; each spec records its
//! [`DeclarationOrigin`] so the validator can name both sides of a conflict.
//!
//! Identity is by declared name (see [`crate::identity`]); the order in which
//! specs are added never matters, and a builder may restate an attribute
//! declaration verbatim or extend an attribute-declared collection with more
//! indexes, but may not change what the attribute said.

pub mod capability;
pub mod collection;
pub mod collections;
pub mod entry;
pub mod field;
pub mod index;
pub mod module;
pub mod rule;

use alloc::vec::Vec;

pub use capability::{CapabilityRequirement, CapabilityStatus, ReceiptPolicy};
pub use collection::{
    BoundedKeyRequirement, CollectionKind, CollectionSpec, GasPaidBy, SecurityLevel, Store,
    TokenCost, TokenCostEffect, TokenCostSpec, TradeMode, WritePolicy,
};
pub use collections::{TypedCollectionKind, TypedCollectionSpec};
pub use entry::{EntrySpec, ParamSpec, Receiver, ValueType};
pub use field::{FieldSpec, FieldType, IntegerBounds, IntegerWidth, ReferenceTarget};
pub use index::{
    ContestedResolution, ContestedSpec, Countability, IndexOnlySpec, IndexSpec, RankedCount,
    Ranking, TimeRangeSpec,
};
pub use module::{InterfaceSpec, InternalFunctionSpec, ModuleSpec, IMPLICIT_MODULE};
pub use rule::{ActionScope, FieldContext, GuardExpr, Literal, RuleKind, RuleSpec};

use crate::manifest::CanonicalManifest;
use crate::validate::{validate, Diagnostic};

/// Where a spec came from. Used to tell a harmless restatement from a
/// conflict and to name both sides in a `ConflictingDeclaration` diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclarationOrigin {
    /// Produced by an attribute on a Rust item.
    Attribute,
    /// Produced by a typed builder in a declaration module.
    Builder,
}

impl core::fmt::Display for DeclarationOrigin {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            DeclarationOrigin::Attribute => "attribute",
            DeclarationOrigin::Builder => "builder",
        })
    }
}

/// Everything one contract package declares, before validation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContractDeclaration {
    /// WASM module targets. Empty means one implicit module named `main`.
    pub modules: Vec<ModuleSpec>,
    /// Interfaces modules provide to each other.
    pub interfaces: Vec<InterfaceSpec>,
    /// Document collections and singletons.
    pub collections: Vec<CollectionSpec>,
    /// Typed specialized collections.
    pub typed_collections: Vec<TypedCollectionSpec>,
    /// Externally callable entries.
    pub entries: Vec<EntrySpec>,
    /// Rules on ordinary document actions.
    pub rules: Vec<RuleSpec>,
    /// Explicitly required capabilities (`requires = [...]`).
    pub capabilities: Vec<CapabilityRequirement>,
    /// Receipt policy; stored unless disabled.
    pub receipts: ReceiptPolicy,
}

impl ContractDeclaration {
    /// An empty declaration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a module.
    pub fn module(mut self, spec: ModuleSpec) -> Self {
        self.modules.push(spec);
        self
    }

    /// Adds an interface.
    pub fn interface(mut self, spec: InterfaceSpec) -> Self {
        self.interfaces.push(spec);
        self
    }

    /// Adds a collection or singleton.
    pub fn collection(mut self, spec: CollectionSpec) -> Self {
        self.collections.push(spec);
        self
    }

    /// Adds a typed specialized collection.
    pub fn typed_collection(mut self, spec: TypedCollectionSpec) -> Self {
        self.typed_collections.push(spec);
        self
    }

    /// Adds an entry.
    pub fn entry(mut self, spec: EntrySpec) -> Self {
        self.entries.push(spec);
        self
    }

    /// Adds a rule.
    pub fn rule(mut self, spec: RuleSpec) -> Self {
        self.rules.push(spec);
        self
    }

    /// Declares an explicitly required capability.
    pub fn require(mut self, capability: CapabilityRequirement) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Sets the receipt policy.
    pub fn receipts(mut self, policy: ReceiptPolicy) -> Self {
        self.receipts = policy;
        self
    }

    /// Validates the declaration and builds its canonical manifest, or returns
    /// every diagnostic. Same as [`validate`].
    pub fn validate(&self) -> Result<CanonicalManifest, Vec<Diagnostic>> {
        validate(self)
    }
}

//! The canonical manifest: the sorted, order-independent description of a
//! validated contract package.
//!
//! Every table is keyed by a required unique identity (name, position or a
//! tuple of names), so no ordering depends on declaration order, attribute
//! versus builder origin, or which Rust module hosted an item. Two
//! declarations that differ only in those respects produce equal manifests.
//! Sugar (`average`, `range_average`) is expanded before the manifest, which
//! therefore has no average fields.
//!
//! A manifest is constructed only by [`crate::validate::validate`]. It has no
//! wire encoding and no digest here: the encoding and the numeric method and
//! type identifiers are allocated by the ABI work, and this module is the
//! provisional home of the shape until then.

pub mod bundle;
pub mod capability;
pub mod collection;
pub mod method;

use alloc::vec::Vec;

pub use bundle::{Binding, InterfaceEntry, ModuleEntry, ModuleTable};
pub use capability::{CapabilityEntry, CapabilityTable};
pub use collection::{CollectionManifest, IndexManifest, RuleManifest, TypedCollectionManifest};
pub use method::{MethodEntry, MethodTable};

use crate::declare::ReceiptPolicy;

/// The canonical manifest of one contract package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalManifest {
    /// Modules, interfaces and bindings.
    pub modules: ModuleTable,
    /// Document collections and singletons, sorted by name.
    pub collections: Vec<CollectionManifest>,
    /// Typed specialized collections, sorted by id.
    pub typed_collections: Vec<TypedCollectionManifest>,
    /// Entries, sorted by method name.
    pub methods: MethodTable,
    /// Rules, sorted by `(collection, name)`.
    pub rules: Vec<RuleManifest>,
    /// Required capabilities, sorted.
    pub capabilities: CapabilityTable,
    /// Receipt policy.
    pub receipts: ReceiptPolicy,
}

impl CanonicalManifest {
    /// Looks a collection up by name.
    pub fn collection(&self, name: &str) -> Option<&CollectionManifest> {
        self.collections
            .iter()
            .find(|collection| collection.name.as_str() == name)
    }

    /// Looks an entry up by method name.
    pub fn method(&self, name: &str) -> Option<&MethodEntry> {
        self.methods.entry(name)
    }
}

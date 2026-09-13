//! The method table.

use alloc::string::String;
use alloc::vec::Vec;

use crate::declare::{CollectionKind, ParamSpec, Receiver, ValueType};
use crate::identity::{MethodName, ModuleName};

/// One entry of the contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodEntry {
    /// The entry's identity.
    pub name: MethodName,
    /// The hosting module: a binding, not part of the identity.
    pub module: ModuleName,
    /// The WASM export symbol, derived from the name.
    pub export: String,
    /// The receiver.
    pub receiver: Receiver,
    /// Whether the receiver is addressed by a document id on the wire. True
    /// for a receiver on a document collection, false for a free entry or a
    /// singleton receiver (the singleton's key is reserved and host known).
    pub takes_document_id: bool,
    /// Whether the entry stages no writes.
    pub read_only: bool,
    /// Wire parameters after the document id, if any.
    pub params: Vec<ParamSpec>,
    /// Wire return type.
    pub returns: ValueType,
}

impl MethodEntry {
    pub(crate) fn takes_document_id(receiver: &Receiver, kind: Option<CollectionKind>) -> bool {
        matches!(receiver, Receiver::Ref(_) | Receiver::Mut(_))
            && kind == Some(CollectionKind::Documents)
    }
}

/// Every entry, sorted by method name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MethodTable {
    /// The entries.
    pub entries: Vec<MethodEntry>,
}

impl MethodTable {
    /// Looks an entry up by method name.
    pub fn entry(&self, name: &str) -> Option<&MethodEntry> {
        self.entries
            .iter()
            .find(|entry| entry.name.as_str() == name)
    }

    /// The method names, in canonical order.
    pub fn names(&self) -> impl Iterator<Item = &MethodName> {
        self.entries.iter().map(|entry| &entry.name)
    }
}

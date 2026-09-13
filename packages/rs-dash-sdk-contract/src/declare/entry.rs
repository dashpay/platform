//! Entries: the externally callable functions of a contract and their wire
//! types.
//!
//! Only a function marked `#[entry]` is callable; Rust `pub` alone exports
//! nothing. An entry's identity is its [`MethodName`], unique across every
//! module of the contract. Its WASM export symbol is derived from that name
//! (see [`crate::identity::entry_export_symbol`]) and its module is only a
//! binding: moving the function between modules changes the binding and
//! nothing else.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::field::IntegerWidth;
use super::DeclarationOrigin;
use crate::identity::{CollectionName, MethodName, ModuleName};

/// The receiver of an entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Receiver {
    /// A free function.
    None,
    /// `&self` on a persistent struct: the wrapper loads the addressed record
    /// and stages nothing.
    Ref(CollectionName),
    /// `&mut self` on a persistent struct: the wrapper loads the addressed
    /// record and stages the receiver update when the method returns
    /// successfully. The only implicit staging point in the model.
    Mut(CollectionName),
}

impl Receiver {
    /// The receiver's collection, if any.
    pub fn collection(&self) -> Option<&CollectionName> {
        match self {
            Receiver::None => None,
            Receiver::Ref(collection) | Receiver::Mut(collection) => Some(collection),
        }
    }

    /// Whether the receiver is mutable.
    pub fn is_mutable(&self) -> bool {
        matches!(self, Receiver::Mut(_))
    }
}

/// A bounded wire value type for entry parameters and returns.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// `()`
    Unit,
    /// `bool`
    Bool,
    /// A Rust integer.
    Integer(IntegerWidth),
    /// `f64`
    F64,
    /// A string with a required character bound.
    String {
        /// Maximum characters; absent is an `UnboundedField` diagnostic.
        max_chars: Option<u16>,
    },
    /// A byte array with a required length bound.
    Bytes {
        /// Maximum bytes; absent is an `UnboundedField` diagnostic.
        max_len: Option<u16>,
    },
    /// A 32-byte identifier.
    Identifier,
    /// A document id.
    DocumentId,
    /// An optional value.
    Option(Box<ValueType>),
    /// A list with a required length bound.
    List {
        /// Maximum items; absent is an `UnboundedField` diagnostic.
        max_len: Option<u32>,
        /// The item type.
        item: Box<ValueType>,
    },
    /// An embedded bounded value struct.
    Struct(Vec<(String, ValueType)>),
}

impl ValueType {
    /// A string of at most `max_chars` characters.
    pub fn string(max_chars: u16) -> Self {
        ValueType::String {
            max_chars: Some(max_chars),
        }
    }

    /// A byte array of at most `max_len` bytes.
    pub fn bytes(max_len: u16) -> Self {
        ValueType::Bytes {
            max_len: Some(max_len),
        }
    }

    /// A list of at most `max_len` items.
    pub fn list(max_len: u32, item: ValueType) -> Self {
        ValueType::List {
            max_len: Some(max_len),
            item: Box::new(item),
        }
    }

    /// An optional value.
    pub fn option(inner: ValueType) -> Self {
        ValueType::Option(Box::new(inner))
    }
}

/// One entry parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamSpec {
    /// The parameter name.
    pub name: String,
    /// The wire type.
    pub ty: ValueType,
}

/// An entry declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntrySpec {
    /// Where the spec came from.
    pub origin: DeclarationOrigin,
    /// The entry's identity.
    pub name: MethodName,
    /// The hosting module; the single module when absent.
    pub module: Option<ModuleName>,
    /// The receiver.
    pub receiver: Receiver,
    /// The entry stages no writes.
    pub read_only: bool,
    /// Wire parameters after the receiver's document id, if any.
    pub params: Vec<ParamSpec>,
    /// Wire return type.
    pub returns: ValueType,
}

impl EntrySpec {
    /// A free entry returning unit.
    pub fn new(name: MethodName) -> Self {
        EntrySpec {
            origin: DeclarationOrigin::Builder,
            name,
            module: None,
            receiver: Receiver::None,
            read_only: false,
            params: Vec::new(),
            returns: ValueType::Unit,
        }
    }

    /// Records the origin.
    pub fn with_origin(mut self, origin: DeclarationOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Binds the entry to a module.
    pub fn module(mut self, module: ModuleName) -> Self {
        self.module = Some(module);
        self
    }

    /// Sets the receiver.
    pub fn receiver(mut self, receiver: Receiver) -> Self {
        self.receiver = receiver;
        self
    }

    /// Marks the entry read-only.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Adds a parameter.
    pub fn param(mut self, name: impl Into<String>, ty: ValueType) -> Self {
        self.params.push(ParamSpec {
            name: name.into(),
            ty,
        });
        self
    }

    /// Sets the return type.
    pub fn returns(mut self, returns: ValueType) -> Self {
        self.returns = returns;
        self
    }
}

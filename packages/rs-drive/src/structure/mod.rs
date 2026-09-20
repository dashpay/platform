//! A description of the complete GroveDB structure Drive builds, as code.
//!
//! Every level of the tree is declared as a [`StructureNode`]: its key (a
//! fixed byte string taken from the real constant, or a template such as
//! "identity id, 32 bytes"), the element kinds that can sit there, the first
//! protocol version it exists in, and its children. Each area declares its own
//! part in a `structure.rs` beside its `paths.rs`, and
//! [`drive_structure`] assembles them under the root layer.
//!
//! The description is documentation that is tested like code:
//!
//! * [`lint`] checks it is internally consistent.
//! * [`conformance`] walks a real GroveDB and reports every element the
//!   description does not cover, so a change that adds structure has to
//!   describe it.
//! * [`export`] serializes it to `packages/rs-drive/grovedb-structure.json`,
//!   which the GroveDB structure viewer reads by git ref.
//!
//! Nothing on the block-execution path depends on this module; it is compiled
//! for tests and under the `structure` feature only.

mod builder;
/// Walks a real GroveDB and compares it with the description
pub mod conformance;
/// Serialization of the description to the committed JSON file
pub mod export;
mod kinds;
/// Static checks of the description
pub mod lint;
#[cfg(test)]
mod shape;
#[cfg(test)]
mod tests;

use crate::drive::structure::root_structure;
use dpp::util::deserializer::ProtocolVersion;
pub use kinds::ElementKind;
use serde::Serialize;

/// The stable identifier of a node: the dotted path of segments from the
/// root, for example `identities.identity.keys.key`.
pub type NodeId = String;

/// How the bytes of a dynamic key are produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyEncoding {
    /// Opaque bytes
    Raw,
    /// ASCII text
    Ascii,
    /// UTF-8 text
    Utf8,
    /// One byte
    U8,
    /// Two bytes, big endian
    U16Be,
    /// Four bytes, big endian
    U32Be,
    /// Eight bytes, big endian
    U64Be,
    /// A variable length integer
    VarInt,
    /// A 32 byte identifier
    Identifier32,
    /// A 20 byte hash
    Hash20,
    /// A 32 byte hash
    Hash32,
    /// A document property value serialized for an index
    SerializedValue,
    /// Several values concatenated; the description says which
    Composite,
}

/// Which keys a dynamic template accepts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "len", rename_all = "snake_case")]
pub enum KeyMatcher {
    /// Any key
    Any,
    /// Keys of exactly this length
    Len(usize),
    /// Keys of one of these lengths
    LenIn(Vec<usize>),
}

impl KeyMatcher {
    /// Whether the matcher accepts this key
    pub fn accepts(&self, key: &[u8]) -> bool {
        match self {
            KeyMatcher::Any => true,
            KeyMatcher::Len(len) => key.len() == *len,
            KeyMatcher::LenIn(lens) => lens.contains(&key.len()),
        }
    }

    /// Whether the matcher accepts every key
    pub fn is_any(&self) -> bool {
        matches!(self, KeyMatcher::Any)
    }

    /// Whether some key could be accepted by both matchers
    pub fn overlaps(&self, other: &KeyMatcher) -> bool {
        match (self, other) {
            (KeyMatcher::Any, _) | (_, KeyMatcher::Any) => true,
            (KeyMatcher::Len(a), KeyMatcher::Len(b)) => a == b,
            (KeyMatcher::Len(a), KeyMatcher::LenIn(b))
            | (KeyMatcher::LenIn(b), KeyMatcher::Len(a)) => b.contains(a),
            (KeyMatcher::LenIn(a), KeyMatcher::LenIn(b)) => a.iter().any(|len| b.contains(len)),
        }
    }
}

/// The key of a node inside its parent layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeySpec {
    /// The root of the whole structure; it has no key
    Root,
    /// One fixed key
    Fixed {
        /// The key bytes
        #[serde(rename = "hex", serialize_with = "export::serialize_hex")]
        bytes: Vec<u8>,
        /// A human readable name of the key
        label: String,
        /// The Rust constant the bytes come from, empty when the code uses a
        /// literal
        constant: String,
        /// The code writes the key as a character (`b"s"`), so it reads
        /// better as one than as a number
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        ascii: bool,
    },
    /// A template standing for many keys
    Dynamic {
        /// The name of the key parameter, for example `identity_id`
        name: String,
        /// Which keys the template accepts
        matcher: KeyMatcher,
        /// How the key bytes are produced
        encoding: KeyEncoding,
        /// What the key is
        description: String,
    },
}

/// Whether a node exists as soon as its parent does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    /// Created together with its parent
    Always,
    /// Created on first use
    Lazy,
}

/// One level of the GroveDB structure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StructureNode {
    /// The stable identifier, computed by [`StructureNode::build`]
    pub id: NodeId,
    /// The last segment of the identifier
    #[serde(skip)]
    pub segment: String,
    /// The key inside the parent layer
    pub key: KeySpec,
    /// The element kinds that can sit at this key. More than one when the
    /// code chooses between them.
    pub kinds: Vec<ElementKind>,
    /// When there are several kinds, what decides between them
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinds_note: Option<String>,
    /// What an item holds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// For references, the identifier of the node they point to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<NodeId>,
    /// The first protocol version the node exists in
    pub since: ProtocolVersion,
    /// The last protocol version the node exists in
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<ProtocolVersion>,
    /// Whether the node exists as soon as its parent does
    pub presence: Presence,
    /// The repository relative file holding the canonical definition
    pub source: String,
    /// The book chapter describing this part, relative to `book/src`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book: Option<String>,
    /// What the node is for
    pub description: String,
    /// The node's children are those of this other node. Used where the
    /// structure repeats to an arbitrary depth, as index levels do.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurse: Option<NodeId>,
    /// Set for trees whose contents are not a Merk of elements (commitment
    /// trees, MMR trees, bulk append trees); says what they hold instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opaque: Option<String>,
    /// The levels below
    pub children: Vec<StructureNode>,
}

/// The complete GroveDB structure of Drive at the latest protocol version,
/// with every node tagged with the protocol version that introduced it.
pub fn drive_structure() -> StructureNode {
    root_structure().build()
}

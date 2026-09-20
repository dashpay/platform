use crate::structure::{ElementKind, NodeId, StructureNode};
use dpp::util::deserializer::ProtocolVersion;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

/// The version of the JSON layout. Bumped when a reader of an older layout
/// could no longer make sense of the file.
pub const STRUCTURE_SCHEMA_VERSION: u32 = 1;

/// The repository relative path of the committed JSON file
pub const STRUCTURE_JSON_PATH: &str = "packages/rs-drive/grovedb-structure.json";

/// The serialized form of the structure: what the viewer reads.
#[derive(Clone, Debug, Serialize)]
pub struct StructureDocument {
    /// The version of this layout
    pub schema_version: u32,
    /// The latest protocol version the description covers
    pub latest_protocol_version: ProtocolVersion,
    /// Every element kind, with what a reader needs to draw it
    pub element_kinds: Vec<ElementKindInfo>,
    /// The structure
    pub root: StructureNode,
    /// The exact Merk binary tree of layers whose keys are all fixed, by the
    /// identifier of the node holding the layer
    pub layer_shapes: BTreeMap<NodeId, LayerShape>,
}

/// What a reader needs to know about an element kind
#[derive(Clone, Debug, Serialize)]
pub struct ElementKindInfo {
    /// The name of the kind
    pub name: ElementKind,
    /// Whether elements of this kind hold a layer below them
    pub is_tree: bool,
    /// Whether that layer is something other than a Merk of elements
    pub is_opaque: bool,
    /// Whether the element points at another element
    pub is_reference: bool,
}

/// The exact Merk binary tree of one layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LayerShape {
    /// How the layer was built, since the shape depends on the order of
    /// insertion: `genesis@<protocol version>` for a fresh chain.
    pub origin: String,
    /// The root of the binary tree
    pub tree: ShapeNode,
}

/// One node of a Merk binary tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShapeNode {
    /// The key of the element at this node
    #[serde(rename = "hex", serialize_with = "serialize_hex")]
    pub key: Vec<u8>,
    /// The left child, holding smaller keys
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<Box<ShapeNode>>,
    /// The right child, holding larger keys
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<Box<ShapeNode>>,
}

impl StructureDocument {
    /// Wraps a built structure for serialization
    pub fn new(
        root: StructureNode,
        latest_protocol_version: ProtocolVersion,
        layer_shapes: BTreeMap<NodeId, LayerShape>,
    ) -> Self {
        StructureDocument {
            schema_version: STRUCTURE_SCHEMA_VERSION,
            latest_protocol_version,
            element_kinds: ElementKind::ALL
                .iter()
                .map(|kind| ElementKindInfo {
                    name: *kind,
                    is_tree: kind.is_tree(),
                    is_opaque: kind.is_opaque(),
                    is_reference: kind.is_reference(),
                })
                .collect(),
            root,
            layer_shapes,
        }
    }
}

/// Serializes bytes as lowercase hex
pub fn serialize_hex<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&hex::encode(bytes))
}

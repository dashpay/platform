//! The GroveDB layout of one document type: every tree and element Drive
//! writes under `[DataContractDocuments, contract id, 1, document type name]`,
//! with the element or tree type Drive picks for it, computed from the
//! document type alone.
//!
//! The tree types come from the functions the insert walkers call
//! (`primary_key_tree_type`, `property_name_tree_type_and_ranked_axes_for_level`,
//! `index_level_tree_types_with_continuation_demotion`,
//! `terminal_member_tree_type`, `continuation_contributes_zero`,
//! `zero_contribution_wrapper`); the element choices the walkers make inline
//! (history, unique terminals, indexOnly entries, sum-carrying references) are
//! restated here and held to the walkers by a test that inserts documents
//! and compares every element Drive wrote with this layout
//! (`tests::layout_matches_what_drive_writes`).
//!
//! Each node names the node of the static structure description
//! (`drive::document::structure`, published as `grovedb-structure.json`) it
//! is an instance of, so a viewer can link a concrete layer to the general
//! description of that kind of layer.

use crate::drive::document::index_level_tree_types::{
    continuation_contributes_zero, index_level_tree_types_with_continuation_demotion,
    level_counts_continuations, terminal_member_tree_type, zero_contribution_wrapper,
    ZeroContributionWrapper,
};
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes_for_level;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{
    is_flat_level_key, DocumentTypeRef, IndexLevel, IndexLevelTypeInfo, IndexType,
};
use dpp::platform_value::{Value, ValueMap};
use dpp::version::PlatformVersion;
use grovedb::element::IndexAxis;
use grovedb_merk::tree_type::TreeType;

/// Every tree and element Drive writes for one document type.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentTypeLayout {
    /// The document type name.
    pub document_type: String,
    /// The document type tree and everything below it.
    pub root: LayoutNode,
}

/// One layer of the layout: a key (fixed, or standing for a family of keys
/// such as every document id), the element Drive writes there, and the
/// layers below it.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutNode {
    /// The key, or what the keys at this position are.
    pub key: LayoutKey,
    /// What this layer is.
    pub role: LayoutRole,
    /// The element Drive writes at this key.
    pub element: LayoutElement,
    /// What Drive writes at this key instead in some cases, and when.
    pub alternative: Option<Box<LayoutAlternative>>,
    /// The indexes of the document type that use this layer, in name order.
    pub indexes: Vec<String>,
    /// Conditions and details a reader needs besides the element.
    pub notes: Vec<LayoutNote>,
    /// The layers below.
    pub children: Vec<LayoutNode>,
}

/// What Drive writes at a key instead of the node's own element, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutAlternative {
    /// When the alternative applies.
    pub when: &'static str,
    /// The alternative element and the layers below it.
    pub node: LayoutNode,
}

/// A key, or the family of keys a node stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutKey {
    /// One fixed key.
    Fixed {
        /// The key bytes.
        bytes: Vec<u8>,
        /// A readable name for the key.
        label: String,
    },
    /// One key per document: the 32 byte document id.
    DocumentId,
    /// One key per stored revision: the block time of the revision in
    /// milliseconds, a u64 big endian with the sign bit flipped.
    RevisionTime,
    /// One key per distinct value of `property` among the documents,
    /// serialized for ordering; an absent or null value is the empty key.
    PropertyValue {
        /// The indexed property.
        property: String,
    },
    /// One key per time window start of `property` that holds a document:
    /// the window start, encoded like the timestamp.
    TimeRangeBucket {
        /// The timestamp the windows bucket.
        property: String,
        /// The length of a window in seconds.
        range_seconds: u64,
        /// The time between window starts in seconds.
        step_seconds: u64,
        /// The shift of the window boundaries in seconds.
        phase_seconds: u64,
    },
    /// One key per entry of an indexOnly document type: the values of the
    /// index's terminal components concatenated (32 bytes for `$ownerId`).
    MemberKey {
        /// The terminal components, in order.
        components: Vec<String>,
    },
}

/// What a layer is, and the node of the structure description it is an
/// instance of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutRole {
    /// The document type tree.
    DocumentType,
    /// The documents by id (`[0]`).
    PrimaryKey,
    /// One document.
    Document,
    /// The pointer to the newest revision of a document with history.
    LatestRevision,
    /// One stored revision of a document with history.
    Revision,
    /// The first property of an index, or the level of a flat indexOnly
    /// index.
    IndexProperty,
    /// The values of one indexed property.
    IndexValue,
    /// The next property of a compound index.
    NextIndexProperty,
    /// Where an index ends at a value (`[0]`).
    Terminal,
    /// One document (or indexOnly entry) under a terminal.
    Member,
}

impl LayoutRole {
    /// The id of the node of the structure description this layer is an
    /// instance of (`drive::document::structure`).
    pub fn structure_node(&self) -> &'static str {
        match self {
            LayoutRole::DocumentType => "contracts.contract.documents.document_type",
            LayoutRole::PrimaryKey => "contracts.contract.documents.document_type.primary_key",
            LayoutRole::Document => {
                "contracts.contract.documents.document_type.primary_key.document"
            }
            LayoutRole::LatestRevision => {
                "contracts.contract.documents.document_type.primary_key.document.latest"
            }
            LayoutRole::Revision => {
                "contracts.contract.documents.document_type.primary_key.document.revision"
            }
            LayoutRole::IndexProperty => {
                "contracts.contract.documents.document_type.index_property"
            }
            LayoutRole::IndexValue => {
                "contracts.contract.documents.document_type.index_property.value"
            }
            LayoutRole::NextIndexProperty => {
                "contracts.contract.documents.document_type.index_property.value.next_property"
            }
            LayoutRole::Terminal => {
                "contracts.contract.documents.document_type.index_property.value.members"
            }
            LayoutRole::Member => {
                "contracts.contract.documents.document_type.index_property.value.members.member"
            }
        }
    }

    fn name(&self) -> &'static str {
        match self {
            LayoutRole::DocumentType => "documentType",
            LayoutRole::PrimaryKey => "primaryKey",
            LayoutRole::Document => "document",
            LayoutRole::LatestRevision => "latestRevision",
            LayoutRole::Revision => "revision",
            LayoutRole::IndexProperty => "indexProperty",
            LayoutRole::IndexValue => "indexValue",
            LayoutRole::NextIndexProperty => "nextIndexProperty",
            LayoutRole::Terminal => "terminal",
            LayoutRole::Member => "member",
        }
    }
}

/// The element Drive writes at a key.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutElement {
    /// The element kind.
    pub kind: LayoutElementKind,
    /// The wrapper that makes the tree contribute nothing to the aggregating
    /// tree above it, if any.
    pub wrapper: Option<ZeroContributionWrapper>,
    /// The ranking axes of an indexed tree, empty otherwise.
    pub ranked_axes: Vec<IndexAxis>,
}

/// The kind of an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutElementKind {
    /// A tree of this type.
    Tree(TreeType),
    /// A value.
    Item,
    /// A value that also carries a sum.
    ItemWithSumItem,
    /// A reference to another element.
    Reference,
    /// A reference that also carries a sum.
    ReferenceWithSumItem,
}

impl LayoutElementKind {
    /// The kind's name, as the structure description spells it.
    pub fn name(&self) -> &'static str {
        match self {
            LayoutElementKind::Tree(tree_type) => tree_kind_name(*tree_type),
            LayoutElementKind::Item => "Item",
            LayoutElementKind::ItemWithSumItem => "ItemWithSumItem",
            LayoutElementKind::Reference => "Reference",
            LayoutElementKind::ReferenceWithSumItem => "ReferenceWithSumItem",
        }
    }
}

fn tree_kind_name(tree_type: TreeType) -> &'static str {
    match tree_type {
        TreeType::NormalTree => "Tree",
        TreeType::SumTree => "SumTree",
        TreeType::BigSumTree => "BigSumTree",
        TreeType::CountTree => "CountTree",
        TreeType::CountSumTree => "CountSumTree",
        TreeType::ProvableCountTree => "ProvableCountTree",
        TreeType::ProvableCountSumTree => "ProvableCountSumTree",
        TreeType::CommitmentTree(_) => "CommitmentTree",
        TreeType::MmrTree => "MmrTree",
        TreeType::BulkAppendTree(_) => "BulkAppendTree",
        TreeType::DenseAppendOnlyFixedSizeTree(_) => "DenseAppendOnlyFixedSizeTree",
        TreeType::ProvableSumTree => "ProvableSumTree",
        TreeType::ProvableCountProvableSumTree => "ProvableCountProvableSumTree",
        TreeType::ProvableSumIndexedTree => "ProvableSumIndexedTree",
        TreeType::ProvableCountIndexedTree => "ProvableCountIndexedTree",
        TreeType::ProvableCountProvableSumIndexedTree => "ProvableCountProvableSumIndexedTree",
        TreeType::PrivateDocumentStore(_) => "PrivateDocumentStore",
    }
}

fn wrapper_name(wrapper: ZeroContributionWrapper) -> &'static str {
    match wrapper {
        ZeroContributionWrapper::NonCounted => "NonCounted",
        ZeroContributionWrapper::NotSummed => "NotSummed",
        ZeroContributionWrapper::NotCountedOrSummed => "NotCountedOrSummed",
    }
}

fn axis_name(axis: IndexAxis) -> &'static str {
    match axis {
        IndexAxis::Count => "count",
        IndexAxis::Sum => "sum",
        IndexAxis::Avg => "avg",
    }
}

/// A condition or detail of a layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutNote {
    /// The document type is indexOnly: no documents are stored whole.
    IndexOnly,
    /// A terminal of an index with `nullSearchable: false`: a document whose
    /// indexed values are all null writes no entry here (the value trees
    /// above are still written, with empty keys).
    NotNullSearchable,
    /// A contested index is laid out as a unique one here; the contest lives
    /// in the votes tree.
    Contested,
    /// A value of an indexOnly type's first property that documents may
    /// leave out: a document without it writes nothing in this index.
    SkipIfAbsent {
        /// The property.
        property: String,
    },
    /// A preallocated indexOnly index: its value trees and empty terminal
    /// are created when the referenced document is inserted.
    Preallocated,
    /// A time window level: a document lands in every window that contains
    /// its time, up to this many.
    TimeRangeOverlap {
        /// The most windows one time falls in.
        windows: u64,
    },
    /// A time window level whose entries expire after the window: written
    /// without storage flags and removed by a later cleanup.
    TimeRangeTtl {
        /// Seconds an entry is kept after its window.
        ttl_seconds: u64,
    },
}

impl LayoutNote {
    fn code(&self) -> &'static str {
        match self {
            LayoutNote::IndexOnly => "indexOnly",
            LayoutNote::NotNullSearchable => "notNullSearchable",
            LayoutNote::Contested => "contested",
            LayoutNote::SkipIfAbsent { .. } => "skipIfAbsent",
            LayoutNote::Preallocated => "preallocated",
            LayoutNote::TimeRangeOverlap { .. } => "timeRangeOverlap",
            LayoutNote::TimeRangeTtl { .. } => "timeRangeTtl",
        }
    }

    /// The note in words.
    pub fn text(&self) -> String {
        match self {
            LayoutNote::IndexOnly => {
                "indexOnly: documents are not stored whole; the index entries are the rows"
                    .to_string()
            }
            LayoutNote::NotNullSearchable => "nullSearchable is false: a document whose indexed \
                 values are all null writes no entry here, though the value trees above are \
                 written with empty keys"
                .to_string(),
            LayoutNote::Contested => {
                "contested: laid out as a unique index here; the contest itself is kept in the \
                 votes tree"
                    .to_string()
            }
            LayoutNote::SkipIfAbsent { property } => {
                format!("a document that leaves out {property} writes nothing in this index")
            }
            LayoutNote::Preallocated => {
                "preallocated: the value trees and the empty terminal of this index are created \
                 when the referenced document is inserted"
                    .to_string()
            }
            LayoutNote::TimeRangeOverlap { windows } => {
                format!("a document lands in every window containing its time: up to {windows}")
            }
            LayoutNote::TimeRangeTtl { ttl_seconds } => format!(
                "entries expire {ttl_seconds} seconds after their window: written without \
                 storage flags and removed by a later cleanup"
            ),
        }
    }
}

/// The GroveDB layout of `document_type` at `platform_version`.
pub fn document_type_layout(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<DocumentTypeLayout, Error> {
    let index_paths = index_paths(document_type);
    let mut children = Vec::new();

    if !document_type.index_only() {
        children.push(primary_key_node(document_type, platform_version)?);
    }

    for (level_key, level) in document_type.index_structure().sub_levels() {
        children.push(top_index_node(
            document_type,
            level_key,
            level,
            &index_paths,
        )?);
    }

    let mut notes = Vec::new();
    if document_type.index_only() {
        notes.push(LayoutNote::IndexOnly);
    }

    Ok(DocumentTypeLayout {
        document_type: document_type.name().clone(),
        root: LayoutNode {
            key: fixed(
                document_type.name().as_bytes().to_vec(),
                document_type.name(),
            ),
            role: LayoutRole::DocumentType,
            element: tree(TreeType::NormalTree),
            alternative: None,
            indexes: index_paths.iter().map(|(name, _)| name.clone()).collect(),
            notes,
            children,
        },
    })
}

/// Each index's name and the level keys its path takes through the index
/// structure, as `IndexLevel::try_from_indices` keys them.
fn index_paths(document_type: DocumentTypeRef) -> Vec<(String, Vec<String>)> {
    document_type
        .indexes()
        .values()
        .map(|index| {
            let path = match index.flat_level_key() {
                Some(flat_key) if index.properties.is_empty() => vec![flat_key],
                _ => index
                    .properties
                    .iter()
                    .enumerate()
                    .map(|(position, property)| index.level_key(position, &property.name))
                    .collect(),
            };
            (index.name.clone(), path)
        })
        .collect()
}

/// The indexes whose path passes through `path`.
fn indexes_through(index_paths: &[(String, Vec<String>)], path: &[String]) -> Vec<String> {
    index_paths
        .iter()
        .filter(|(_, index_path)| index_path.starts_with(path))
        .map(|(name, _)| name.clone())
        .collect()
}

/// The index whose path ends at `path`.
fn index_ending_at(index_paths: &[(String, Vec<String>)], path: &[String]) -> Vec<String> {
    index_paths
        .iter()
        .filter(|(_, index_path)| index_path.as_slice() == path)
        .map(|(name, _)| name.clone())
        .collect()
}

fn primary_key_node(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<LayoutNode, Error> {
    let summable = document_type.documents_summable().is_some();
    let document = if document_type.documents_keep_history() {
        LayoutNode {
            key: LayoutKey::DocumentId,
            role: LayoutRole::Document,
            element: tree(if summable {
                TreeType::SumTree
            } else {
                TreeType::NormalTree
            }),
            alternative: None,
            indexes: vec![],
            notes: vec![],
            children: vec![
                leaf(
                    fixed(vec![0], "Latest"),
                    LayoutRole::LatestRevision,
                    reference(summable),
                ),
                leaf(
                    LayoutKey::RevisionTime,
                    LayoutRole::Revision,
                    element(LayoutElementKind::Item),
                ),
            ],
        }
    } else {
        leaf(
            LayoutKey::DocumentId,
            LayoutRole::Document,
            element(if summable {
                LayoutElementKind::ItemWithSumItem
            } else {
                LayoutElementKind::Item
            }),
        )
    };

    Ok(LayoutNode {
        key: fixed(vec![0], "PrimaryKey"),
        role: LayoutRole::PrimaryKey,
        element: tree(document_type.primary_key_tree_type(platform_version)?),
        alternative: None,
        indexes: vec![],
        notes: vec![],
        children: vec![document],
    })
}

/// A first-level index tree, created with the contract.
fn top_index_node(
    document_type: DocumentTypeRef,
    level_key: &str,
    level: &IndexLevel,
    index_paths: &[(String, Vec<String>)],
) -> Result<LayoutNode, Error> {
    let path = vec![level_key.to_string()];
    let (property_name_tree_type, ranked_axes) =
        property_name_tree_type_and_ranked_axes_for_level(level)?;

    if is_flat_level_key(level_key) {
        // A flat indexOnly index: its entries live straight under the level,
        // with no value level.
        let info = level.has_index_with_type().ok_or_else(|| {
            Error::Drive(DriveError::CorruptedContractIndexes(format!(
                "flat index level {level_key:?} of document type {} ends no index",
                document_type.name()
            )))
        })?;
        let components = info.terminal.clone().unwrap_or_default();
        return Ok(LayoutNode {
            key: fixed(
                level_key.as_bytes().to_vec(),
                format!("(flat) {}", components.join(", ")),
            ),
            role: LayoutRole::IndexProperty,
            element: LayoutElement {
                kind: LayoutElementKind::Tree(property_name_tree_type),
                wrapper: None,
                ranked_axes,
            },
            alternative: None,
            indexes: indexes_through(index_paths, &path),
            notes: vec![],
            children: vec![terminal_node(info, index_ending_at(index_paths, &path))],
        });
    }

    let mut notes = Vec::new();
    let property = match level.time_range() {
        Some(transform) => transform.source.clone(),
        None => level_key.to_string(),
    };
    if document_type.index_only() && !document_type.required_fields().contains(&property) {
        notes.push(LayoutNote::SkipIfAbsent {
            property: property.clone(),
        });
    }

    Ok(LayoutNode {
        key: fixed(level_key.as_bytes().to_vec(), level_key),
        role: LayoutRole::IndexProperty,
        element: LayoutElement {
            kind: LayoutElementKind::Tree(property_name_tree_type),
            wrapper: None,
            ranked_axes,
        },
        alternative: None,
        indexes: indexes_through(index_paths, &path),
        notes,
        children: vec![value_node(level, level_key, &path, true, index_paths)?],
    })
}

/// The values of one indexed property, with what hangs under each.
fn value_node(
    level: &IndexLevel,
    level_key: &str,
    path: &[String],
    top: bool,
    index_paths: &[(String, Vec<String>)],
) -> Result<LayoutNode, Error> {
    let value_tree_type = index_level_tree_types_with_continuation_demotion(level)?.value_tree_type;

    let mut notes = Vec::new();
    let key = match level.time_range().filter(|_| top) {
        Some(transform) => {
            notes.push(LayoutNote::TimeRangeOverlap {
                windows: transform.overlap_factor(),
            });
            if let Some(ttl_seconds) = transform.ttl_seconds {
                notes.push(LayoutNote::TimeRangeTtl { ttl_seconds });
            }
            LayoutKey::TimeRangeBucket {
                property: transform.source.clone(),
                range_seconds: transform.range_seconds,
                step_seconds: transform.step_seconds,
                phase_seconds: transform.phase_seconds,
            }
        }
        None => LayoutKey::PropertyValue {
            property: level_key.to_string(),
        },
    };

    let mut children = Vec::new();
    if let Some(info) = level.has_index_with_type() {
        children.push(terminal_node(info, index_ending_at(index_paths, path)));
    }

    let parent_counts_continuations = level_counts_continuations(level);
    for (sub_key, sub_level) in level.sub_levels() {
        let (property_name_tree_type, ranked_axes) =
            property_name_tree_type_and_ranked_axes_for_level(sub_level)?;
        let wrapper = if continuation_contributes_zero(
            value_tree_type,
            parent_counts_continuations,
            sub_level,
        ) {
            zero_contribution_wrapper(value_tree_type, property_name_tree_type).map_err(
                |refusal| {
                    Error::Drive(DriveError::CorruptedContractIndexes(format!(
                        "index level {sub_key:?} cannot hang under a {} value tree: {refusal:?}",
                        tree_kind_name(value_tree_type)
                    )))
                },
            )?
        } else {
            None
        };
        let mut sub_path = path.to_vec();
        sub_path.push(sub_key.clone());
        children.push(LayoutNode {
            key: fixed(sub_key.as_bytes().to_vec(), sub_key),
            role: LayoutRole::NextIndexProperty,
            element: LayoutElement {
                kind: LayoutElementKind::Tree(property_name_tree_type),
                wrapper,
                ranked_axes,
            },
            alternative: None,
            indexes: indexes_through(index_paths, &sub_path),
            notes: vec![],
            children: vec![value_node(
                sub_level,
                sub_key,
                &sub_path,
                false,
                index_paths,
            )?],
        });
    }

    Ok(LayoutNode {
        key,
        role: LayoutRole::IndexValue,
        element: tree(value_tree_type),
        alternative: None,
        indexes: indexes_through(index_paths, path),
        notes,
        children,
    })
}

/// Where an index ends at a value: the `[0]` key.
fn terminal_node(info: &IndexLevelTypeInfo, indexes: Vec<String>) -> LayoutNode {
    let summable = info.summable.is_some();
    let mut notes = Vec::new();
    if !info.should_insert_with_all_null {
        notes.push(LayoutNote::NotNullSearchable);
    }
    if info.index_type == IndexType::ContestedResourceIndex {
        notes.push(LayoutNote::Contested);
    }
    if info.preallocated {
        notes.push(LayoutNote::Preallocated);
    }

    let members = |member: LayoutNode| LayoutNode {
        key: fixed(vec![0], "Members"),
        role: LayoutRole::Terminal,
        element: tree(terminal_member_tree_type(info)),
        alternative: None,
        indexes: indexes.clone(),
        notes: notes.clone(),
        children: vec![member],
    };

    if let Some(components) = &info.terminal {
        // indexOnly: each entry is an item keyed by its terminal components.
        return members(leaf(
            LayoutKey::MemberKey {
                components: components.clone(),
            },
            LayoutRole::Member,
            element(if summable {
                LayoutElementKind::ItemWithSumItem
            } else {
                LayoutElementKind::Item
            }),
        ));
    }

    let by_document = members(leaf(
        LayoutKey::DocumentId,
        LayoutRole::Member,
        reference(summable),
    ));
    if !info.index_type.is_unique() {
        return by_document;
    }

    // A unique index writes the reference straight at `[0]`, unless a
    // value is null: then several documents can share the key, and they
    // go in a tree by id like a non-unique index.
    LayoutNode {
        key: fixed(vec![0], "Unique reference"),
        role: LayoutRole::Terminal,
        element: reference(summable),
        alternative: Some(Box::new(LayoutAlternative {
            when: "an indexed value of the document is null",
            node: by_document,
        })),
        indexes,
        notes,
        children: vec![],
    }
}

fn fixed(bytes: Vec<u8>, label: impl Into<String>) -> LayoutKey {
    LayoutKey::Fixed {
        bytes,
        label: label.into(),
    }
}

fn element(kind: LayoutElementKind) -> LayoutElement {
    LayoutElement {
        kind,
        wrapper: None,
        ranked_axes: vec![],
    }
}

fn tree(tree_type: TreeType) -> LayoutElement {
    element(LayoutElementKind::Tree(tree_type))
}

fn reference(summable: bool) -> LayoutElement {
    element(if summable {
        LayoutElementKind::ReferenceWithSumItem
    } else {
        LayoutElementKind::Reference
    })
}

fn leaf(key: LayoutKey, role: LayoutRole, element: LayoutElement) -> LayoutNode {
    LayoutNode {
        key,
        role,
        element,
        alternative: None,
        indexes: vec![],
        notes: vec![],
        children: vec![],
    }
}

fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

fn map(entries: Vec<(&str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect::<ValueMap>(),
    )
}

fn texts(values: &[String]) -> Value {
    Value::Array(values.iter().map(|value| text(value)).collect())
}

impl LayoutKey {
    fn to_value(&self) -> Value {
        match self {
            LayoutKey::Fixed { bytes, label } => map(vec![
                ("kind", text("fixed")),
                ("hex", text(&hex::encode(bytes))),
                ("label", text(label)),
            ]),
            LayoutKey::DocumentId => map(vec![("kind", text("documentId"))]),
            LayoutKey::RevisionTime => map(vec![("kind", text("revisionTime"))]),
            LayoutKey::PropertyValue { property } => map(vec![
                ("kind", text("propertyValue")),
                ("property", text(property)),
            ]),
            LayoutKey::TimeRangeBucket {
                property,
                range_seconds,
                step_seconds,
                phase_seconds,
            } => map(vec![
                ("kind", text("timeRangeBucket")),
                ("property", text(property)),
                ("rangeSeconds", Value::U64(*range_seconds)),
                ("stepSeconds", Value::U64(*step_seconds)),
                ("phaseSeconds", Value::U64(*phase_seconds)),
            ]),
            LayoutKey::MemberKey { components } => map(vec![
                ("kind", text("memberKey")),
                ("components", texts(components)),
            ]),
        }
    }
}

impl LayoutNode {
    /// The node as a plain value: `{ key, role, structureNode, element,
    /// wrapper, rankedAxes, indexes, notes, alternative, children }`.
    pub fn to_value(&self) -> Value {
        map(vec![
            ("key", self.key.to_value()),
            ("role", text(self.role.name())),
            ("structureNode", text(self.role.structure_node())),
            ("element", text(self.element.kind.name())),
            (
                "wrapper",
                self.element
                    .wrapper
                    .map(|wrapper| text(wrapper_name(wrapper)))
                    .unwrap_or(Value::Null),
            ),
            (
                "rankedAxes",
                Value::Array(
                    self.element
                        .ranked_axes
                        .iter()
                        .map(|axis| text(axis_name(*axis)))
                        .collect(),
                ),
            ),
            ("indexes", texts(&self.indexes)),
            (
                "notes",
                Value::Array(
                    self.notes
                        .iter()
                        .map(|note| {
                            map(vec![
                                ("code", text(note.code())),
                                ("text", text(&note.text())),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "alternative",
                self.alternative
                    .as_ref()
                    .map(|alternative| {
                        map(vec![
                            ("when", text(alternative.when)),
                            ("node", alternative.node.to_value()),
                        ])
                    })
                    .unwrap_or(Value::Null),
            ),
            (
                "children",
                Value::Array(self.children.iter().map(LayoutNode::to_value).collect()),
            ),
        ])
    }
}

impl DocumentTypeLayout {
    /// The layout as a plain value: `{ documentType, root }`.
    pub fn to_value(&self) -> Value {
        map(vec![
            ("documentType", text(&self.document_type)),
            ("root", self.root.to_value()),
        ])
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::drive::{Drive, RootTree};
    use crate::structure::{drive_structure, ElementKind, StructureNode};
    use crate::util::test_helpers::setup::{
        setup_document, setup_drive_with_initial_state_structure,
    };
    use crate::util::test_helpers::setup_contract;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::DataContract;
    use dpp::document::{Document, DocumentV0Getters};
    use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
    use grovedb::{Element, PathQuery, Query, SizedQuery};
    use std::collections::BTreeSet;

    /// Contracts covering the index shapes Drive lays out: plain, unique and
    /// compound indexes, history, countable and summable types and indexes,
    /// ranked and chained indexes, time windows, indexOnly types with
    /// terminals, flat and preallocated indexes.
    const CONTRACTS: [&str; 19] = [
        "tests/supporting_files/contract/family/family-contract.json",
        "tests/supporting_files/contract/family/family-contract-fields-optional.json",
        "tests/supporting_files/contract/family/family-contract-countable.json",
        "tests/supporting_files/contract/family/family-contract-with-history.json",
        "tests/supporting_files/contract/dashpay/dashpay-contract.json",
        "tests/supporting_files/contract/references/references_with_contract_history.json",
        "tests/supporting_files/contract/restaurants/restaurants-contract.json",
        "tests/supporting_files/contract/trending/trending-contract.json",
        "tests/supporting_files/contract/trending/trending-sibling-contract.json",
        "tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json",
        "tests/supporting_files/contract/yappr-likes/yappr-likes-preallocated-contract.json",
        "tests/supporting_files/contract/yappr-likes/yappr-likes-author-preallocated-contract.json",
        "tests/supporting_files/contract/yappr-feed/yappr-feed-contract.json",
        "tests/supporting_files/contract/index-only-scalar-terminal/index-only-scalar-terminal-contract.json",
        "tests/supporting_files/contract/tally/tally-contract.json",
        "tests/supporting_files/contract/tip-jar/tip-jar-contract.json",
        "tests/supporting_files/contract/grades/grades-contract.json",
        "tests/supporting_files/contract/grades/grades-ranked-contract.json",
        "tests/supporting_files/contract/grades/grades-compound-ranked-contract.json",
    ];

    fn layer(drive: &Drive, path: &[Vec<u8>]) -> Vec<(Vec<u8>, Element)> {
        let mut query = Query::new();
        query.insert_all();
        let path_query = PathQuery::new(path.to_vec(), SizedQuery::new(query, None, None));
        let (elements, _) = drive
            .grove_get_raw_path_query(
                &path_query,
                None,
                QueryKeyElementPairResultType,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("expected to read a layer");
        elements.to_key_elements()
    }

    fn wrapper_of(element: &Element) -> Option<ZeroContributionWrapper> {
        match element {
            Element::NonCounted(_) => Some(ZeroContributionWrapper::NonCounted),
            Element::NotSummed(_) => Some(ZeroContributionWrapper::NotSummed),
            Element::NotCountedOrSummed(_) => Some(ZeroContributionWrapper::NotCountedOrSummed),
            _ => None,
        }
    }

    fn describe(node: &LayoutNode) -> String {
        match &node.key {
            LayoutKey::Fixed { label, .. } => format!("{:?} {label:?}", node.role),
            other => format!("{:?} {other:?}", node.role),
        }
    }

    /// What the walk saw, across every fixture.
    #[derive(Default)]
    struct Walk {
        mismatches: Vec<String>,
        roles: BTreeSet<&'static str>,
        wrappers: usize,
        ranked: usize,
        buckets: usize,
        alternatives: usize,
        members: usize,
    }

    impl Walk {
        /// Checks every element under `path`, whose element `node` describes.
        fn layer(&mut self, drive: &Drive, path: Vec<Vec<u8>>, node: &LayoutNode) {
            for (key, element) in layer(drive, &path) {
                let fixed = node.children.iter().find(
                    |child| matches!(&child.key, LayoutKey::Fixed { bytes, .. } if bytes == &key),
                );
                let Some(described) = fixed.or_else(|| {
                    node.children
                        .iter()
                        .find(|child| !matches!(child.key, LayoutKey::Fixed { .. }))
                }) else {
                    self.mismatches.push(format!(
                        "under {} at {}: key {} is not in the layout",
                        describe(node),
                        hex::encode(path.concat()),
                        hex::encode(&key)
                    ));
                    continue;
                };

                let kind = format!("{:?}", ElementKind::of(&element));
                let wrapper = wrapper_of(&element);
                let matches = |candidate: &LayoutNode| {
                    candidate.element.kind.name() == kind && candidate.element.wrapper == wrapper
                };
                let chosen = if matches(described) {
                    described
                } else if let Some(alternative) = described
                    .alternative
                    .as_ref()
                    .map(|alternative| &alternative.node)
                    .filter(|alternative| matches(alternative))
                {
                    self.alternatives += 1;
                    alternative
                } else {
                    self.mismatches.push(format!(
                        "{} at {} key {}: Drive wrote {kind}{} where the layout says {}{}",
                        describe(described),
                        hex::encode(path.concat()),
                        hex::encode(&key),
                        wrapper.map(|w| format!(" in {w:?}")).unwrap_or_default(),
                        described.element.kind.name(),
                        described
                            .element
                            .wrapper
                            .map(|w| format!(" in {w:?}"))
                            .unwrap_or_default(),
                    ));
                    continue;
                };

                self.roles.insert(chosen.role.name());
                self.wrappers += usize::from(chosen.element.wrapper.is_some());
                self.ranked += usize::from(!chosen.element.ranked_axes.is_empty());
                self.buckets +=
                    usize::from(matches!(chosen.key, LayoutKey::TimeRangeBucket { .. }));
                self.members += usize::from(matches!(chosen.role, LayoutRole::Member));
                if matches!(chosen.element.kind, LayoutElementKind::Tree(_)) {
                    let mut below = path.clone();
                    below.push(key);
                    self.layer(drive, below, chosen);
                }
            }
        }
    }

    fn apply(drive: &Drive, index: usize, path: &str) -> DataContract {
        let platform_version = PlatformVersion::latest();
        let contract = setup_contract(
            drive,
            path,
            Some([index as u8 + 1; 32]),
            None,
            None::<fn(&mut DataContract)>,
            None,
            Some(platform_version),
        );
        for document_type in contract.document_types().values() {
            for seed in 1..8 {
                let mut document = document_type
                    .random_document(Some(seed), platform_version)
                    .expect("expected a random document");
                small_sums(&mut document, document_type.as_ref(), seed);
                if seed <= 2 {
                    leave_out_optional_unique_values(&mut document, document_type.as_ref());
                }
                setup_document(drive, &document, &contract, document_type.as_ref(), None);
            }
        }
        contract
    }

    /// Leaves out the optional properties of the type's unique indexes, so a
    /// unique index gets entries with null values (two such documents share
    /// a key, so they go in a tree by id).
    fn leave_out_optional_unique_values(document: &mut Document, document_type: DocumentTypeRef) {
        let required = document_type.required_fields();
        for index in document_type
            .indexes()
            .values()
            .filter(|index| index.unique)
        {
            for property in &index.properties {
                if !property.name.starts_with('$') && !required.contains(&property.name) {
                    document.properties_mut().remove(&property.name);
                }
            }
        }
    }

    /// Random integers ignore the schema's bounds, and a few of them overflow
    /// a sum tree; give each summed property a small value instead (within
    /// the bounds of every fixture: 1 to 7).
    fn small_sums(document: &mut Document, document_type: DocumentTypeRef, seed: u64) {
        let summed = document_type
            .documents_summable()
            .map(str::to_string)
            .into_iter()
            .chain(
                document_type
                    .indexes()
                    .values()
                    .filter_map(|index| index.summable.clone()),
            )
            .collect::<BTreeSet<_>>();
        for property in summed {
            if let Some(value) = document.properties_mut().get_mut(&property) {
                let small = match &*value {
                    Value::U8(_) => Value::U8(seed as u8),
                    Value::I8(_) => Value::I8(seed as i8),
                    Value::U16(_) => Value::U16(seed as u16),
                    Value::I16(_) => Value::I16(seed as i16),
                    Value::U32(_) => Value::U32(seed as u32),
                    Value::I32(_) => Value::I32(seed as i32),
                    Value::U64(_) => Value::U64(seed),
                    Value::I64(_) => Value::I64(seed as i64),
                    other => other.clone(),
                };
                *value = small;
            }
        }
    }

    #[test]
    fn layout_matches_what_drive_writes() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let mut walk = Walk::default();

        for (index, path) in CONTRACTS.into_iter().enumerate() {
            let contract = apply(&drive, index, path);
            let documents_path = vec![
                vec![RootTree::DataContractDocuments as u8],
                contract.id().to_vec(),
                vec![1],
            ];
            for (name, document_type) in contract.document_types() {
                let layout = document_type_layout(document_type.as_ref(), platform_version)
                    .expect("expected a layout");
                let type_path = [documents_path.clone(), vec![name.as_bytes().to_vec()]].concat();
                walk.layer(&drive, type_path, &layout.root);
            }
        }

        assert!(
            walk.mismatches.is_empty(),
            "the layout disagrees with what Drive wrote:\n{}",
            walk.mismatches.join("\n")
        );
        for role in [
            "primaryKey",
            "document",
            "latestRevision",
            "revision",
            "indexProperty",
            "indexValue",
            "nextIndexProperty",
            "terminal",
            "member",
        ] {
            assert!(
                walk.roles.contains(role),
                "no fixture reached a {role} layer"
            );
        }
        assert!(walk.members > 0, "no fixture wrote an index entry");
        assert!(
            walk.wrappers > 0,
            "no fixture wrote a zero-contribution wrapper"
        );
        assert!(walk.ranked > 0, "no fixture wrote a ranked (indexed) tree");
        assert!(walk.buckets > 0, "no fixture wrote a time window");
        assert!(
            walk.alternatives > 0,
            "no fixture wrote a unique index entry with a null value"
        );
    }

    #[test]
    fn every_layer_names_a_node_of_the_structure_description() {
        fn ids(node: &StructureNode, out: &mut BTreeSet<String>) {
            out.insert(node.id.to_string());
            for child in &node.children {
                ids(child, out);
            }
        }
        let mut known = BTreeSet::new();
        ids(&drive_structure(), &mut known);
        for role in [
            LayoutRole::DocumentType,
            LayoutRole::PrimaryKey,
            LayoutRole::Document,
            LayoutRole::LatestRevision,
            LayoutRole::Revision,
            LayoutRole::IndexProperty,
            LayoutRole::IndexValue,
            LayoutRole::NextIndexProperty,
            LayoutRole::Terminal,
            LayoutRole::Member,
        ] {
            assert!(
                known.contains(role.structure_node()),
                "{} is not a node of the structure description",
                role.structure_node()
            );
        }
    }

    #[test]
    fn a_type_with_history_stores_each_document_as_a_tree_of_revisions() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = apply(&drive, 0, CONTRACTS[3]);
        let person = contract.document_type_for_name("person").expect("person");
        let layout = document_type_layout(person, platform_version).expect("layout");

        let primary = &layout.root.children[0];
        assert_eq!(primary.role, LayoutRole::PrimaryKey);
        let document = &primary.children[0];
        assert_eq!(
            document.element.kind,
            LayoutElementKind::Tree(TreeType::NormalTree)
        );
        assert_eq!(
            document.children.iter().map(|c| c.role).collect::<Vec<_>>(),
            vec![LayoutRole::LatestRevision, LayoutRole::Revision]
        );
    }
}

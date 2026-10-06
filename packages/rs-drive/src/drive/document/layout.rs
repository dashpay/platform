//! The GroveDB layout of one document type: every tree and element Drive
//! writes under `[DataContractDocuments, contract id, 1, document type name]`,
//! with the element or tree type Drive picks for it, computed from the
//! document type alone.
//!
//! The tree types come from the functions the insert walkers call
//! (`primary_key_tree_type`, `index_level_tree_types_with_continuation_demotion`,
//! `terminal_member_tree_type`, `continuation_contributes_zero`,
//! `zero_contribution_wrapper`); the element choices and skips the walkers
//! make inline (history, unique terminals, indexOnly entries, sum-carrying
//! references, `skipIfAbsent` levels) are restated here and held to
//! the walkers by a test that inserts documents and compares what Drive wrote
//! with this layout, both ways (`tests::should_lay_out_what_drive_writes`).
//! Those are the rules of the v2 index walkers, so only a platform version
//! that selects them has a layout.
//!
//! Each node names the node of the static structure description
//! (`drive::document::structure`, published as `grovedb-structure.json`) it
//! is an instance of, so a viewer can link a concrete layer to the general
//! description of that kind of layer.

pub use crate::drive::document::index_level_tree_types::ZeroContributionWrapper;
use crate::drive::document::index_level_tree_types::{
    continuation_contributes_zero, index_level_tree_types_with_continuation_demotion,
    terminal_member_tree_type, zero_contribution_wrapper,
};
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::document::sdk_value::{map, number, text, texts};
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{
    is_flat_level_key, DocumentTypeRef, Index, IndexBucketing, IndexLevel, IndexLevelTypeInfo,
    IndexType,
};
use dpp::platform_value::Value;
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
    /// One key per integer window start of `property` that holds a
    /// document: the window start, encoded like the property.
    IntegerRangeBucket {
        /// The integer property the windows bucket.
        property: String,
        /// The length of a window.
        range: u64,
        /// The distance between window starts.
        step: u64,
        /// The shift of the window boundaries.
        phase: u64,
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

    /// The role's name, as `to_value` spells it.
    pub fn name(&self) -> &'static str {
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
    /// A sum: a `summableOffCountIndex` index's counter.
    SumItem,
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
            LayoutElementKind::SumItem => "SumItem",
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
    /// A level keyed by a property that `skipIfAbsent` indexes skip on, or a
    /// level only `skipIfAbsent` indexes pass through: a document without the
    /// property writes nothing in those indexes (on a stored type, an index
    /// through the level that does not skip on it writes the null key
    /// instead).
    SkipIfAbsent {
        /// The property.
        property: String,
        /// The indexes through this level that skip a document leaving the
        /// property out, in name order.
        indexes: Vec<String>,
    },
    /// A preallocated indexOnly index: its value trees and empty terminal
    /// are created when the referenced document is inserted.
    Preallocated,
    /// An indexOnly index whose entries outlive a delete of their document:
    /// a delete leaves them to expire with their window, and a create writes
    /// over one already standing at its key.
    OutlivesDelete,
    /// A time window level: a document written here lands in every window
    /// that contains its time, up to this many.
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
    /// An integer window level: a document lands in every window that
    /// contains its value, up to this many.
    IntegerRangeOverlap {
        /// The most windows one value falls in.
        windows: u64,
    },
    /// A `summableOffCountIndex` index's counter: one sum item per value,
    /// holding how many entries the source index keeps for it, in place of
    /// the value tree and its entries.
    SummableOffCountIndex {
        /// The index whose entries the counter counts.
        source: String,
    },
}

impl LayoutNote {
    /// The note's code, as `to_value` spells it.
    pub fn code(&self) -> &'static str {
        match self {
            LayoutNote::IndexOnly => "indexOnly",
            LayoutNote::NotNullSearchable => "notNullSearchable",
            LayoutNote::Contested => "contested",
            LayoutNote::SkipIfAbsent { .. } => "skipIfAbsent",
            LayoutNote::Preallocated => "preallocated",
            LayoutNote::OutlivesDelete => "outlivesDelete",
            LayoutNote::TimeRangeOverlap { .. } => "timeRangeOverlap",
            LayoutNote::TimeRangeTtl { .. } => "timeRangeTtl",
            LayoutNote::IntegerRangeOverlap { .. } => "integerRangeOverlap",
            LayoutNote::SummableOffCountIndex { .. } => "summableOffCountIndex",
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
            LayoutNote::SkipIfAbsent { property, indexes } => {
                format!(
                    "a document that leaves out {property} writes nothing in {}",
                    indexes.join(", ")
                )
            }
            LayoutNote::Preallocated => {
                "preallocated: the value trees and the empty terminal of this index are created \
                 when the referenced document is inserted"
                    .to_string()
            }
            LayoutNote::OutlivesDelete => "outlivesDelete: a delete leaves these entries to \
                 expire with their window, and a create writes over one already here"
                .to_string(),
            LayoutNote::TimeRangeOverlap { windows } => {
                format!(
                    "a document written here lands in every window containing its time: up to \
                     {windows}"
                )
            }
            LayoutNote::TimeRangeTtl { ttl_seconds } => format!(
                "entries expire {ttl_seconds} seconds after their window: written without \
                 storage flags and removed by a later cleanup"
            ),
            LayoutNote::IntegerRangeOverlap { windows } => {
                format!("a document lands in every window containing its value: up to {windows}")
            }
            LayoutNote::SummableOffCountIndex { source } => format!(
                "summableOffCountIndex: one sum item per value, holding how many entries \
                 {source} keeps for it; a create adds one and a delete takes one away"
            ),
        }
    }
}

/// The GroveDB layout of `document_type` at `platform_version`.
///
/// The layout follows the v2 index walkers (protocol version 14 on); a
/// version that selects other walkers is refused, since they pick other
/// trees and wrappers for some shapes.
pub fn document_type_layout(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<DocumentTypeLayout, Error> {
    match platform_version
        .drive
        .methods
        .document
        .insert
        .add_indices_for_index_level_for_contract_operations
    {
        2 => {}
        version => {
            return Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "document_type_layout".to_string(),
                known_versions: vec![2],
                received: version,
            }))
        }
    }
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
pub(crate) fn index_paths(document_type: DocumentTypeRef) -> Vec<(String, Vec<String>)> {
    document_type
        .indexes()
        .values()
        .map(|index| {
            let path = match index.flat_level_key() {
                Some(flat_key) => vec![flat_key],
                None => index
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
pub(crate) fn indexes_through(
    index_paths: &[(String, Vec<String>)],
    path: &[String],
) -> Vec<String> {
    index_paths
        .iter()
        .filter(|(_, index_path)| index_path.starts_with(path))
        .map(|(name, _)| name.clone())
        .collect()
}

/// The index whose path ends at `path`.
pub(crate) fn index_ending_at(
    index_paths: &[(String, Vec<String>)],
    path: &[String],
) -> Vec<String> {
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
    let tree_types = index_level_tree_types_with_continuation_demotion(level)?;

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
                kind: LayoutElementKind::Tree(tree_types.property_name_tree_type),
                wrapper: None,
                ranked_axes: tree_types.ranked_axes,
            },
            alternative: None,
            indexes: indexes_through(index_paths, &path),
            notes: vec![],
            children: vec![terminal_node(info, index_ending_at(index_paths, &path))],
        });
    }

    let property = match level.bucketing() {
        Some(bucketing) => bucketing.source().to_string(),
        None => level_key.to_string(),
    };
    let notes = skip_notes(document_type, &property, &path, index_paths);

    Ok(LayoutNode {
        key: fixed(level_key.as_bytes().to_vec(), level_key),
        role: LayoutRole::IndexProperty,
        element: LayoutElement {
            kind: LayoutElementKind::Tree(tree_types.property_name_tree_type),
            wrapper: None,
            ranked_axes: tree_types.ranked_axes,
        },
        alternative: None,
        indexes: indexes_through(index_paths, &path),
        notes,
        children: vec![value_node(
            document_type,
            level,
            tree_types.value_tree_type,
            level_key,
            &path,
            true,
            index_paths,
        )?],
    })
}

/// The notes of a level keyed by `property` on `path` when indexes through it
/// skip a document that leaves a property out (the walkers' rule: they build
/// the level only for a document some index through it takes part in). The
/// level's own property comes first. When every index through the level
/// skips, each skip property not bound higher on the path gets a note too:
/// a document leaving them all out never builds the level, as on a grid
/// only skip indexes use. System properties are never skip properties.
fn skip_notes(
    document_type: DocumentTypeRef,
    property: &str,
    path: &[String],
    index_paths: &[(String, Vec<String>)],
) -> Vec<LayoutNote> {
    let through_names = indexes_through(index_paths, path);
    let through: Vec<&Index> = document_type
        .indexes()
        .values()
        .filter(|index| through_names.contains(&index.name))
        .collect();
    let mut properties = vec![property.to_string()];
    if through
        .iter()
        .all(|index| !index.skip_if_absent_properties.is_empty())
    {
        let bound_higher = &path[..path.len().saturating_sub(1)];
        for skip_property in through
            .iter()
            .flat_map(|index| &index.skip_if_absent_properties)
        {
            if !bound_higher.contains(skip_property) && !properties.contains(skip_property) {
                properties.push(skip_property.clone());
            }
        }
    }
    properties
        .into_iter()
        .filter_map(|property| {
            let indexes: Vec<String> = through
                .iter()
                .filter(|index| index.skip_if_absent_properties.contains(&property))
                .map(|index| index.name.clone())
                .collect();
            (!indexes.is_empty()).then_some(LayoutNote::SkipIfAbsent { property, indexes })
        })
        .collect()
}

/// The values of one indexed property, with what hangs under each.
fn value_node(
    document_type: DocumentTypeRef,
    level: &IndexLevel,
    value_tree_type: TreeType,
    level_key: &str,
    path: &[String],
    top: bool,
    index_paths: &[(String, Vec<String>)],
) -> Result<LayoutNode, Error> {
    let mut notes = Vec::new();
    let key = match level.bucketing().filter(|_| top) {
        Some(IndexBucketing::Time(transform)) => {
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
        Some(IndexBucketing::Integer(transform)) => {
            notes.push(LayoutNote::IntegerRangeOverlap {
                windows: transform.overlap_factor(),
            });
            LayoutKey::IntegerRangeBucket {
                property: transform.source.clone(),
                range: transform.range,
                step: transform.step,
                phase: transform.phase,
            }
        }
        None => LayoutKey::PropertyValue {
            property: level_key.to_string(),
        },
    };

    // A `summableOffCountIndex` index keeps its counter at the value
    // position, where another index grows a value tree, and nothing continues
    // below it (registration refuses an index that would).
    if let Some(info) = level.summable_off_count_index_info() {
        notes.push(LayoutNote::SummableOffCountIndex {
            source: info.summable_off_count_index.clone().unwrap_or_default(),
        });
        if info.preallocated {
            notes.push(LayoutNote::Preallocated);
        }
        return Ok(LayoutNode {
            key,
            role: LayoutRole::IndexValue,
            element: element(LayoutElementKind::SumItem),
            alternative: None,
            indexes: indexes_through(index_paths, path),
            notes,
            children: vec![],
        });
    }

    let mut children = Vec::new();
    if let Some(info) = level.has_index_with_type() {
        children.push(terminal_node(info, index_ending_at(index_paths, path)));
    }

    let parent_counts_continuations = level.is_ranked_chain_level();
    for (sub_key, sub_level) in level.sub_levels() {
        let sub_tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
        let wrapper = if continuation_contributes_zero(
            value_tree_type,
            parent_counts_continuations,
            sub_level,
        ) {
            zero_contribution_wrapper(value_tree_type, sub_tree_types.property_name_tree_type)
                .map_err(|refusal| {
                    Error::Drive(DriveError::CorruptedContractIndexes(format!(
                        "index level {sub_key:?} cannot hang under a {} value tree: {refusal:?}",
                        tree_kind_name(value_tree_type)
                    )))
                })?
        } else {
            None
        };
        let mut sub_path = path.to_vec();
        sub_path.push(sub_key.clone());
        children.push(LayoutNode {
            key: fixed(sub_key.as_bytes().to_vec(), sub_key),
            role: LayoutRole::NextIndexProperty,
            element: LayoutElement {
                kind: LayoutElementKind::Tree(sub_tree_types.property_name_tree_type),
                wrapper,
                ranked_axes: sub_tree_types.ranked_axes,
            },
            alternative: None,
            indexes: indexes_through(index_paths, &sub_path),
            notes: skip_notes(document_type, sub_key, &sub_path, index_paths),
            children: vec![value_node(
                document_type,
                sub_level,
                sub_tree_types.value_tree_type,
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
    if info.outlives_delete {
        notes.push(LayoutNote::OutlivesDelete);
    }

    let members = |member: LayoutNode, indexes: Vec<String>, notes: Vec<LayoutNote>| LayoutNode {
        key: fixed(vec![0], "Members"),
        role: LayoutRole::Terminal,
        element: tree(terminal_member_tree_type(info)),
        alternative: None,
        indexes,
        notes,
        children: vec![member],
    };

    if let Some(components) = &info.terminal {
        // indexOnly: each entry is an item keyed by its terminal components.
        return members(
            leaf(
                LayoutKey::MemberKey {
                    components: components.clone(),
                },
                LayoutRole::Member,
                element(if summable {
                    LayoutElementKind::ItemWithSumItem
                } else {
                    LayoutElementKind::Item
                }),
            ),
            indexes,
            notes,
        );
    }

    let by_document = leaf(
        LayoutKey::DocumentId,
        LayoutRole::Member,
        reference(summable),
    );
    if !info.index_type.is_unique() {
        return members(by_document, indexes, notes);
    }

    // A unique index writes the reference straight at `[0]`, unless a
    // value is null: then several documents can share the key, and they
    // go in a tree by id like a non-unique index. The alternative does not
    // repeat the indexes and notes of the `[0]` it stands in for.
    LayoutNode {
        key: fixed(vec![0], "Unique reference"),
        role: LayoutRole::Terminal,
        element: reference(summable),
        alternative: Some(Box::new(LayoutAlternative {
            when: "an indexed value of the document is null",
            node: members(by_document, vec![], vec![]),
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
                ("rangeSeconds", number(*range_seconds)),
                ("stepSeconds", number(*step_seconds)),
                ("phaseSeconds", number(*phase_seconds)),
            ]),
            LayoutKey::IntegerRangeBucket {
                property,
                range,
                step,
                phase,
            } => map(vec![
                ("kind", text("integerRangeBucket")),
                ("property", text(property)),
                ("range", number(*range)),
                ("step", number(*step)),
                ("phase", number(*phase)),
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
    /// wrapper?, rankedAxes, indexes, notes, alternative?, children }`, with
    /// `wrapper` and `alternative` left out when there is none.
    pub fn to_value(&self) -> Value {
        let mut entries = vec![
            ("key", self.key.to_value()),
            ("role", text(self.role.name())),
            ("structureNode", text(self.role.structure_node())),
            ("element", text(self.element.kind.name())),
        ];
        if let Some(wrapper) = self.element.wrapper {
            entries.push(("wrapper", text(wrapper_name(wrapper))));
        }
        entries.push((
            "rankedAxes",
            Value::Array(
                self.element
                    .ranked_axes
                    .iter()
                    .map(|axis| text(axis_name(*axis)))
                    .collect(),
            ),
        ));
        entries.push(("indexes", texts(&self.indexes)));
        entries.push((
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
        ));
        if let Some(alternative) = &self.alternative {
            entries.push((
                "alternative",
                map(vec![
                    ("when", text(alternative.when)),
                    ("node", alternative.node.to_value()),
                ]),
            ));
        }
        entries.push((
            "children",
            Value::Array(self.children.iter().map(LayoutNode::to_value).collect()),
        ));
        map(entries)
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
    use crate::drive::document::fixture_contracts::{
        leave_out_optional_indexed_values, leave_out_optional_unique_values,
        leave_out_skip_properties, small_sums, CONTRACTS,
    };
    use crate::drive::{Drive, RootTree};
    use crate::structure::{drive_structure, ElementKind, StructureNode};
    use crate::util::test_helpers::setup::{
        setup_document, setup_drive_with_initial_state_structure,
    };
    use crate::util::test_helpers::setup_contract;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::DataContract;
    use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
    use grovedb::{Element, PathQuery, Query, SizedQuery};
    use std::collections::BTreeSet;

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

    /// Whether Drive may write nothing for `child` in a tree `parent` stands
    /// for: the terminal a document with only null values skips
    /// (`nullSearchable: false`), the entries of a preallocated terminal
    /// (created empty), a value tree's levels when only preallocation built
    /// it (`beyond_preallocation`: Drive wrote one of the tree's children
    /// preallocation does not build), and the levels and values the walkers
    /// build only for documents carrying a skip property. The first-level
    /// trees, created with the contract, are never absent.
    fn may_be_absent(parent: &LayoutNode, child: &LayoutNode, beyond_preallocation: bool) -> bool {
        let under_a_value = parent.role == LayoutRole::IndexValue;
        // A value tree preallocation built (for a referenced document, before
        // any entry) holds only the preallocated index's trees until a
        // document of another index sharing the value arrives.
        (under_a_value
            && !beyond_preallocation
            && parent.children.iter().any(preallocated)
            && !preallocated(child))
            || child.notes.contains(&LayoutNote::NotNullSearchable)
            // A level below a value that skip indexes skip on is only built
            // for documents carrying the property.
            || (under_a_value
                && child
                    .notes
                    .iter()
                    .any(|note| matches!(note, LayoutNote::SkipIfAbsent { .. })))
            || parent.notes.iter().any(|note| {
                matches!(
                    note,
                    LayoutNote::Preallocated | LayoutNote::SkipIfAbsent { .. }
                )
            })
    }

    /// Whether `node` or a level below it is a preallocated index's.
    fn preallocated(node: &LayoutNode) -> bool {
        node.notes.contains(&LayoutNote::Preallocated) || node.children.iter().any(preallocated)
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
        /// Checks every element under `path`, whose element `node` describes,
        /// and that every layer the layout names under it was written.
        fn layer(&mut self, drive: &Drive, path: Vec<Vec<u8>>, node: &LayoutNode) {
            let mut reached = vec![false; node.children.len()];
            for (key, element) in layer(drive, &path) {
                let position = node
                    .children
                    .iter()
                    .position(
                        |child| matches!(&child.key, LayoutKey::Fixed { bytes, .. } if bytes == &key),
                    )
                    .or_else(|| {
                        node.children
                            .iter()
                            .position(|child| !matches!(child.key, LayoutKey::Fixed { .. }))
                    });
                let Some(position) = position else {
                    self.mismatches.push(format!(
                        "under {} at {}: key {} is not in the layout",
                        describe(node),
                        hex::encode(path.concat()),
                        hex::encode(&key)
                    ));
                    continue;
                };
                reached[position] = true;
                let described = &node.children[position];

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

            let beyond_preallocation = node
                .children
                .iter()
                .zip(&reached)
                .any(|(child, reached)| *reached && !preallocated(child));
            for (child, reached) in node.children.iter().zip(reached) {
                if !reached && !may_be_absent(node, child, beyond_preallocation) {
                    self.mismatches.push(format!(
                        "under {} at {}: the layout has {} but Drive wrote nothing there",
                        describe(node),
                        hex::encode(path.concat()),
                        describe(child)
                    ));
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
                if (3..=4).contains(&seed) {
                    leave_out_skip_properties(&mut document, document_type.as_ref());
                }
                if (5..=6).contains(&seed) {
                    leave_out_optional_indexed_values(
                        &mut document,
                        document_type.as_ref(),
                        seed == 6,
                    );
                }
                setup_document(drive, &document, &contract, document_type.as_ref(), None);
            }
        }
        contract
    }

    #[test]
    fn should_lay_out_what_drive_writes() {
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
    fn should_name_a_node_of_the_structure_description_for_every_layer() {
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
    fn should_skip_only_an_optional_property_of_an_index_only_type_when_absent() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = apply(&drive, 0, CONTRACTS[9]);
        let mark = contract.document_type_for_name("mark").expect("mark");
        let layout = document_type_layout(mark, platform_version).expect("layout");
        let skipped = |label: &str| {
            layout
                .root
                .children
                .iter()
                .find(|c| matches!(&c.key, LayoutKey::Fixed { label: l, .. } if l == label))
                .map(|c| {
                    c.notes
                        .iter()
                        .any(|n| matches!(n, LayoutNote::SkipIfAbsent { .. }))
                })
                .expect("level")
        };
        // `a` is required, `b` is not
        assert!(!skipped("a"));
        assert!(skipped("b"));

        let tip = contract.document_type_for_name("tip").expect("tip");
        let layout = document_type_layout(tip, platform_version).expect("layout");
        assert!(layout.root.children.iter().all(|c| c
            .notes
            .iter()
            .all(|n| !matches!(n, LayoutNote::SkipIfAbsent { .. }))));
    }

    #[test]
    fn should_store_each_document_of_a_type_with_history_as_a_tree_of_revisions() {
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

    #[test]
    fn should_refuse_a_version_that_selects_other_index_walkers() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = apply(&drive, 0, CONTRACTS[0]);
        let person = contract.document_type_for_name("person").expect("person");
        let version_13 = PlatformVersion::get(13).expect("protocol version 13");
        assert!(matches!(
            document_type_layout(person, version_13),
            Err(Error::Drive(DriveError::UnknownVersionMismatch {
                received: 1,
                ..
            }))
        ));
        assert!(document_type_layout(person, platform_version).is_ok());
    }
}

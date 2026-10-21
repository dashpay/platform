use crate::drive::contract::structure::CONTRACT_FLAGS;
use crate::structure::{ElementKind, FlagsKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/document/paths.rs";
const DOCUMENT_FLAGS: &str =
    "The owner is the owner of the document, who is refunded when it is deleted. \
     Documents the system writes, and documents of a type declaring a `ttl`, carry no flags.";
const LEVEL_FLAGS: &str = "The owner is the owner of the document that created this level of the \
     index. Levels created with the contract carry the contract owner, and levels of \
     system data, or created by a document of a type declaring a `ttl`, carry no flags.";
const DOCUMENT: &str = "contracts.contract.documents.document_type.primary_key.document";
const REVISION: &str = "contracts.contract.documents.document_type.primary_key.document.revision";
const RETAINED_REVISION: &str =
    "contracts.contract.documents.document_type.history.document.revision";
const INDEX_PROPERTY: &str = "contracts.contract.documents.document_type.index_property";

const PRIMARY_KEY_TREES: [ElementKind; 8] = [
    ElementKind::Tree,
    ElementKind::CountTree,
    ElementKind::ProvableCountTree,
    ElementKind::SumTree,
    ElementKind::ProvableSumTree,
    ElementKind::CountSumTree,
    ElementKind::ProvableCountSumTree,
    ElementKind::ProvableCountProvableSumTree,
];

const INDEX_PROPERTY_TREES: [ElementKind; 10] = [
    ElementKind::Tree,
    ElementKind::CountTree,
    ElementKind::ProvableCountTree,
    ElementKind::SumTree,
    ElementKind::ProvableSumTree,
    ElementKind::CountSumTree,
    ElementKind::ProvableCountProvableSumTree,
    ElementKind::ProvableCountIndexedTree,
    ElementKind::ProvableSumIndexedTree,
    ElementKind::ProvableCountProvableSumIndexedTree,
];
const INDEX_PROPERTY_NOTE: &str = "A plain tree unless the index is range \
                                   countable, range summable or ranked, or the \
                                   level carries a ranking's counts or sums up \
                                   to an earlier level; see \
                                   ranked_index_tree_type.rs.";

/// One document type of a contract: its documents and its indexes
pub(crate) fn document_type() -> StructureNode {
    StructureNode::dynamic(
        "document_type",
        "document_type_name",
        KeyMatcher::Any,
        KeyEncoding::Utf8,
        "The document type name",
    )
    .kind(ElementKind::Tree)
    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
    .source(SOURCE)
    .describe(
        "One document type. Created with the contract, \
                   together with the first level of each index.",
    )
    .children(vec![
        StructureNode::fixed("primary_key", &[0], "PrimaryKey", "")
            .kinds(
                &PRIMARY_KEY_TREES,
                "Chosen by the document type's documentsCountable \
                                            and documentsSummable settings; see \
                                            primary_key_tree_type.rs.",
            )
            .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
            .lazy()
            .source("packages/rs-drive/src/drive/document/primary_key_tree_type.rs")
            .book("drive/document-count-trees.md")
            .describe(
                "The documents, by id. Absent for index only \
                           document types, which store no documents.",
            )
            .child(
                StructureNode::identifier("document", "document_id", "The document id")
                    .kinds(
                        &[
                            ElementKind::Item,
                            ElementKind::ItemWithSumItem,
                            ElementKind::Tree,
                            ElementKind::SumTree,
                            ElementKind::Reference,
                            ElementKind::ReferenceWithSumItem,
                        ],
                        "An item; with a sum when the document type is \
                             summable; a tree of revisions when it keeps \
                             history before protocol version 15, and a reference \
                             to its retained current revision from version 15.",
                    )
                    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], DOCUMENT_FLAGS)
                    .value("serialized document")
                    .reference(RETAINED_REVISION)
                    .describe("One document.")
                    .children(vec![
                        StructureNode::fixed("latest", &[0], "Latest", "")
                            .kinds(
                                &[ElementKind::Reference, ElementKind::ReferenceWithSumItem],
                                "With a sum when the document type is summable.",
                            )
                            .reference(REVISION)
                            .flags(
                                &[FlagsKind::EpochOwned, FlagsKind::None],
                                "The document owner when the contract can be deleted; otherwise no flags.",
                            )
                            .until(14)
                            .describe("A sibling reference to the newest revision."),
                        StructureNode::dynamic(
                            "revision",
                            "block_time",
                            KeyMatcher::Len(8),
                            KeyEncoding::U64Be,
                            "The block time of the revision in milliseconds, \
                                                    u64 big endian with the sign bit flipped",
                        )
                        .kind(ElementKind::Item)
                        .flags(&[FlagsKind::EpochOwned, FlagsKind::None], DOCUMENT_FLAGS)
                        .value("serialized document")
                        .until(14)
                        .describe("The document as it was at that time."),
                    ]),
            ),
        StructureNode::fixed("history", &[2], "History", "DOCUMENT_HISTORY_TREE_KEY")
            .kind(ElementKind::Tree)
            .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
            .since(15)
            .lazy()
            .describe("Retained revisions of a type that keeps history, separate from its current primary entries.")
            .child(
                StructureNode::identifier("document", "document_id", "The document id")
                    .kind(ElementKind::ProvableCountTree)
                    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], DOCUMENT_FLAGS)
                    .describe("One document's retained history, with an authenticated revision count.")
                    .child(
                        StructureNode::dynamic("revision", "block_time_and_revision", KeyMatcher::Len(16), KeyEncoding::Composite, "The block time followed by the revision, both unsigned u64 big endian")
                            .kind(ElementKind::Item)
                            .flags(&[FlagsKind::EpochOwned, FlagsKind::None], DOCUMENT_FLAGS)
                            .value("serialized document")
                            .describe("An immutable retained revision."),
                    ),
            ),
        StructureNode::dynamic(
            "index_property",
            "index_property_name",
            KeyMatcher::Any,
            KeyEncoding::Utf8,
            "The name of an index's next property; a time \
                                    or integer range property appends its grid as \
                                    #range#step",
        )
        .kinds(&INDEX_PROPERTY_TREES, INDEX_PROPERTY_NOTE)
        .flags(&[FlagsKind::EpochOwned, FlagsKind::None], LEVEL_FLAGS)
        .source("packages/rs-drive/src/drive/document/index_level_tree_types.rs")
        .book("drive/indexes.md")
        .describe(
            "One property of an index. Indexes sharing a \
                           prefix share these levels.",
        )
        .child(index_value()),
    ])
}

fn index_value() -> StructureNode {
    StructureNode::dynamic(
        "value",
        "property_value",
        KeyMatcher::Any,
        KeyEncoding::SerializedValue,
        "The property's value serialized for ordering; \
         empty for null; a window start for a time or \
         integer range property; one element for a typed \
         array, a document being under each distinct \
         element of its list",
    )
    .kinds(
        &[
            ElementKind::Tree,
            ElementKind::CountTree,
            ElementKind::SumTree,
            ElementKind::CountSumTree,
            ElementKind::ProvableCountSumTree,
            ElementKind::ProvableCountProvableSumTree,
            ElementKind::SumItem,
        ],
        "A plain tree unless the index is countable or \
         summable, so each value carries its count or \
         sum. At the last property of a \
         summableOffCountIndex index, a sum item \
         counting the source index's entries for the \
         value, in place of the tree.",
    )
    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], LEVEL_FLAGS)
    .describe(
        "Every document with this value, reached through \
         the rest of the index.",
    )
    .children(vec![
        StructureNode::fixed("members", &[0], "Members", "")
            .kinds(
                &[
                    ElementKind::Tree,
                    ElementKind::CountTree,
                    ElementKind::ProvableCountTree,
                    ElementKind::SumTree,
                    ElementKind::ProvableSumTree,
                    ElementKind::CountSumTree,
                    ElementKind::ProvableCountSumTree,
                    ElementKind::ProvableCountProvableSumTree,
                    ElementKind::Reference,
                    ElementKind::ReferenceWithSumItem,
                ],
                "The end of an index: a tree of members, or for a \
                 unique index the one reference itself.",
            )
            .flags(&[FlagsKind::EpochOwned, FlagsKind::None], LEVEL_FLAGS)
            .lazy()
            .reference(DOCUMENT)
            .describe("Where an index ends at this value.")
            .child(
                StructureNode::identifier(
                    "member",
                    "document_id",
                    "The document id; for index only document types \
                     the owner id or the referenced identifier",
                )
                .kinds(
                    &[
                        ElementKind::Reference,
                        ElementKind::ReferenceWithSumItem,
                        ElementKind::Item,
                        ElementKind::ItemWithSumItem,
                    ],
                    "A reference to the document; for index only \
                     document types an item holding the row's \
                     commitment. With a sum under a summable index.",
                )
                .flags(&[FlagsKind::EpochOwned, FlagsKind::None], DOCUMENT_FLAGS)
                .reference(DOCUMENT)
                .value(
                    "for index only document types, a 32 byte row \
                     commitment",
                )
                .describe("One document with this value."),
            ),
        StructureNode::dynamic(
            "next_property",
            "index_property_name",
            KeyMatcher::Any,
            KeyEncoding::Utf8,
            "The name of the index's next property",
        )
        .kinds(&INDEX_PROPERTY_TREES, INDEX_PROPERTY_NOTE)
        .flags(&[FlagsKind::EpochOwned, FlagsKind::None], LEVEL_FLAGS)
        .recurse(INDEX_PROPERTY)
        .describe(
            "The next property of a compound index, shaped \
             like the level above.",
        ),
    ])
}

use crate::drive::votes::paths::{
    ACTIVE_POLLS_TREE_KEY, CONTESTED_DOCUMENT_INDEXES_TREE_KEY,
    CONTESTED_DOCUMENT_STORAGE_TREE_KEY, CONTESTED_RESOURCE_TREE_KEY, END_DATE_QUERIES_TREE_KEY,
    IDENTITY_VOTES_TREE_KEY, RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32,
    RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, RESOURCE_STORED_INFO_KEY_U8_32, VOTE_DECISIONS_TREE_KEY,
    VOTING_STORAGE_TREE_KEY,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/votes/paths.rs";
const CONTESTED_DOCUMENT: &str =
    "votes.contested_resource.active_polls.contract.document_type.storage.document";
const INDEX_VALUE: &str =
    "votes.contested_resource.active_polls.contract.document_type.indexes.value";

fn voting_storage() -> StructureNode {
    StructureNode::fixed(
        "votes",
        &[VOTING_STORAGE_TREE_KEY],
        "VotingStorage",
        "VOTING_STORAGE_TREE_KEY",
    )
    .kind(ElementKind::SumTree)
    .describe("The votes for this choice; the sum is the tally.")
    .child(
        StructureNode::identifier(
            "voter",
            "pro_tx_hash",
            "The voting masternode's pro tx hash",
        )
        .kind(ElementKind::SumItem)
        .value(
            "vote strength: 1 for a masternode, 4 for an \
             evonode",
        )
        .describe("One masternode's vote."),
    )
}

/// Masternode voting
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "votes",
        &[RootTree::Votes as u8],
        "Votes",
        "RootTree::Votes",
    )
    .kind(ElementKind::Tree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .describe(
        "Masternode votes, today on contested resources \
         such as premium DPNS names.",
    )
    .children(vec![
        StructureNode::fixed(
            "contested_resource",
            &[CONTESTED_RESOURCE_TREE_KEY as u8],
            "ContestedResource",
            "CONTESTED_RESOURCE_TREE_KEY",
        )
        .ascii()
        .kind(ElementKind::Tree)
        .source(SOURCE)
        .describe(
            "Votes deciding which identity gets a contested \
             unique index value.",
        )
        .children(vec![identity_votes(), active_polls()]),
        StructureNode::fixed(
            "decisions",
            &[VOTE_DECISIONS_TREE_KEY as u8],
            "Decisions",
            "VOTE_DECISIONS_TREE_KEY",
        )
        .ascii()
        .kind(ElementKind::Tree)
        .source(SOURCE)
        .describe(
            "Reserved for polls that decide something for the \
             network. Nothing writes to it yet.",
        ),
        StructureNode::fixed(
            "end_date_queries",
            &[END_DATE_QUERIES_TREE_KEY as u8],
            "EndDateQueries",
            "END_DATE_QUERIES_TREE_KEY",
        )
        .ascii()
        .kind(ElementKind::Tree)
        .source(SOURCE)
        .describe(
            "Polls by the time they end, so each block can \
             close the ones that are due.",
        )
        .child(
            StructureNode::dynamic(
                "end_date",
                "end_date",
                KeyMatcher::Len(8),
                KeyEncoding::U64Be,
                "The end time in milliseconds, u64 big endian \
                 with the sign bit flipped",
            )
            .kind(ElementKind::Tree)
            .describe("The polls ending at this time.")
            .child(
                StructureNode::dynamic(
                    "poll",
                    "vote_poll_id",
                    KeyMatcher::Len(32),
                    KeyEncoding::Hash32,
                    "The double sha256 of the serialized vote poll",
                )
                .kind(ElementKind::Item)
                .value("serialized VotePoll")
                .describe("One poll ending at this time."),
            ),
        ),
    ])
}

fn identity_votes() -> StructureNode {
    StructureNode::fixed(
        "identity_votes",
        &[IDENTITY_VOTES_TREE_KEY as u8],
        "IdentityVotes",
        "IDENTITY_VOTES_TREE_KEY",
    )
    .ascii()
    .kind(ElementKind::Tree)
    .describe(
        "The votes each masternode cast, so they can be \
         listed and changed.",
    )
    .child(
        StructureNode::identifier(
            "voter",
            "pro_tx_hash",
            "The voting masternode's pro tx hash",
        )
        .kind(ElementKind::Tree)
        .describe("One masternode's votes.")
        .child(
            StructureNode::dynamic(
                "vote",
                "vote_poll_id",
                KeyMatcher::Len(32),
                KeyEncoding::Hash32,
                "The double sha256 of the serialized vote poll",
            )
            .kind(ElementKind::Item)
            .value(
                "bincode \
                 ContestedDocumentResourceVoteReferenceStorageForm: \
                 a reference path to the vote and how many times \
                 the masternode voted on the poll",
            )
            .describe(
                "Where the masternode's vote on this poll is. An \
                 item holding a reference path rather than a \
                 reference, so proofs do not carry the value it \
                 points at.",
            ),
        ),
    )
}

fn active_polls() -> StructureNode {
    StructureNode::fixed(
        "active_polls",
        &[ACTIVE_POLLS_TREE_KEY as u8],
        "ActivePolls",
        "ACTIVE_POLLS_TREE_KEY",
    )
    .ascii()
    .kind(ElementKind::Tree)
    .describe(
        "The polls in progress, laid out like the \
         contested index they decide.",
    )
    .child(
        StructureNode::identifier("contract", "contract_id", "The data contract id")
            .kind(ElementKind::Tree)
            .describe(
                "Created with a contract that has a contested \
                 index.",
            )
            .child(
                StructureNode::dynamic(
                    "document_type",
                    "document_type_name",
                    KeyMatcher::Any,
                    KeyEncoding::Utf8,
                    "The document type name",
                )
                .kind(ElementKind::Tree)
                .describe("A document type with a contested index.")
                .children(vec![
                    StructureNode::fixed(
                        "storage",
                        &[CONTESTED_DOCUMENT_STORAGE_TREE_KEY],
                        "ContestedDocumentStorage",
                        "CONTESTED_DOCUMENT_STORAGE_TREE_KEY",
                    )
                    .kind(ElementKind::Tree)
                    .describe(
                        "The documents competing, held here until a poll \
                         awards one of them.",
                    )
                    .child(
                        StructureNode::identifier("document", "document_id", "The document id")
                            .kind(ElementKind::Item)
                            .value("serialized document")
                            .describe("One competing document."),
                    ),
                    StructureNode::fixed(
                        "indexes",
                        &[CONTESTED_DOCUMENT_INDEXES_TREE_KEY],
                        "ContestedDocumentIndexes",
                        "CONTESTED_DOCUMENT_INDEXES_TREE_KEY",
                    )
                    .kind(ElementKind::Tree)
                    .describe(
                        "The contested index. Only the values make levels \
                         here; property names are left out on purpose.",
                    )
                    .child(index_value()),
                ]),
            ),
    )
}

fn index_value() -> StructureNode {
    StructureNode::dynamic(
        "value",
        "index_value",
        KeyMatcher::Any,
        KeyEncoding::SerializedValue,
        "The value of the next index property; empty for \
         null",
    )
    .kind(ElementKind::Tree)
    .describe(
        "One value of the contested index. Below the last \
         value sit the poll's choices.",
    )
    .children(vec![
        StructureNode::fixed(
            "stored_info",
            &RESOURCE_STORED_INFO_KEY_U8_32,
            "StoredInfo",
            "RESOURCE_STORED_INFO_KEY_U8_32",
        )
        .kind(ElementKind::Item)
        .lazy()
        .value("serialized ContestedDocumentVotePollStoredInfo")
        .describe("The poll's status and, once it ends, its result."),
        StructureNode::fixed(
            "abstain",
            &RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32,
            "Abstain",
            "RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32",
        )
        .kind(ElementKind::Tree)
        .lazy()
        .describe("Votes to abstain.")
        .child(voting_storage()),
        StructureNode::fixed(
            "lock",
            &RESOURCE_LOCK_VOTE_TREE_KEY_U8_32,
            "Lock",
            "RESOURCE_LOCK_VOTE_TREE_KEY_U8_32",
        )
        .kind(ElementKind::Tree)
        .lazy()
        .describe("Votes to lock the value so nobody gets it.")
        .child(voting_storage()),
        StructureNode::identifier("contender", "identity_id", "The contending identity's id")
            .kind(ElementKind::Tree)
            .describe(
                "One contender, below the last value of the \
                 index. A 32 byte key is an index value at the \
                 levels before that, so what is below the key \
                 tells the two apart.",
            )
            .children(vec![
                StructureNode::fixed("document", &[0], "Document", "")
                    .kind(ElementKind::Reference)
                    .reference(CONTESTED_DOCUMENT)
                    .describe("The contender's document."),
                voting_storage(),
            ]),
        StructureNode::dynamic(
            "next_value",
            "index_value",
            KeyMatcher::Any,
            KeyEncoding::SerializedValue,
            "The value of the next index property; empty for \
             null",
        )
        .kind(ElementKind::Tree)
        .recurse(INDEX_VALUE)
        .describe(
            "The next property of the index, shaped like this \
             level.",
        ),
    ])
}

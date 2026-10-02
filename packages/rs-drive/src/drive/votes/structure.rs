use crate::drive::votes::paths::{
    ACTIVE_POLLS_TREE_KEY, CONTESTED_DOCUMENT_INDEXES_TREE_KEY,
    CONTESTED_DOCUMENT_STORAGE_TREE_KEY, CONTESTED_RESOURCE_TREE_KEY, END_DATE_QUERIES_TREE_KEY,
    IDENTITY_VOTES_TREE_KEY, READINESS_CONTRACTS_TREE_KEY, READINESS_CURRENT_ROUND_POINTER_KEY,
    READINESS_DEADLINES_TREE_KEY, READINESS_EVALUATION_CURSOR_KEY,
    READINESS_RETIRED_ROUNDS_TREE_KEY, READINESS_ROUND_RECORD_KEY,
    READINESS_ROUND_REPORTS_TREE_KEY, READINESS_ROUND_SCAN_CURSOR_KEY, READINESS_TREE_KEY,
    RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, RESOURCE_LOCK_VOTE_TREE_KEY_U8_32,
    RESOURCE_STORED_INFO_KEY_U8_32, VOTE_DECISIONS_TREE_KEY, VOTING_STORAGE_TREE_KEY,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, FlagsKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/votes/paths.rs";
const CONTENDER_FLAGS: &str =
    "The owner is the contender whose document created the level, who is \
     refunded when the poll is cleaned up. Written without storage flags, it carries none.";
const POLL_FLAGS: &str = "The owner is the identity whose contested document started the poll.";
const OWNED: [FlagsKind; 2] = [FlagsKind::EpochOwned, FlagsKind::None];
const CONTESTED_DOCUMENT: &str =
    "votes.contested_resource.active_polls.contract.document_type.storage.document";
const INDEX_VALUE: &str =
    "votes.contested_resource.active_polls.contract.document_type.indexes.value";
const INDEX_VALUE_TREES: [ElementKind; 2] = [ElementKind::Tree, ElementKind::CountTree];
const INDEX_VALUE_NOTE: &str =
    "A count tree for the last value of the index when the poll started at \
     protocol version 14 or later, counting its contenders plus its stored \
     info, abstain and lock entries, so a join reads how many contenders the \
     poll has in one fetch. A tree otherwise.";

fn voting_storage() -> StructureNode {
    StructureNode::fixed(
        "votes",
        &[VOTING_STORAGE_TREE_KEY],
        "VotingStorage",
        "VOTING_STORAGE_TREE_KEY",
    )
    .kind(ElementKind::SumTree)
    .flags(&OWNED, CONTENDER_FLAGS)
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
            .flags(&[FlagsKind::EpochOwned], POLL_FLAGS)
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
                .flags(&[FlagsKind::EpochOwned], POLL_FLAGS)
                .value("serialized VotePoll")
                .describe("One poll ending at this time."),
            ),
        ),
        readiness(),
    ])
}

/// Compilation readiness: the rounds evonodes report on before a contract's executable
/// bundle activates
fn readiness() -> StructureNode {
    StructureNode::fixed(
        "readiness",
        &[READINESS_TREE_KEY as u8],
        "Readiness",
        "READINESS_TREE_KEY",
    )
    .ascii()
    .kind(ElementKind::Tree)
    .since(17)
    .source(SOURCE)
    .book("drive/compilation-readiness.md")
    .describe(
        "Readiness rounds: a contract's executable bundle activates once \
         enough evonodes report that they have prepared it. The keys are \
         provisional.",
    )
    .children(vec![
        StructureNode::fixed(
            "contracts",
            &[READINESS_CONTRACTS_TREE_KEY],
            "Contracts",
            "READINESS_CONTRACTS_TREE_KEY",
        )
        .kind(ElementKind::Tree)
        .describe("The contracts holding a readiness round.")
        .child(
            StructureNode::identifier("contract", "contract_id", "The data contract id")
                .kind(ElementKind::Tree)
                .describe(
                    "Created with the contract's first round. Stays while a \
                     retired round still waits for its cleanup.",
                )
                .children(vec![
                    StructureNode::fixed(
                        "current_round",
                        &[READINESS_CURRENT_ROUND_POINTER_KEY as u8],
                        "CurrentRound",
                        "READINESS_CURRENT_ROUND_POINTER_KEY",
                    )
                    .ascii()
                    .kind(ElementKind::Item)
                    .until_deleted()
                    .value("round_id (32 bytes)")
                    .describe(
                        "The contract's current round. Replacing a round swaps \
                         the pointer; cancelling or activating it deletes it.",
                    ),
                    readiness_round(),
                ]),
        ),
        StructureNode::fixed(
            "deadlines",
            &[READINESS_DEADLINES_TREE_KEY],
            "Deadlines",
            "READINESS_DEADLINES_TREE_KEY",
        )
        .kind(ElementKind::Tree)
        .describe(
            "Crossed rounds by the time they activate, so each block can \
             activate the ones that are due.",
        )
        .child(
            StructureNode::dynamic(
                "deadline",
                "deadline_ms",
                KeyMatcher::Len(8),
                KeyEncoding::U64Be,
                "The activation deadline in milliseconds, u64 big endian \
                 with the sign bit flipped",
            )
            .kind(ElementKind::Tree)
            .describe(
                "The rounds activating at this time. Removed with its last \
                 entry.",
            )
            .child(
                StructureNode::identifier("entry", "contract_id", "The data contract id")
                    .kind(ElementKind::Item)
                    .value("round_id (32 bytes)")
                    .describe(
                        "The round the contract activates at this time. Stale, \
                         and dropped without activation, once it is no longer the \
                         contract's current round.",
                    ),
            ),
        ),
        StructureNode::fixed(
            "evaluation_cursor",
            &[READINESS_EVALUATION_CURSOR_KEY],
            "EvaluationCursor",
            "READINESS_EVALUATION_CURSOR_KEY",
        )
        .kind(ElementKind::Item)
        .lazy()
        .value("contract_id (32 bytes)")
        .describe(
            "The last contract the block event evaluated, so the next block \
             continues after it.",
        ),
        StructureNode::fixed(
            "retired_rounds",
            &[READINESS_RETIRED_ROUNDS_TREE_KEY],
            "RetiredRounds",
            "READINESS_RETIRED_ROUNDS_TREE_KEY",
        )
        .kind(ElementKind::Tree)
        .describe(
            "Rounds replaced, cancelled or activated, waiting for the bounded \
             cleanup that deletes their trees.",
        )
        .child(
            StructureNode::dynamic(
                "retired_round",
                "round_id",
                KeyMatcher::Len(32),
                KeyEncoding::Hash32,
                "The round id, a double sha256 of the network, the contract, \
                 the version, the bundle digest and the height it was accepted at",
            )
            .kind(ElementKind::Item)
            .value("contract_id (32 bytes)")
            .describe("One retired round and the contract its tree is under."),
        ),
    ])
}

fn readiness_round() -> StructureNode {
    StructureNode::dynamic(
        "round",
        "round_id",
        KeyMatcher::Len(32),
        KeyEncoding::Hash32,
        "The round id, a double sha256 of the network, the contract, the \
         version, the bundle digest and the height it was accepted at",
    )
    .kind(ElementKind::Tree)
    .describe(
        "One round, normally only the current one. A retired round stays \
         here, unreachable through the pointer, until its cleanup.",
    )
    .children(vec![
        StructureNode::fixed(
            "record",
            &[READINESS_ROUND_RECORD_KEY],
            "Record",
            "READINESS_ROUND_RECORD_KEY",
        )
        .kind(ElementKind::Item)
        .value("serialized ReadinessRound")
        .describe(
            "The bundle, the status, the last evaluation mark and the fund \
             of the round.",
        ),
        StructureNode::fixed(
            "reports",
            &[READINESS_ROUND_REPORTS_TREE_KEY],
            "Reports",
            "READINESS_ROUND_REPORTS_TREE_KEY",
        )
        .kind(ElementKind::CountTree)
        .describe(
            "The accepted reports; the count is the raw number of distinct \
             reporters.",
        )
        .child(
            StructureNode::identifier(
                "report",
                "pro_tx_hash",
                "The reporting evonode's pro tx hash",
            )
            .kind(ElementKind::Item)
            .value("serialized ReadinessReportRecord")
            .describe("One evonode's report that it prepared the bundle."),
        ),
        StructureNode::fixed(
            "scan_cursor",
            &[READINESS_ROUND_SCAN_CURSOR_KEY],
            "ScanCursor",
            "READINESS_ROUND_SCAN_CURSOR_KEY",
        )
        .kind(ElementKind::Item)
        .lazy()
        .value("serialized ReadinessScanCursor")
        .describe(
            "Where a walk over the reports that did not finish in one block \
             continues. Absent when no walk is open.",
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
                            .flags(&OWNED, CONTENDER_FLAGS)
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
    .kinds(&INDEX_VALUE_TREES, INDEX_VALUE_NOTE)
    .flags(&OWNED, CONTENDER_FLAGS)
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
        .flags(&OWNED, CONTENDER_FLAGS)
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
        .flags(&OWNED, CONTENDER_FLAGS)
        .lazy()
        .describe("Votes to lock the value so nobody gets it.")
        .child(voting_storage()),
        StructureNode::identifier("contender", "identity_id", "The contending identity's id")
            .kind(ElementKind::Tree)
            .flags(&OWNED, CONTENDER_FLAGS)
            .describe(
                "One contender, below the last value of the \
                 index. A 32 byte key is an index value at the \
                 levels before that, so what is below the key \
                 tells the two apart.",
            )
            .children(vec![
                StructureNode::fixed("document", &[0], "Document", "")
                    .kind(ElementKind::Reference)
                    .flags(&OWNED, CONTENDER_FLAGS)
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
        .kinds(&INDEX_VALUE_TREES, INDEX_VALUE_NOTE)
        .flags(&OWNED, CONTENDER_FLAGS)
        .recurse(INDEX_VALUE)
        .describe(
            "The next property of the index, shaped like this \
             level.",
        ),
    ])
}

use crate::drive::votes::paths::{
    ACTIVE_POLLS_TREE_KEY, CONTESTED_DOCUMENT_INDEXES_TREE_KEY,
    CONTESTED_DOCUMENT_STORAGE_TREE_KEY, CONTESTED_RESOURCE_TREE_KEY, END_DATE_QUERIES_TREE_KEY,
    IDENTITY_VOTES_TREE_KEY, RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32,
    RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, RESOURCE_STORED_INFO_KEY_U8_32, VOTE_DECISIONS_TREE_KEY,
    VOTING_STORAGE_TREE_KEY, YES_NO_VOTE_POLL_ABSTAIN_VOTES_TREE_KEY,
    YES_NO_VOTE_POLL_NO_VOTES_TREE_KEY, YES_NO_VOTE_POLL_STORED_INFO_KEY,
    YES_NO_VOTE_POLL_YES_VOTES_TREE_KEY,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, FlagsKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/votes/paths.rs";
const CONTENDER_FLAGS: &str =
    "The owner is the contender whose document created the level, who is \
     refunded when the poll is cleaned up. Written without storage flags, it carries none.";
const POLL_FLAGS: &str = "The owner is the identity whose contested document started the \
                          poll. A yes/no poll is opened by the system and carries none.";
const POLL: [FlagsKind; 2] = [FlagsKind::EpochOwned, FlagsKind::None];
const OWNED: [FlagsKind; 2] = [FlagsKind::EpochOwned, FlagsKind::None];
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
        "Masternode votes: on contested resources such as \
         premium DPNS names, and on yes/no decisions.",
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
            "Yes/no polls: decisions taken by a supermajority \
             of the masternodes once enough voting power was \
             cast.",
        )
        .children(vec![decision_identity_votes(), decision_active_polls()]),
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
            .flags(&POLL, POLL_FLAGS)
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
                .flags(&POLL, POLL_FLAGS)
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
    .kind(ElementKind::Tree)
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
        .kind(ElementKind::Tree)
        .flags(&OWNED, CONTENDER_FLAGS)
        .recurse(INDEX_VALUE)
        .describe(
            "The next property of the index, shaped like this \
             level.",
        ),
    ])
}

fn decision_vote_choice(
    segment: &str,
    key: u8,
    label: &str,
    constant: &str,
    what: &str,
) -> StructureNode {
    StructureNode::fixed(segment, &[key], label, constant)
        .kind(ElementKind::SumTree)
        .until_deleted()
        .describe(what)
        .child(
            StructureNode::identifier(
                "voter",
                "pro_tx_hash",
                "The voting masternode's pro tx hash",
            )
            .kind(ElementKind::SumItem)
            .value("vote strength: 1 for a masternode, 4 for an evonode")
            .describe("One masternode's vote; the tree's sum is the tally."),
        )
}

fn decision_active_polls() -> StructureNode {
    StructureNode::fixed(
        "active_polls",
        &[ACTIVE_POLLS_TREE_KEY as u8],
        "DecisionActivePolls",
        "ACTIVE_POLLS_TREE_KEY",
    )
    .ascii()
    .kind(ElementKind::Tree)
    .since(14)
    .source(SOURCE)
    .describe("The yes/no polls, by their unique id.")
    .child(
        StructureNode::dynamic(
            "poll",
            "vote_poll_id",
            KeyMatcher::Len(32),
            KeyEncoding::Hash32,
            "The double sha256 of the serialized vote poll",
        )
        .kind(ElementKind::Tree)
        .describe(
            "One yes/no poll: its stored info and its three \
             vote sum trees. The vote trees go when the poll \
             ends; the stored info stays as the record of the \
             decision.",
        )
        .children(vec![
            StructureNode::fixed(
                "stored_info",
                &[YES_NO_VOTE_POLL_STORED_INFO_KEY],
                "YesNoStoredInfo",
                "YES_NO_VOTE_POLL_STORED_INFO_KEY",
            )
            .kind(ElementKind::Item)
            .value("serialized YesNoVotePollStoredInfo")
            .describe("The poll's status and, once it ends, its result."),
            decision_vote_choice(
                "yes",
                YES_NO_VOTE_POLL_YES_VOTES_TREE_KEY,
                "YesVotes",
                "YES_NO_VOTE_POLL_YES_VOTES_TREE_KEY",
                "Votes for yes.",
            ),
            decision_vote_choice(
                "no",
                YES_NO_VOTE_POLL_NO_VOTES_TREE_KEY,
                "NoVotes",
                "YES_NO_VOTE_POLL_NO_VOTES_TREE_KEY",
                "Votes for no.",
            ),
            decision_vote_choice(
                "abstain",
                YES_NO_VOTE_POLL_ABSTAIN_VOTES_TREE_KEY,
                "AbstainVotes",
                "YES_NO_VOTE_POLL_ABSTAIN_VOTES_TREE_KEY",
                "Votes to abstain, which count towards nothing.",
            ),
        ]),
    )
}

fn decision_identity_votes() -> StructureNode {
    StructureNode::fixed(
        "identity_votes",
        &[IDENTITY_VOTES_TREE_KEY as u8],
        "DecisionIdentityVotes",
        "IDENTITY_VOTES_TREE_KEY",
    )
    .ascii()
    .kind(ElementKind::Tree)
    .since(14)
    .source(SOURCE)
    .describe(
        "The yes/no votes each masternode cast, so they can \
         be changed and removed with the masternode.",
    )
    .child(
        StructureNode::identifier(
            "voter",
            "pro_tx_hash",
            "The voting masternode's pro tx hash",
        )
        .kind(ElementKind::Tree)
        .describe(
            "One masternode's yes/no votes. Stays once its \
             votes are gone, like its contested counterpart.",
        )
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
                "bincode YesNoVoteReferenceStorageForm: the \
                 choice and how many times the masternode voted \
                 on the poll",
            )
            .describe("The masternode's current answer to one poll."),
        ),
    )
}

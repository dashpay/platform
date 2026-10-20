use crate::drive::contract::paths::{
    CONTRACT_BANLIST_KEY, CONTRACT_DOCUMENT_REMOVALS_KEY, CONTRACT_LAST_MODERATORS_FEE_CLAIM_KEY,
    CONTRACT_LAST_OWNER_FEE_CLAIM_KEY, CONTRACT_MODERATION_ACTION_COUNTS_KEY, CONTRACT_OTHER_KEY,
    CONTRACT_SUSPENSIONS_KEY, CONTRACT_TEAM_ACTIONS_KEY, CONTRACT_TEAM_ACTION_INFO_KEY,
    CONTRACT_TEAM_ACTION_SIGNERS_KEY, CONTRACT_TEAM_ACTIVE_ACTIONS_KEY,
    CONTRACT_TEAM_CLOSED_ACTIONS_KEY, CONTRACT_VERSION_KEY, CONTRACT_WARNINGS_KEY,
};
use crate::drive::document::structure::document_type;
use crate::drive::RootTree;
use crate::structure::{ElementKind, FlagsKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/contract/paths.rs";
/// The flags of the elements written with a contract, shared by every area that describes one
pub(crate) const CONTRACT_FLAGS: &str =
    "The contract's flags. Only the system contracts the upgrades to protocol versions 6, 9 \
     and 13 registered carry them (wallet utils, token history, keyword search and document \
     history), owned by the all-zero system owner in the epoch of the upgrade. Genesis, state \
     transitions and later upgrades write none.";
const REMOVAL_FLAGS: &str =
    "The owner is the moderator who deleted the document, restored it, or deleted it again \
     once restored, as GroveDB's flag merge passes the record on: a longer rewrite passes to \
     its writer, who pays for the added bytes; a shorter one refunds the bytes it frees to the \
     moderator the record named, and passes to its writer only within the epoch the record \
     was paid in (or once the record spans epochs), keeping the earlier moderator otherwise; \
     a rewrite of the same size keeps the earlier moderator. Nothing deletes it.";
const MODERATOR_FLAGS: &str =
    "The owner is the moderator who added the entry. They pay for it, and are \
     refunded when it is removed. A suspension replaced with a longer reason \
     passes to the moderator who replaced it, who pays for the added bytes; \
     replaced with a shorter or an equally long one it stays the first \
     moderator's, who is refunded the removed bytes.";

/// Data contracts and their documents
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "contracts",
        &[RootTree::DataContractDocuments as u8],
        "DataContractDocuments",
        "RootTree::DataContractDocuments",
    )
    .kind(ElementKind::Tree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("data-model/data-contracts.md")
    .describe(
        "Every data contract with its documents and their \
         indexes. The root of the root layer, since it is \
         read most.",
    )
    .child(
        StructureNode::identifier("contract", "contract_id", "The data contract id")
            .kind(ElementKind::Tree)
            .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
            .source(SOURCE)
            .describe("One data contract.")
            .children(vec![
                StructureNode::fixed("contract", &[0], "Contract", "")
                    .kinds(
                        &[ElementKind::Item, ElementKind::Tree],
                        "An item holding the contract; a tree of \
                         revisions when the contract keeps history.",
                    )
                    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                    .value("serialized DataContract")
                    .describe("The contract itself.")
                    .children(vec![
                        StructureNode::fixed("latest", &[0], "Latest", "")
                            .kind(ElementKind::Reference)
                            .reference("contracts.contract.contract.revision")
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
                        .value("serialized DataContract")
                        .describe("The contract as it was at that time."),
                    ]),
                StructureNode::fixed("documents", &[1], "Documents", "")
                    .kind(ElementKind::Tree)
                    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                    .book("drive/indexes.md")
                    .describe("The documents of the contract, by document type.")
                    .child(document_type()),
                StructureNode::fixed(
                    "other",
                    &[CONTRACT_OTHER_KEY],
                    "Other",
                    "CONTRACT_OTHER_KEY",
                )
                .kind(ElementKind::Tree)
                .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                .since(14)
                .book("data-model/contract-moderation.md")
                .describe(
                    "Everything a contract keeps beside itself and its \
                         documents. One key, so that the contract's layer \
                         holds three and its Merk keeps the documents on top. \
                         Inside, the keys are spread like the root layer's, \
                         with the most read one, the banlist, in the middle.",
                )
                .children(vec![
                    StructureNode::fixed(
                        "document_removals",
                        &[CONTRACT_DOCUMENT_REMOVALS_KEY],
                        "DocumentRemovals",
                        "CONTRACT_DOCUMENT_REMOVALS_KEY",
                    )
                    .kind(ElementKind::Tree)
                    .lazy()
                    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                    .describe(
                        "The records of the documents the contract's moderators \
                             deleted. Created with the first document type whose \
                             moderators' deletions keep records, by the contract's \
                             creation or by an update. Read by clients, never by a \
                             document transition, so it sorts below the rest.",
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
                        .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                        .describe(
                            "The removed documents of one document type whose \
                                 moderators' deletions keep records. Created with the \
                                 document type, by the contract's creation or by the update \
                                 that adds the type.",
                        )
                        .child(
                            StructureNode::identifier(
                                "document",
                                "document_id",
                                "The id the removed document had",
                            )
                            .kind(ElementKind::Item)
                            .flags(&[FlagsKind::EpochOwned], REMOVAL_FLAGS)
                            .value(
                                "the document owner's id, the moderator's id, the block \
                                     time in milliseconds of the removal as a u64 big \
                                     endian, the hash of the removed document, a tag byte \
                                     (bit 0: restored, bit 1: kept fields), the restoring \
                                     moderator's id and the restoration time when \
                                     restored, the kept fields when any (their length as a \
                                     u32 big endian, then the bytes), then the moderator's \
                                     reason as in a banlist entry",
                            )
                            .describe(
                                "One removal: whose document it was, who removed it, \
                                     when and why, what it was, and who restored it. Never \
                                     deleted: replaced in place by the restore that marks \
                                     it and by the deletion of a restored document, which \
                                     writes a fresh record over it.",
                            ),
                        ),
                    ),
                    StructureNode::fixed(
                        "team_actions",
                        &[CONTRACT_TEAM_ACTIONS_KEY],
                        "TeamActions",
                        "CONTRACT_TEAM_ACTIONS_KEY",
                    )
                    .kind(ElementKind::Tree)
                    .lazy()
                    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                    .describe(
                        "The actions the contract's seated moderation team votes on, \
                             shaped like a token group's: one member proposes, the others \
                             approve by the action's id, and the action runs once the \
                             approvals meet its rule. Today the deletion of a settled \
                             document, past its type's `deleteWithin`. Created with the \
                             contract when one of its document types sets \
                             `moderatorAbilities.deleteSettled`: no update can add such a \
                             type, since the elected declaration fixed at creation must give \
                             the team `deleteDocuments` on it. Read by the team and by \
                             clients, never by a document transition, so it sorts below the \
                             rest.",
                    )
                    .children(vec![
                        StructureNode::fixed(
                            "active",
                            CONTRACT_TEAM_ACTIVE_ACTIONS_KEY,
                            "ActiveTeamActions",
                            "CONTRACT_TEAM_ACTIVE_ACTIONS_KEY",
                        )
                        .ascii()
                        .kind(ElementKind::Tree)
                        .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                        .describe(
                            "Team actions still gathering approvals. Nothing lapses: an \
                                 action that never gets there stays.",
                        )
                        .child(team_action(
                            "One action in progress. Moved to the closed actions when the \
                                 approvals meet its rule.",
                            &[FlagsKind::EpochOwned],
                            "The owner is the member who proposed (the action) or approved \
                                 (an approval), in the epoch they did, refunded when the \
                                 action closes and moves.",
                        )),
                        StructureNode::fixed(
                            "closed",
                            CONTRACT_TEAM_CLOSED_ACTIONS_KEY,
                            "ClosedTeamActions",
                            "CONTRACT_TEAM_CLOSED_ACTIONS_KEY",
                        )
                        .ascii()
                        .kind(ElementKind::Tree)
                        .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                        .describe(
                            "Team actions that ran, kept for good with the approvals that \
                                 counted: those of members who left the team were dropped.",
                        )
                        .child(team_action(
                            "One action that ran.",
                            &[FlagsKind::None],
                            "Entries moved or written when an action closes carry no \
                                 flags: the member whose approval closed it pays for them.",
                        )),
                    ]),
                    StructureNode::fixed(
                        "moderation_action_counts",
                        &[CONTRACT_MODERATION_ACTION_COUNTS_KEY],
                        "ModerationActionCounts",
                        "CONTRACT_MODERATION_ACTION_COUNTS_KEY",
                    )
                    .kind(ElementKind::Tree)
                    .lazy()
                    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                    .describe(
                        "How many moderation actions each member of the contract's \
                             seated moderation team signed since its moderators pot \
                             was last settled. Created with a contract that declares \
                             elected moderation. Read by the team's actions and by a \
                             settle, never by a document transition, so it sorts \
                             below the version item.",
                    )
                    .child(
                        StructureNode::identifier(
                            "member",
                            "identity_id",
                            "The member of the seated team",
                        )
                        .kind(ElementKind::Item)
                        .flags(
                            &[FlagsKind::None],
                            "None: the member whose action writes the count pays \
                                 for it, and the settle that deletes it refunds nobody.",
                        )
                        .value("u32 big endian")
                        .describe(
                            "The member's count of bans, suspensions, warnings and \
                                 document deletions since the last settle. Rewritten \
                                 one higher by each; every settle, a claim or a change \
                                 of the team, pays the pot's action share by the counts \
                                 and deletes them.",
                        ),
                    ),
                    StructureNode::fixed(
                        "version",
                        &[CONTRACT_VERSION_KEY],
                        "ContractVersion",
                        "CONTRACT_VERSION_KEY",
                    )
                    .kind(ElementKind::Item)
                    .flags(&[FlagsKind::EpochOwned, FlagsKind::None], CONTRACT_FLAGS)
                    .value("u32 big endian")
                    .describe(
                        "The contract's version, readable without \
                             deserializing the contract. Rewritten on every \
                             update.",
                    ),
                    StructureNode::fixed(
                        "last_owner_fee_claim",
                        &[CONTRACT_LAST_OWNER_FEE_CLAIM_KEY],
                        "LastOwnerFeeClaim",
                        "CONTRACT_LAST_OWNER_FEE_CLAIM_KEY",
                    )
                    .kind(ElementKind::Item)
                    .lazy()
                    .value(
                        "42 bytes: epoch index, u16 big endian; block time in \
                             milliseconds, u64 big endian; claimant identity id",
                    )
                    .describe(
                        "The last claim of the contract's owner fee pot: the \
                             epoch and the block time it was paid out in, and \
                             the identity that claimed, which is the contract \
                             owner. Written by the first claim and replaced by \
                             every later one; a pot is claimed at most once per \
                             epoch.",
                    ),
                    StructureNode::fixed(
                        "last_moderators_fee_claim",
                        &[CONTRACT_LAST_MODERATORS_FEE_CLAIM_KEY],
                        "LastModeratorsFeeClaim",
                        "CONTRACT_LAST_MODERATORS_FEE_CLAIM_KEY",
                    )
                    .kind(ElementKind::Item)
                    .lazy()
                    .value(
                        "42 bytes: epoch index, u16 big endian; block time in \
                             milliseconds, u64 big endian; claimant identity id",
                    )
                    .describe(
                        "The last claim of the contract's moderators fee \
                             pot: the epoch and the block time it was paid out \
                             in, and the member of the moderation team that \
                             claimed it for the team. Written by the first \
                             claim and replaced by every later one; a pot is \
                             claimed at most once per epoch.",
                    ),
                    StructureNode::fixed(
                        "banlist",
                        &[CONTRACT_BANLIST_KEY],
                        "Banlist",
                        "CONTRACT_BANLIST_KEY",
                    )
                    .kind(ElementKind::Tree)
                    .lazy()
                    .describe(
                        "The identities banned from the contract's \
                             documents. Created with the contract, and only \
                             when its config declares a banlist.",
                    )
                    .child(
                        StructureNode::identifier(
                            "identity",
                            "identity_id",
                            "The banned identity's id",
                        )
                        .kind(ElementKind::Item)
                        .flags(&[FlagsKind::EpochOwned], MODERATOR_FLAGS)
                        .value(
                            "the moderator's reason: a tag byte (bit 0 a code, bit \
                                 1 documents, bit 2 a reason document), the code as a \
                                 u16 big endian, the 32-byte reason document id and the \
                                 documents cited (a count byte, then each type name as \
                                 a length byte and the name, and the 32-byte id) when \
                                 tagged, then the text as UTF-8 to the end of the value",
                        )
                        .describe("One ban and why, until an unban."),
                    ),
                    StructureNode::fixed(
                        "suspensions",
                        &[CONTRACT_SUSPENSIONS_KEY],
                        "Suspensions",
                        "CONTRACT_SUSPENSIONS_KEY",
                    )
                    .kind(ElementKind::Tree)
                    .lazy()
                    .describe(
                        "The identities suspended from the contract's \
                             documents. Created with the contract, and only \
                             when its config declares a suspension list.",
                    )
                    .child(
                        StructureNode::identifier(
                            "identity",
                            "identity_id",
                            "The suspended identity's id",
                        )
                        .kind(ElementKind::Item)
                        .flags(&[FlagsKind::EpochOwned], MODERATOR_FLAGS)
                        .value(
                            "the block time in milliseconds the suspension \
                                 runs until, u64 big endian, then the moderator's \
                                 reason as in a banlist entry",
                        )
                        .describe(
                            "One suspension and why. A lapsed one stays until the \
                                 identity's next document transition sweeps it.",
                        ),
                    ),
                    StructureNode::fixed(
                        "warnings",
                        &[CONTRACT_WARNINGS_KEY],
                        "Warnings",
                        "CONTRACT_WARNINGS_KEY",
                    )
                    .kind(ElementKind::Tree)
                    .lazy()
                    .describe(
                        "The identities warned on the contract, and why. \
                             Created with the contract, and only when its config \
                             declares a warning list. A warning bars nothing, so no \
                             document transition reads it and its depth does not \
                             matter: above 192, it leaves the banlist on top in \
                             the likely combinations of lists.",
                    )
                    .child(
                        StructureNode::identifier(
                            "identity",
                            "identity_id",
                            "The warned identity's id",
                        )
                        .kind(ElementKind::Item)
                        .flags(&[FlagsKind::EpochOwned], MODERATOR_FLAGS)
                        .value(
                            "one or more warnings, oldest first, each: the block \
                                 time in milliseconds of the warning, u64 big endian, \
                                 the length of its reason, u16 big endian, then the \
                                 moderator's reason as in a banlist entry",
                        )
                        .describe(
                            "The warnings one identity carries. Each warn rewrites \
                                 the item one warning longer, so it belongs to the \
                                 moderator that warned last; a clearing deletes it.",
                        ),
                    ),
                ]),
            ]),
    )
}

/// One team action under the active or the closed actions: what it does (`I`) and who approved
/// it (`S`), as a token group action.
fn team_action(description: &str, flags: &[FlagsKind], flags_note: &str) -> StructureNode {
    StructureNode::identifier("team_action", "action_id", "The action id")
        .kind(ElementKind::Tree)
        .describe(description)
        .children(vec![
            StructureNode::fixed(
                "info",
                CONTRACT_TEAM_ACTION_INFO_KEY,
                "TeamActionInfo",
                "CONTRACT_TEAM_ACTION_INFO_KEY",
            )
            .ascii()
            .kind(ElementKind::Item)
            .flags(flags, flags_note)
            .value(
                "a tag byte (0: delete a settled document), the proposer's id, the block time \
                 of the proposal as a u64 big endian, the document type name's length in one \
                 byte and the name, the document id, the document's last modification and \
                 its revision then (0: none), each a u64 big endian, then the reason as in a \
                 banlist entry",
            )
            .describe("What the action does and why, written by its proposer."),
            StructureNode::fixed(
                "signers",
                CONTRACT_TEAM_ACTION_SIGNERS_KEY,
                "TeamActionSigners",
                "CONTRACT_TEAM_ACTION_SIGNERS_KEY",
            )
            .ascii()
            .kind(ElementKind::SumTree)
            .describe("Who approved, the proposer first; the sum is the number of approvals.")
            .child(
                StructureNode::identifier("signer", "identity_id", "The member's identity id")
                    .kind(ElementKind::SumItem)
                    .flags(flags, flags_note)
                    .value("1")
                    .describe("One approval."),
            ),
        ])
}

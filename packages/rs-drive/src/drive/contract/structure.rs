use crate::drive::contract::paths::{
    CONTRACT_BANLIST_KEY, CONTRACT_DOCUMENT_REMOVALS_KEY, CONTRACT_LAST_MODERATORS_FEE_CLAIM_KEY,
    CONTRACT_LAST_OWNER_FEE_CLAIM_KEY, CONTRACT_MODERATION_ACTION_COUNTS_KEY, CONTRACT_OTHER_KEY,
    CONTRACT_SUSPENSIONS_KEY, CONTRACT_VERSION_KEY, CONTRACT_WARNINGS_KEY,
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
    "The owner is the moderator who deleted the document. They pay for the record, \
     which nothing deletes or replaces.";
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
                             deleted. Created with the first document type that \
                             sets canBeDeletedByModerators, by the contract's \
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
                            "The removed documents of one document type that sets \
                                 canBeDeletedByModerators. Created with the document \
                                 type, by the contract's creation or by the update \
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
                                     endian, then the moderator's reason as in a banlist \
                                     entry",
                            )
                            .describe(
                                "One removal: whose document it was, who removed it, \
                                     when and why. Never deleted and never replaced: a \
                                     document id is produced at most once.",
                            ),
                        ),
                    ),
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

use crate::drive::contract::paths::{
    CONTRACT_BANLIST_KEY, CONTRACT_DOCUMENT_REMOVALS_KEY,
    CONTRACT_LAST_MODERATORS_FEE_CLAIM_EPOCH_KEY, CONTRACT_LAST_OWNER_FEE_CLAIM_EPOCH_KEY,
    CONTRACT_OTHER_KEY, CONTRACT_SUSPENSIONS_KEY, CONTRACT_VERSION_KEY,
};
use crate::drive::document::structure::document_type;
use crate::drive::RootTree;
use crate::structure::{ElementKind, FlagsKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/contract/paths.rs";
const CONTRACT_FLAGS: &str =
    "The owner is the contract owner, and the epoch the one the contract was \
     created in. System contracts created at genesis carry no flags.";
const REMOVAL_FLAGS: &str =
    "The owner is the moderator who deleted the document. They pay for the record, \
     which nothing deletes. A record replaced with a reason of another length passes \
     to the moderator who replaced it.";
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
                                     when and why. Never deleted; replaced when a document \
                                     of the same id is created and removed again.",
                            ),
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
                        "last_owner_fee_claim_epoch",
                        &[CONTRACT_LAST_OWNER_FEE_CLAIM_EPOCH_KEY],
                        "LastOwnerFeeClaimEpoch",
                        "CONTRACT_LAST_OWNER_FEE_CLAIM_EPOCH_KEY",
                    )
                    .kind(ElementKind::Item)
                    .lazy()
                    .value("epoch index, u16 big endian")
                    .describe(
                        "The epoch the contract's owner fee pot was last \
                             claimed in. Written by the first claim; a pot is \
                             claimed at most once per epoch.",
                    ),
                    StructureNode::fixed(
                        "last_moderators_fee_claim_epoch",
                        &[CONTRACT_LAST_MODERATORS_FEE_CLAIM_EPOCH_KEY],
                        "LastModeratorsFeeClaimEpoch",
                        "CONTRACT_LAST_MODERATORS_FEE_CLAIM_EPOCH_KEY",
                    )
                    .kind(ElementKind::Item)
                    .lazy()
                    .value("epoch index, u16 big endian")
                    .describe(
                        "The epoch the contract's moderators fee pot was \
                             last claimed in. Written by the first claim; a \
                             pot is claimed at most once per epoch.",
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
                            "the moderator's reason: a tag byte (0 no code, 1 a \
                                 code), the code as a u16 big endian when tagged, \
                                 then the text as UTF-8 to the end of the value",
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
                ]),
            ]),
    )
}

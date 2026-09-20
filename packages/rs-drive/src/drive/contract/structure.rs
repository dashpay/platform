use crate::drive::contract::paths::{
    CONTRACT_BANLIST_KEY, CONTRACT_LAST_MODERATORS_FEE_CLAIM_KEY,
    CONTRACT_LAST_OWNER_FEE_CLAIM_KEY, CONTRACT_OTHER_KEY, CONTRACT_SUSPENSIONS_KEY,
    CONTRACT_VERSION_KEY,
};
use crate::drive::document::structure::document_type;
use crate::drive::RootTree;
use crate::structure::{ElementKind, FlagsKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/contract/paths.rs";
const CONTRACT_FLAGS: &str =
    "The owner is the contract owner, and the epoch the one the contract was \
     created in. System contracts created at genesis carry no flags.";
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

use crate::drive::balances::{TOTAL_SYSTEM_CREDITS_STORAGE_KEY, TOTAL_TOKEN_SUPPLIES_STORAGE_KEY};
use crate::drive::document::expiration::paths::DOCUMENTS_EXPIRATIONS_KEY;
use crate::drive::initialization::genesis_core_height::GENESIS_CORE_HEIGHT_KEY;
use crate::drive::platform_state::REDUCED_PLATFORM_STATE_KEY;
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

/// Single values that belong to no other tree
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed("misc", &[RootTree::Misc as u8], "Misc", "RootTree::Misc")
        .kind(ElementKind::Tree)
        .source("packages/rs-drive/src/drive/mod.rs")
        .describe(
            "Chain wide values: total credits, total token \
             supplies, the genesis core height, the documents \
             waiting to expire, and the reduced platform state.",
        )
        .children(vec![
            StructureNode::fixed(
                "genesis_core_height",
                GENESIS_CORE_HEIGHT_KEY,
                "GenesisCoreHeight",
                "GENESIS_CORE_HEIGHT_KEY",
            )
            .ascii()
            .kind(ElementKind::Item)
            .lazy()
            .source("packages/rs-drive/src/drive/initialization/genesis_core_height/mod.rs")
            .value("u32 big endian")
            .describe(
                "The core chain height Platform started at. \
                 Written when the chain is initialized, after the \
                 structure.",
            ),
            StructureNode::fixed(
                "documents_expirations",
                DOCUMENTS_EXPIRATIONS_KEY,
                "DocumentsExpirations",
                "DOCUMENTS_EXPIRATIONS_KEY",
            )
            .ascii()
            .kind(ElementKind::Tree)
            .since(14)
            .source("packages/rs-drive/src/drive/document/expiration/paths.rs")
            .book("data-model/document-ttl.md")
            .describe(
                "Every document whose type declares a `ttl`, by the time it \
                 expires. After each block's state transitions the platform \
                 deletes the expired ones, oldest first, a versioned number per \
                 block.",
            )
            .child(
                StructureNode::dynamic(
                    "expiry_time",
                    "expires_at",
                    KeyMatcher::Len(8),
                    KeyEncoding::U64Be,
                    "When the documents under it expire, in ms",
                )
                .kind(ElementKind::Tree)
                .lazy()
                .describe(
                    "The documents expiring at one time: created in one block \
                     by types of one time to live. Removed with its last \
                     entry.",
                )
                .child(
                    StructureNode::identifier("document", "document_id", "The document id")
                        .kind(ElementKind::Item)
                        .value("contract id (32 bytes), then the document type name")
                        .describe(
                            "Where the expiring document is. Removed with the document, \
                             whoever deletes it. No flags: the document's creation paid \
                             for the time it lives.",
                        ),
                ),
            ),
            StructureNode::fixed(
                "total_system_credits",
                TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                "TotalSystemCredits",
                "TOTAL_SYSTEM_CREDITS_STORAGE_KEY",
            )
            .ascii()
            .kind(ElementKind::Item)
            .source("packages/rs-drive/src/drive/balances/mod.rs")
            .value("credits, varint")
            .describe(
                "Every credit in Platform. Must equal the sum of \
                 all balance and pool trees.",
            ),
            StructureNode::fixed(
                "total_token_supplies",
                TOTAL_TOKEN_SUPPLIES_STORAGE_KEY,
                "TotalTokenSupplies",
                "TOTAL_TOKEN_SUPPLIES_STORAGE_KEY",
            )
            .ascii()
            .kind(ElementKind::BigSumTree)
            .since(9)
            .source("packages/rs-drive/src/drive/balances/mod.rs")
            .book("data-model/data-contracts.md")
            .describe("The total supply of every token.")
            .child(
                StructureNode::identifier("token", "token_id", "The token id")
                    .kind(ElementKind::SumItem)
                    .value("token amount")
                    .describe(
                        "The supply of one token. Must equal the sum of \
                         its balances.",
                    ),
            ),
            StructureNode::fixed(
                "reduced_saved_state",
                REDUCED_PLATFORM_STATE_KEY,
                "ReducedSavedState",
                "REDUCED_PLATFORM_STATE_KEY",
            )
            .ascii()
            .kind(ElementKind::Item)
            .since(15)
            .lazy()
            .source("packages/rs-drive/src/drive/platform_state/mod.rs")
            .value("ReducedPlatformState, platform serialization (big endian)")
            .describe(
                "The part of the platform state a node restored by state \
                 sync cannot derive from Dash Core: block info, protocol \
                 versions, validator set quorum hashes and positions, \
                 previous fee versions and superseded lock quorums. \
                 Rewritten every block just before the app hash is taken, \
                 so snapshots carry it; the full platform state lives in aux \
                 storage, which state sync does not transfer.",
            ),
        ])
}

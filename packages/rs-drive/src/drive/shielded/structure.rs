use crate::drive::shielded::paths::{
    MAIN_SHIELDED_CREDIT_POOL_KEY, SHIELDED_ANCHORS_BY_HEIGHT_KEY, SHIELDED_ANCHORS_IN_POOL_KEY,
    SHIELDED_NOTES_KEY, SHIELDED_NULLIFIERS_KEY, SHIELDED_TOTAL_BALANCE_KEY,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/shielded/paths.rs";

/// The shielded credit pools
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "shielded_balances",
        &[RootTree::ShieldedBalances as u8],
        "ShieldedBalances",
        "RootTree::ShieldedBalances",
    )
    .kind(ElementKind::SumTree)
    .since(12)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("fees/shielded-fees.md")
    .describe(
        "Credits held in shielded pools. Separate from \
         AddressBalances so a pool's inner trees cannot \
         leak into the address total.",
    )
    .child(
        StructureNode::fixed(
            "main_pool",
            MAIN_SHIELDED_CREDIT_POOL_KEY,
            "MainShieldedCreditPool",
            "MAIN_SHIELDED_CREDIT_POOL_KEY",
        )
        .ascii()
        .kind(ElementKind::SumTree)
        .source(SOURCE)
        .describe(
            "The shielded credit pool. Its five keys are \
             spaced so the notes tree is the root of the \
             layer and the spend path sits one hop below.",
        )
        .children(vec![
            StructureNode::fixed(
                "total_balance",
                &[SHIELDED_TOTAL_BALANCE_KEY],
                "TotalBalance",
                "SHIELDED_TOTAL_BALANCE_KEY",
            )
            .kind(ElementKind::SumItem)
            .value("credits")
            .describe("Every credit in the pool."),
            StructureNode::fixed(
                "nullifiers",
                &[SHIELDED_NULLIFIERS_KEY],
                "Nullifiers",
                "SHIELDED_NULLIFIERS_KEY",
            )
            .kind(ElementKind::ProvableCountTree)
            .describe(
                "The nullifier of every spent note, so none is \
                 spent twice.",
            )
            .child(
                StructureNode::dynamic(
                    "nullifier",
                    "nullifier",
                    KeyMatcher::Len(32),
                    KeyEncoding::Hash32,
                    "The nullifier",
                )
                .kind(ElementKind::Item)
                .value("empty")
                .describe("One spent note."),
            ),
            StructureNode::fixed(
                "anchors_by_height",
                &[SHIELDED_ANCHORS_BY_HEIGHT_KEY],
                "AnchorsByHeight",
                "SHIELDED_ANCHORS_BY_HEIGHT_KEY",
            )
            .kind(ElementKind::Tree)
            .describe(
                "The commitment tree anchor after each block that \
                 added notes, for pruning old anchors.",
            )
            .child(
                StructureNode::dynamic(
                    "height",
                    "block_height",
                    KeyMatcher::Len(8),
                    KeyEncoding::U64Be,
                    "The block height",
                )
                .kind(ElementKind::Item)
                .value("the anchor, 32 bytes")
                .describe("The anchor at that height."),
            ),
            StructureNode::fixed(
                "notes",
                &[SHIELDED_NOTES_KEY],
                "Notes",
                "SHIELDED_NOTES_KEY",
            )
            .kind(ElementKind::CommitmentTree)
            .opaque(
                "Note commitments with their encrypted notes in a \
                 bulk append tree, plus the Sinsemilla frontier \
                 the anchors are computed from.",
            )
            .describe(
                "Every note ever added to the pool. Read by every \
                 wallet sync.",
            ),
            StructureNode::fixed(
                "anchors_in_pool",
                &[SHIELDED_ANCHORS_IN_POOL_KEY],
                "AnchorsInPool",
                "SHIELDED_ANCHORS_IN_POOL_KEY",
            )
            .kind(ElementKind::Tree)
            .describe("The anchors a spend may still prove against.")
            .child(
                StructureNode::dynamic(
                    "anchor",
                    "anchor",
                    KeyMatcher::Len(32),
                    KeyEncoding::Hash32,
                    "The anchor",
                )
                .kind(ElementKind::Item)
                .value("the block height, u64 big endian")
                .describe("One valid anchor."),
            ),
        ]),
    )
}

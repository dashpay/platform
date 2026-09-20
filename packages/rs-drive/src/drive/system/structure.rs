use crate::drive::balances::{TOTAL_SYSTEM_CREDITS_STORAGE_KEY, TOTAL_TOKEN_SUPPLIES_STORAGE_KEY};
use crate::drive::initialization::genesis_core_height::GENESIS_CORE_HEIGHT_KEY;
use crate::drive::RootTree;
use crate::structure::{ElementKind, StructureNode};

/// Single values that belong to no other tree
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed("misc", &[RootTree::Misc as u8], "Misc", "RootTree::Misc")
        .kind(ElementKind::Tree)
        .source("packages/rs-drive/src/drive/mod.rs")
        .describe("Chain wide values: total credits, total token supplies, the genesis core height.")
        .children(vec![
            StructureNode::fixed("genesis_core_height", GENESIS_CORE_HEIGHT_KEY, "GenesisCoreHeight", "GENESIS_CORE_HEIGHT_KEY")
.ascii()
                .kind(ElementKind::Item)
                .lazy()
                .source("packages/rs-drive/src/drive/initialization/genesis_core_height/mod.rs")
                .value("u32 big endian")
                .describe("The core chain height Platform started at. Written when the chain is initialized, after the structure."),
            StructureNode::fixed("total_system_credits", TOTAL_SYSTEM_CREDITS_STORAGE_KEY, "TotalSystemCredits", "TOTAL_SYSTEM_CREDITS_STORAGE_KEY")
.ascii()
                .kind(ElementKind::Item)
                .source("packages/rs-drive/src/drive/balances/mod.rs")
                .value("credits, varint")
                .describe("Every credit in Platform. Must equal the sum of all balance and pool trees."),
            StructureNode::fixed("total_token_supplies", TOTAL_TOKEN_SUPPLIES_STORAGE_KEY, "TotalTokenSupplies", "TOTAL_TOKEN_SUPPLIES_STORAGE_KEY")
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
                        .describe("The supply of one token. Must equal the sum of its balances."),
                ),
        ])
}

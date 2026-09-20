use crate::drive::saved_block_transactions::{
    ADDRESS_BALANCES_KEY, COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY, COMPACTED_ADDRESS_BALANCES_KEY,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/saved_block_transactions/queries.rs";

/// Address balance changes per block, kept so wallets can sync
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "saved_block_transactions",
        &[RootTree::SavedBlockTransactions as u8],
        "SavedBlockTransactions",
        "RootTree::SavedBlockTransactions",
    )
    .kind(ElementKind::Tree)
    .since(11)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("addresses/platform-addresses.md")
    .describe("The address balance changes of recent blocks, so a wallet can catch up without replaying blocks.")
    .children(vec![
        StructureNode::fixed("compacted", COMPACTED_ADDRESS_BALANCES_KEY, "CompactedAddressBalances", "COMPACTED_ADDRESS_BALANCES_KEY")
.ascii()
            .kind(ElementKind::Tree)
            .source(SOURCE)
            .describe("Runs of blocks merged into one entry once enough have piled up.")
            .child(
                StructureNode::dynamic("range", "block_range", KeyMatcher::Len(16), KeyEncoding::Composite, "The first and last block height, each u64 big endian")
                    .kind(ElementKind::Item)
                    .value("the merged address balance changes of the range")
                    .describe("One compacted run of blocks."),
            ),
        StructureNode::fixed("compacted_expiration", COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY, "CompactedAddressesExpirationTime", "COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY")
.ascii()
            .kind(ElementKind::Tree)
            .source(SOURCE)
            .describe("When each compacted run can be deleted.")
            .child(
                StructureNode::dynamic("expiration", "expiration_time", KeyMatcher::Len(8), KeyEncoding::U64Be, "The expiration time in milliseconds")
                    .kind(ElementKind::Item)
                    .value("the block ranges expiring at that time")
                    .describe("The compacted runs to delete at this time."),
            ),
        StructureNode::fixed("address_balances", ADDRESS_BALANCES_KEY, "AddressBalances", "ADDRESS_BALANCES_KEY")
.ascii()
            .kind(ElementKind::CountSumTree)
            .source(SOURCE)
            .describe("One entry per recent block. The count and sum decide when to compact.")
            .child(
                StructureNode::dynamic("block", "block_height", KeyMatcher::Len(8), KeyEncoding::U64Be, "The block height")
                    .kind(ElementKind::ItemWithSumItem)
                    .value("the block's address balance changes; the sum is the number of entries")
                    .describe("The address balance changes of one block."),
            ),
    ])
}

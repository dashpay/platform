use crate::drive::credit_pools::epochs::epoch_key_constants::{
    KEY_FEE_MULTIPLIER, KEY_FINISHED_EPOCH_INFO, KEY_POOL_PROCESSING_FEES, KEY_POOL_STORAGE_FEES,
    KEY_PROPOSERS, KEY_PROTOCOL_VERSION, KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT,
    KEY_START_TIME,
};
use crate::drive::credit_pools::epochs::epochs_root_tree_key_constants::{
    KEY_PENDING_EPOCH_REFUNDS, KEY_STORAGE_FEE_POOL, KEY_UNPAID_EPOCH_INDEX,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

const ROOT_KEYS: &str =
    "packages/rs-drive/src/drive/credit_pools/epochs/epochs_root_tree_key_constants.rs";
const EPOCH_KEYS: &str = "packages/rs-drive/src/drive/credit_pools/epochs/epoch_key_constants.rs";

/// The credit pools: the storage fee pool and one tree per epoch
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "pools",
        &[RootTree::Pools as u8],
        "Pools",
        "RootTree::Pools",
    )
    .kind(ElementKind::SumTree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("fees/overview.md")
    .describe(
        "Credits collected as fees and not yet paid out. A sum tree, so its total is every \
         credit held by the pools.",
    )
    .child(
        StructureNode::fixed(
            "storage_fee_pool",
            KEY_STORAGE_FEE_POOL,
            "StorageFeePool",
            "KEY_STORAGE_FEE_POOL",
        )
        .ascii()
        .kind(ElementKind::SumItem)
        .source(ROOT_KEYS)
        .value("credits")
        .describe(
            "Storage fees waiting to be spread over the \
             coming epochs.",
        ),
    )
    .child(
        StructureNode::fixed(
            "unpaid_epoch_index",
            KEY_UNPAID_EPOCH_INDEX,
            "UnpaidEpochIndex",
            "KEY_UNPAID_EPOCH_INDEX",
        )
        .ascii()
        .kind(ElementKind::Item)
        .source(ROOT_KEYS)
        .value("epoch index, u16 big endian")
        .describe(
            "The oldest epoch whose proposers have not been \
             paid yet.",
        ),
    )
    .child(
        StructureNode::fixed(
            "pending_epoch_refunds",
            KEY_PENDING_EPOCH_REFUNDS,
            "PendingEpochRefunds",
            "KEY_PENDING_EPOCH_REFUNDS",
        )
        .ascii()
        .kind(ElementKind::SumTree)
        .source(ROOT_KEYS)
        .describe(
            "Refunds owed out of future epochs' storage fees, \
             taken when each epoch starts.",
        )
        .child(
            StructureNode::dynamic(
                "epoch",
                "epoch_index",
                KeyMatcher::Len(2),
                KeyEncoding::U16Be,
                "The epoch the refund comes out of, offset by 256",
            )
            .kind(ElementKind::SumItem)
            .value("credits, negative")
            .describe(
                "The credits to take out of this epoch's storage \
                 fees.",
            ),
        ),
    )
    .child(
        StructureNode::dynamic(
            "epoch",
            "epoch_index",
            KeyMatcher::Len(2),
            KeyEncoding::U16Be,
            "The epoch index offset by 256, so epoch keys \
             sort after the one byte keys",
        )
        .kind(ElementKind::SumTree)
        .source(EPOCH_KEYS)
        .describe(
            "One epoch. Trees for 50 eras of epochs are created at genesis so storage fees can \
             be spread forward.",
        )
        .state(
            "future",
            "Future",
            "Created at genesis with every epoch of the next 50 eras, so that storage fees \
             can be spread forward to it. It holds only its share of those fees.",
            &["storage_fees"],
        )
        .state(
            "running",
            "Running, or ended and not paid yet",
            "Started by its first block, which in one batch writes the start fields, the \
             protocol version, the fee multiplier and the proposers tree, and pays the \
             block's processing fees in. The epoch stays like this after it ends, until \
             its proposers are paid.",
            &[
                "start_block_core_height",
                "start_block_height",
                "proposers",
                "processing_fees",
                "storage_fees",
                "start_time",
                "protocol_version",
                "fee_multiplier",
            ],
        )
        .state(
            "paid",
            "Paid out",
            "One batch pays the proposers, deletes the proposers tree and both fee items, \
             and writes the finished epoch info. The summary exists since protocol \
             version 9; before that a paid epoch kept its start fields only.",
            &[
                "start_block_core_height",
                "finished_epoch_info",
                "start_block_height",
                "start_time",
                "protocol_version",
                "fee_multiplier",
            ],
        )
        .children(vec![
            StructureNode::fixed(
                "start_block_core_height",
                KEY_START_BLOCK_CORE_HEIGHT,
                "StartBlockCoreHeight",
                "KEY_START_BLOCK_CORE_HEIGHT",
            )
            .ascii()
            .kind(ElementKind::Item)
            .lazy()
            .value("u32 big endian")
            .describe(
                "The core chain height at the first block of the \
                 epoch.",
            ),
            StructureNode::fixed(
                "finished_epoch_info",
                KEY_FINISHED_EPOCH_INFO,
                "FinishedEpochInfo",
                "KEY_FINISHED_EPOCH_INFO",
            )
            .ascii()
            .kind(ElementKind::Item)
            .since(9)
            .lazy()
            .value("serialized FinalizedEpochInfo")
            .describe(
                "A summary of the epoch, written by the batch that pays it out \
                 and deletes its proposers and fee items.",
            ),
            StructureNode::fixed(
                "start_block_height",
                KEY_START_BLOCK_HEIGHT,
                "StartBlockHeight",
                "KEY_START_BLOCK_HEIGHT",
            )
            .ascii()
            .kind(ElementKind::Item)
            .lazy()
            .value("u64 big endian")
            .describe("The height of the first block of the epoch."),
            StructureNode::fixed("proposers", KEY_PROPOSERS, "Proposers", "KEY_PROPOSERS")
                .ascii()
                .kind(ElementKind::Tree)
                .lazy()
                .describe(
                    "How many blocks each masternode proposed in the \
                     epoch. Deleted when the epoch is paid out.",
                )
                .child(
                    StructureNode::identifier(
                        "proposer",
                        "pro_tx_hash",
                        "The proposer's pro tx hash",
                    )
                    .kind(ElementKind::Item)
                    .value("block count, u64 big endian")
                    .describe("Blocks proposed by this masternode."),
                ),
            StructureNode::fixed(
                "processing_fees",
                KEY_POOL_PROCESSING_FEES,
                "ProcessingFees",
                "KEY_POOL_PROCESSING_FEES",
            )
            .ascii()
            .kind(ElementKind::SumItem)
            .lazy()
            .value("credits")
            .describe(
                "Processing fees collected in the epoch. Deleted \
                 when the epoch is paid out.",
            ),
            StructureNode::fixed(
                "storage_fees",
                KEY_POOL_STORAGE_FEES,
                "StorageFees",
                "KEY_POOL_STORAGE_FEES",
            )
            .ascii()
            .kind(ElementKind::SumItem)
            .until_deleted()
            .value("credits")
            .describe(
                "Storage fees distributed to the epoch. Deleted \
                 when the epoch is paid out.",
            ),
            StructureNode::fixed("start_time", KEY_START_TIME, "StartTime", "KEY_START_TIME")
                .ascii()
                .kind(ElementKind::Item)
                .lazy()
                .value("milliseconds, u64 big endian")
                .describe(
                    "The time of the first block of the epoch. Epoch \
                     0's is the genesis time.",
                ),
            StructureNode::fixed(
                "protocol_version",
                KEY_PROTOCOL_VERSION,
                "ProtocolVersion",
                "KEY_PROTOCOL_VERSION",
            )
            .ascii()
            .kind(ElementKind::Item)
            .lazy()
            .value("u32 big endian")
            .describe("The protocol version the epoch runs."),
            StructureNode::fixed(
                "fee_multiplier",
                KEY_FEE_MULTIPLIER,
                "FeeMultiplier",
                "KEY_FEE_MULTIPLIER",
            )
            .ascii()
            .kind(ElementKind::Item)
            .lazy()
            .value("permille, u64 big endian")
            .describe("The fee multiplier of the epoch."),
        ]),
    )
}

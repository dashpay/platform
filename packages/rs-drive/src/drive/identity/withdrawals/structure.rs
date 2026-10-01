use crate::drive::identity::withdrawals::paths::{
    WITHDRAWAL_CORE_CREDIT_POOL_BALANCES_KEY, WITHDRAWAL_CORE_DATED_CREDIT_INFLOWS_SUM_TREE_KEY,
    WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY, WITHDRAWAL_PENDING_ASSET_LOCK_INFLOWS_KEY,
    WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY, WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY,
    WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY, WITHDRAWAL_TRANSACTIONS_QUEUE_KEY,
    WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

/// Withdrawal transactions on their way to the core chain
pub(crate) fn structure() -> StructureNode {
    let index = |segment: &str, kind: ElementKind, value: &str, description: &str| {
        StructureNode::dynamic(
            segment,
            "transaction_index",
            KeyMatcher::Len(8),
            KeyEncoding::U64Be,
            "The index of the withdrawal transaction",
        )
        .kind(kind)
        .value(value)
        .describe(description)
    };

    StructureNode::fixed(
        "withdrawals",
        &[RootTree::WithdrawalTransactions as u8],
        "WithdrawalTransactions",
        "RootTree::WithdrawalTransactions",
    )
    .kind(ElementKind::Tree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .describe(
        "Asset unlock transactions built from withdrawal \
         documents, from queue to broadcast.",
    )
    .children(vec![
        StructureNode::fixed(
            "next_index",
            &WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY,
            "NextIndex",
            "WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY",
        )
        .kind(ElementKind::Item)
        .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
        .value("u64 big endian")
        .describe("The index the next withdrawal transaction gets."),
        StructureNode::fixed(
            "queue",
            &WITHDRAWAL_TRANSACTIONS_QUEUE_KEY,
            "Queue",
            "WITHDRAWAL_TRANSACTIONS_QUEUE_KEY",
        )
        .kind(ElementKind::Tree)
        .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
        .describe("Transactions waiting to be signed and broadcast.")
        .child(index(
            "transaction",
            ElementKind::Item,
            "the unsigned asset unlock transaction",
            "One queued transaction.",
        )),
        StructureNode::fixed(
            "sum_amount",
            &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
            "SumAmount",
            "WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY",
        )
        .kind(ElementKind::SumTree)
        .since(4)
        .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
        .describe(
            "The amount of each recent withdrawal, summed to \
             enforce the daily limit.",
        )
        .child(
            StructureNode::dynamic(
                "entry",
                "expiration_date",
                KeyMatcher::Len(8),
                KeyEncoding::U64Be,
                "When the amount stops counting against the limit, in milliseconds",
            )
            .kind(ElementKind::SumItem)
            .value("credits")
            .describe(
                "What was withdrawn with this expiration date. Withdrawals sharing a date \
                 are added together.",
            ),
        ),
        StructureNode::fixed(
            "broadcasted",
            &WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY,
            "Broadcasted",
            "WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY",
        )
        .kind(ElementKind::SumTree)
        .since(4)
        .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
        .describe(
            "Transactions that were broadcast, kept until the \
             core chain confirms or expires them. A sum tree whose \
             entries are plain items moved over from the queue, so \
             its sum stays zero.",
        )
        .child(index(
            "transaction",
            ElementKind::Item,
            "the asset unlock transaction, as it was queued",
            "One broadcast transaction.",
        )),
        StructureNode::fixed(
            "total_credits_history",
            &WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY,
            "TotalCreditsHistory",
            "WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY",
        )
        .kind(ElementKind::Tree)
        .since(14)
        .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
        .describe(
            "Snapshots of the total credits in Platform, the \
             base of the relative withdrawal limit.",
        )
        .child(
            StructureNode::dynamic(
                "snapshot",
                "time",
                KeyMatcher::Len(8),
                KeyEncoding::U64Be,
                "The block time in milliseconds",
            )
            .kind(ElementKind::Item)
            .value("total credits, u64 big endian")
            .describe("Total credits at that time."),
        ),
        StructureNode::fixed(
            "credit_inflows",
            &WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
            "CreditInflows",
            "WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY",
        )
        .kind(ElementKind::SumTree)
        .since(14)
        .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
        .describe(
            "Recent credit inflows, which raise the relative \
             withdrawal limit.",
        )
        .child(
            StructureNode::dynamic(
                "inflow",
                "time",
                KeyMatcher::Len(8),
                KeyEncoding::U64Be,
                "The block time in milliseconds",
            )
            .kind(ElementKind::SumItem)
            .value("credits")
            .describe("Credits that entered Platform at that time."),
        ),
        StructureNode::fixed(
            "core_credit_pool_balances",
            &WITHDRAWAL_CORE_CREDIT_POOL_BALANCES_KEY,
            "CoreCreditPoolBalances",
            "WITHDRAWAL_CORE_CREDIT_POOL_BALANCES_KEY",
        )
        .kind(ElementKind::Tree)
        .since(14)
        .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
        .describe(
            "Core's credit pool balance after each Core block \
             read, for the Core-anchored withdrawal limit.",
        )
        .child(
            StructureNode::dynamic(
                "balance",
                "core_height",
                KeyMatcher::Len(4),
                KeyEncoding::U32Be,
                "The Core block height",
            )
            .kind(ElementKind::Item)
            .value("credits, u64 big endian")
            .describe("The credit pool balance after that Core block."),
        ),
        StructureNode::fixed(
            "pending_asset_lock_inflows",
            &WITHDRAWAL_PENDING_ASSET_LOCK_INFLOWS_KEY,
            "PendingAssetLockInflows",
            "WITHDRAWAL_PENDING_ASSET_LOCK_INFLOWS_KEY",
        )
        .kind(ElementKind::Tree)
        .since(14)
        .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
        .describe(
            "Asset locks consumed before Core mined them, \
             waiting to be dated by the Core block that does.",
        )
        .child(
            StructureNode::dynamic(
                "asset_lock",
                "txid",
                KeyMatcher::Len(32),
                KeyEncoding::Hash32,
                "The asset lock transaction id",
            )
            .kind(ElementKind::Item)
            .value("credits u64, block time u64 and Core height u32, big endian")
            .describe("The credits the asset lock minted, and the block that minted them first."),
        ),
        StructureNode::fixed(
            "core_dated_credit_inflows",
            &WITHDRAWAL_CORE_DATED_CREDIT_INFLOWS_SUM_TREE_KEY,
            "CoreDatedCreditInflows",
            "WITHDRAWAL_CORE_DATED_CREDIT_INFLOWS_SUM_TREE_KEY",
        )
        .kind(ElementKind::SumTree)
        .since(14)
        .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
        .describe(
            "Asset lock credit inflows, dated by the Core block \
             that mined them, which raise the relative withdrawal limit.",
        )
        .child(
            StructureNode::dynamic(
                "inflow",
                "expiry_and_time",
                KeyMatcher::Len(12),
                KeyEncoding::Composite,
                "The Core height the entry stops counting at (u32 big endian), then the block \
                 time in milliseconds it was recorded at (u64 big endian)",
            )
            .kind(ElementKind::SumItem)
            .value("credits")
            .describe("Credits asset locks minted, counting until that Core height."),
        ),
    ])
}

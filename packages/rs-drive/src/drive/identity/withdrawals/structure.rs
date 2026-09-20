use crate::drive::identity::withdrawals::paths::{
    WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY, WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY,
    WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY, WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY,
    WITHDRAWAL_TRANSACTIONS_QUEUE_KEY, WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
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
    .describe("Asset unlock transactions built from withdrawal documents, from queue to broadcast.")
    .children(vec![
        StructureNode::fixed("next_index", &WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY, "NextIndex", "WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY")
            .kind(ElementKind::Item)
            .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
            .value("u64 big endian")
            .describe("The index the next withdrawal transaction gets."),
        StructureNode::fixed("queue", &WITHDRAWAL_TRANSACTIONS_QUEUE_KEY, "Queue", "WITHDRAWAL_TRANSACTIONS_QUEUE_KEY")
            .kind(ElementKind::Tree)
            .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
            .describe("Transactions waiting to be signed and broadcast.")
            .child(index("transaction", ElementKind::Item, "the unsigned asset unlock transaction", "One queued transaction.")),
        StructureNode::fixed("sum_amount", &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY, "SumAmount", "WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY")
            .kind(ElementKind::SumTree)
            .since(4)
            .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
            .describe("The amount of each recent withdrawal, summed to enforce the daily limit.")
            .child(
                StructureNode::dynamic("entry", "time_and_index", KeyMatcher::Any, KeyEncoding::Composite, "The block time followed by the transaction index")
                    .kind(ElementKind::SumItem)
                    .value("credits")
                    .describe("One withdrawal's amount."),
            ),
        StructureNode::fixed("broadcasted", &WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY, "Broadcasted", "WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY")
            .kind(ElementKind::SumTree)
            .since(4)
            .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
            .describe("Transactions that were broadcast, kept until the core chain confirms or expires them.")
            .child(index("transaction", ElementKind::ItemWithSumItem, "the signed transaction and its amount", "One broadcast transaction.")),
        StructureNode::fixed("total_credits_history", &WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY, "TotalCreditsHistory", "WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY")
            .kind(ElementKind::Tree)
            .since(14)
            .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
            .describe("Snapshots of the total credits in Platform, the base of the relative withdrawal limit.")
            .child(
                StructureNode::dynamic("snapshot", "time", KeyMatcher::Len(8), KeyEncoding::U64Be, "The block time in milliseconds")
                    .kind(ElementKind::Item)
                    .value("total credits, u64 big endian")
                    .describe("Total credits at that time."),
            ),
        StructureNode::fixed("credit_inflows", &WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY, "CreditInflows", "WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY")
            .kind(ElementKind::SumTree)
            .since(14)
            .source("packages/rs-drive/src/drive/identity/withdrawals/paths.rs")
            .describe("Recent credit inflows, which raise the relative withdrawal limit.")
            .child(
                StructureNode::dynamic("inflow", "time", KeyMatcher::Len(8), KeyEncoding::U64Be, "The block time in milliseconds")
                    .kind(ElementKind::SumItem)
                    .value("credits")
                    .describe("Credits that entered Platform at that time."),
            ),
    ])
}

use crate::drive::prefunded_specialized_balances::PREFUNDED_BALANCES_FOR_VOTING;
use crate::drive::RootTree;
use crate::structure::{ElementKind, StructureNode};

/// Balances set aside to pay for specific later state transitions
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "prefunded_balances",
        &[RootTree::PreFundedSpecializedBalances as u8],
        "PreFundedSpecializedBalances",
        "RootTree::PreFundedSpecializedBalances",
    )
    .kind(ElementKind::SumTree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .describe(
        "Credits put aside in advance to fund specific \
         state transitions, such as masternode votes on a \
         contested resource.",
    )
    .child(
        StructureNode::fixed(
            "for_voting",
            &[PREFUNDED_BALANCES_FOR_VOTING],
            "ForVoting",
            "PREFUNDED_BALANCES_FOR_VOTING",
        )
        .kind(ElementKind::SumTree)
        .source("packages/rs-drive/src/drive/prefunded_specialized_balances/mod.rs")
        .describe(
            "Balances that pay for votes on contested \
             resources.",
        )
        .child(
            StructureNode::identifier(
                "balance",
                "specialized_balance_id",
                "The id of the vote poll the balance pays for",
            )
            .kind(ElementKind::SumItem)
            .value("credits")
            .describe("What is left to pay for votes on this poll."),
        ),
    )
}

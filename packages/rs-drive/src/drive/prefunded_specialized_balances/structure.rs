use crate::drive::contract::paths::{
    PREFUNDED_BALANCES_FOR_CONTRACT_MODERATOR_FEES, PREFUNDED_BALANCES_FOR_CONTRACT_OWNER_FEES,
};
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
    .child(contract_fee_pots(
        "owner_fee_pots",
        PREFUNDED_BALANCES_FOR_CONTRACT_OWNER_FEES,
        "ContractOwnerFees",
        "PREFUNDED_BALANCES_FOR_CONTRACT_OWNER_FEES",
        "The owner parts of every contract's document action fees, \
         until the contract owner claims them.",
        "What the contract's owner can claim, at most once per epoch.",
    ))
    .child(contract_fee_pots(
        "moderators_fee_pots",
        PREFUNDED_BALANCES_FOR_CONTRACT_MODERATOR_FEES,
        "ContractModeratorFees",
        "PREFUNDED_BALANCES_FOR_CONTRACT_MODERATOR_FEES",
        "The moderators parts of every contract's document action \
         fees, until a member of the moderation team claims them \
         for the team.",
        "What the contract's moderation team shares equally, at \
         most once per epoch.",
    ))
}

/// One of the two sum trees of contract fee pots: they differ in key and wording only.
fn contract_fee_pots(
    id: &'static str,
    key: u8,
    name: &'static str,
    constant: &'static str,
    description: &'static str,
    pot_description: &'static str,
) -> StructureNode {
    StructureNode::fixed(id, &[key], name, constant)
        .kind(ElementKind::SumTree)
        .since(14)
        .source("packages/rs-drive/src/drive/contract/paths.rs")
        .book("fees/overview.md")
        .describe(description)
        .child(
            StructureNode::identifier("pot", "contract_id", "The data contract id")
                .kind(ElementKind::SumItem)
                .lazy()
                .value("credits")
                .describe(pot_description),
        )
}

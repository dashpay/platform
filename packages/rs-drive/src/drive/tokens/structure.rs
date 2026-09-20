use crate::drive::tokens::paths::{
    TOKEN_BALANCES_KEY, TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY, TOKEN_CONTRACT_INFO_KEY,
    TOKEN_DIRECT_SELL_PRICE_KEY, TOKEN_DISTRIBUTIONS_KEY, TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY,
    TOKEN_IDENTITY_INFO_KEY, TOKEN_MS_TIMED_DISTRIBUTIONS_KEY,
    TOKEN_ONCE_PER_IDENTITY_DISTRIBUTIONS_KEY,
    TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY,
    TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY, TOKEN_PERPETUAL_DISTRIBUTIONS_KEY,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY, TOKEN_STATUS_INFO_KEY, TOKEN_TIMED_DISTRIBUTIONS_KEY,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/tokens/paths.rs";

fn token(description: &str) -> StructureNode {
    StructureNode::identifier("token", "token_id", "The token id").describe(description)
}

/// Tokens: balances, status, and scheduled distributions
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed("tokens", &[RootTree::Tokens as u8], "Tokens", "RootTree::Tokens")
        .kind(ElementKind::Tree)
        .source("packages/rs-drive/src/drive/mod.rs")
        .book("data-model/data-contracts.md")
        .describe("Everything about tokens. The root tree exists from the first protocol version; its subtrees arrive with tokens in protocol version 9.")
        .children(vec![
            distributions(),
            StructureNode::fixed("status", &[TOKEN_STATUS_INFO_KEY], "TokenStatusInfo", "TOKEN_STATUS_INFO_KEY")
                .kind(ElementKind::Tree)
                .since(9)
                .source(SOURCE)
                .describe("Whether each token is paused.")
                .child(token("The status of one token.").kind(ElementKind::Item).value("serialized TokenStatus")),
            StructureNode::fixed("direct_sell_price", &[TOKEN_DIRECT_SELL_PRICE_KEY], "TokenDirectSellPrice", "TOKEN_DIRECT_SELL_PRICE_KEY")
                .kind(ElementKind::Tree)
                .since(9)
                .source(SOURCE)
                .describe("The price the owner sells each token at, if it is for sale.")
                .child(token("The price schedule of one token.").kind(ElementKind::Item).value("serialized TokenPricingSchedule")),
            StructureNode::fixed("balances", &[TOKEN_BALANCES_KEY], "TokenBalances", "TOKEN_BALANCES_KEY")
                .kind(ElementKind::BigSumTree)
                .since(9)
                .source(SOURCE)
                .describe("Who holds how much of each token. The root of the tokens layer, since balances are read most.")
                .child(
                    token("The balances of one token; the sum is its circulating supply.")
                        .kind(ElementKind::SumTree)
                        .child(
                            StructureNode::identifier("identity", "identity_id", "The holder's identity id")
                                .kind(ElementKind::SumItem)
                                .value("token amount")
                                .describe("One identity's balance of the token."),
                        ),
                ),
            StructureNode::fixed("contract_info", &[TOKEN_CONTRACT_INFO_KEY], "TokenContractInfo", "TOKEN_CONTRACT_INFO_KEY")
                .kind(ElementKind::Tree)
                .since(9)
                .source(SOURCE)
                .describe("Which contract and position each token id comes from.")
                .child(token("Where one token is defined.").kind(ElementKind::Item).value("serialized TokenContractInfo")),
            StructureNode::fixed("identity_info", &[TOKEN_IDENTITY_INFO_KEY], "TokenIdentityInfo", "TOKEN_IDENTITY_INFO_KEY")
                .kind(ElementKind::Tree)
                .since(9)
                .source(SOURCE)
                .describe("Per identity flags of each token, such as frozen.")
                .child(
                    token("The identity flags of one token.")
                        .kind(ElementKind::Tree)
                        .child(
                            StructureNode::identifier("identity", "identity_id", "The identity id")
                                .kind(ElementKind::Item)
                                .value("serialized IdentityTokenInfo")
                                .describe("One identity's flags for the token."),
                        ),
                ),
        ])
}

fn distributions() -> StructureNode {
    let identity = |value: &str, description: &str| {
        StructureNode::identifier("identity", "identity_id", "The claiming identity's id")
            .kind(ElementKind::Item)
            .value(value)
            .describe(description)
    };

    StructureNode::fixed("distributions", &[TOKEN_DISTRIBUTIONS_KEY], "TokenDistributions", "TOKEN_DISTRIBUTIONS_KEY")
        .kind(ElementKind::Tree)
        .since(9)
        .source(SOURCE)
        .describe("Scheduled token distributions.")
        .children(vec![
            StructureNode::fixed("once_per_identity", &[TOKEN_ONCE_PER_IDENTITY_DISTRIBUTIONS_KEY], "OncePerIdentity", "TOKEN_ONCE_PER_IDENTITY_DISTRIBUTIONS_KEY")
                .kind(ElementKind::Tree)
                .since(14)
                .describe("Distributions every identity can claim once.")
                .child(
                    token("Who already claimed this token. Created with the token when it has a once per identity rule.")
                        .kind(ElementKind::Tree)
                        .child(identity("the claim's block time in milliseconds, u64 big endian", "One identity that claimed.")),
                ),
            StructureNode::fixed("perpetual", &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY], "Perpetual", "TOKEN_PERPETUAL_DISTRIBUTIONS_KEY")
                .kind(ElementKind::Tree)
                .describe("Distributions that release tokens at an interval for as long as the token exists.")
                .child(
                    token("The perpetual distribution of one token. Created with the token when it has one.")
                        .kind(ElementKind::Tree)
                        .children(vec![
                            StructureNode::fixed("info", &[TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY], "Info", "TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY")
                                .kind(ElementKind::Item)
                                .value("serialized TokenPerpetualDistribution")
                                .describe("The distribution rule, copied from the contract."),
                            StructureNode::fixed("last_claim", &[TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY], "LastClaim", "TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY")
                                .kind(ElementKind::Tree)
                                .describe("The moment each identity last claimed, so the next claim pays out from there.")
                                .child(identity("the moment of the last claim: 8 bytes big endian for a block height or a time, 2 bytes for an epoch", "One identity's last claim.")),
                        ]),
                ),
            StructureNode::fixed("timed", &[TOKEN_TIMED_DISTRIBUTIONS_KEY], "Timed", "TOKEN_TIMED_DISTRIBUTIONS_KEY")
                .kind(ElementKind::Tree)
                .describe("Upcoming distribution moments, ordered by when they happen.")
                .children(vec![
                    StructureNode::fixed("block", &[TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY], "BlockTimed", "TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY")
                        .kind(ElementKind::Tree)
                        .describe("Moments given as a block height. Nothing writes here yet."),
                    StructureNode::fixed("ms", &[TOKEN_MS_TIMED_DISTRIBUTIONS_KEY], "MsTimed", "TOKEN_MS_TIMED_DISTRIBUTIONS_KEY")
                        .kind(ElementKind::Tree)
                        .describe("Moments given as a time in milliseconds. Pre-programmed releases are queued here.")
                        .child(
                            StructureNode::dynamic("time", "release_time", KeyMatcher::Len(8), KeyEncoding::U64Be, "The release time in milliseconds")
                                .kind(ElementKind::Tree)
                                .describe("Everything released at this time, across tokens.")
                                .child(
                                    StructureNode::dynamic("release", "token_distribution_key", KeyMatcher::Any, KeyEncoding::Composite, "The serialized TokenDistributionKey: token id, recipient and distribution type")
                                        .kind(ElementKind::Reference)
                                        .reference("tokens.distributions.pre_programmed.token.time.recipient")
                                        .describe("One pending release. Deleted when it is claimed."),
                                ),
                        ),
                    StructureNode::fixed("epoch", &[TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY], "EpochTimed", "TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY")
                        .kind(ElementKind::Tree)
                        .describe("Moments given as an epoch. Nothing writes here yet."),
                ]),
            StructureNode::fixed("pre_programmed", &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY], "PreProgrammed", "TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY")
                .kind(ElementKind::Tree)
                .describe("Distributions fixed in the contract: who gets how much and when.")
                .child(
                    token("The pre-programmed distribution of one token. Created with the token when it has one.")
                        .kind(ElementKind::Tree)
                        .children(vec![
                            StructureNode::fixed("last_claim", &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY], "LastClaim", "TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY")
                                .kind(ElementKind::Tree)
                                .describe("The latest release each identity claimed. A one byte key beside the eight byte release times.")
                                .child(identity("the release time in milliseconds, u64 big endian", "One identity's last claimed release.")),
                            StructureNode::dynamic("time", "release_time", KeyMatcher::Len(8), KeyEncoding::U64Be, "The release time in milliseconds")
                                .kind(ElementKind::SumTree)
                                .describe("One release; the sum is everything it hands out.")
                                .child(
                                    StructureNode::identifier("recipient", "identity_id", "The recipient's identity id")
                                        .kind(ElementKind::SumItem)
                                        .value("token amount")
                                        .describe("What one recipient gets at this release."),
                                ),
                        ]),
                ),
        ])
}

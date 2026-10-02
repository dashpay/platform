//! Readiness funds: one sum item per round under `[PreFundedSpecializedBalances] / 129`,
//! keyed by the fund id the round derives. Each method is a copy of its voting sibling on
//! `[40, 128]`; the two families stay separate rather than sharing a path flag so that the
//! shipped voting generations remain byte-identical.

mod add_readiness_fund_operations;
mod deduct_from_readiness_fund_operations;
mod empty_readiness_fund_operations;
mod fetch_readiness_fund;
mod prove_readiness_fund;

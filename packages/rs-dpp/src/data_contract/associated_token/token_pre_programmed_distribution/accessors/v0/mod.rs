use crate::balances::credits::TokenAmount;
use crate::prelude::TimestampMillis;
use platform_value::Identifier;
use std::collections::BTreeMap;

/// Trait for managing pre-programmed token distributions.
pub trait TokenPreProgrammedDistributionV0Methods {
    /// Gets the scheduled token distributions.
    fn distributions(&self) -> &BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>;

    /// Sets the scheduled token distributions.
    fn set_distributions(
        &mut self,
        distributions: BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>,
    );

    /// Adds a new token distribution for a recipient at a specific time.
    fn add_distribution(
        &mut self,
        time: TimestampMillis,
        recipient: Identifier,
        amount: TokenAmount,
    );
}

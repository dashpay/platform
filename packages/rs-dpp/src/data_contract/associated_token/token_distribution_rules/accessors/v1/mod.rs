use crate::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;

/// Getters for the fields added in version 1 of the distribution rules.
pub trait TokenDistributionRulesV1Getters {
    /// The once-per-identity distribution, if the token has one. Always `None` on version 0
    /// rules.
    fn once_per_identity_distribution(&self) -> Option<&TokenOncePerIdentityDistribution>;

    fn once_per_identity_distribution_mut(
        &mut self,
    ) -> Option<&mut TokenOncePerIdentityDistribution>;
}

/// Setters for the fields added in version 1 of the distribution rules.
pub trait TokenDistributionRulesV1Setters {
    /// Sets the once-per-identity distribution. On version 0 rules, setting `Some` upgrades the
    /// rules to version 1 in place; setting `None` leaves version 0 rules untouched and
    /// downgrades version 1 rules to version 0.
    fn set_once_per_identity_distribution(
        &mut self,
        once_per_identity_distribution: Option<TokenOncePerIdentityDistribution>,
    );
}

use crate::state_transition::token_unshield_with_shielded_fee_transition::v0::TokenUnshieldWithShieldedFeeTransitionV0;
use crate::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

impl From<TokenUnshieldWithShieldedFeeTransitionV0> for StateTransition {
    fn from(value: TokenUnshieldWithShieldedFeeTransitionV0) -> Self {
        let transition: TokenUnshieldWithShieldedFeeTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for TokenUnshieldWithShieldedFeeTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::TokenUnshieldWithShieldedFee
    }

    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.recipient_id]
    }

    /// Unique by every spent nullifier of both bundles: two transitions spending the same note
    /// in either pool can never both be valid.
    fn unique_identifiers(&self) -> Vec<String> {
        self.token_actions
            .iter()
            .chain(self.fee_actions.iter())
            .map(|action| hex::encode(action.nullifier))
            .collect()
    }
}

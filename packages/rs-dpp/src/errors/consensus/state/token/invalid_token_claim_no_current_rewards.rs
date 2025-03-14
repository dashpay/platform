use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "No current rewards available for recipient '{}' on token ID '{}' at moment '{}'. Last claimed moment: '{}'",
    recipient_id,
    token_id,
    current_moment,
    last_claimed_moment.as_ref().map_or("Never claimed before".to_string(), |moment| moment.to_string())
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidTokenClaimNoCurrentRewards {
    pub token_id: Identifier,
    pub recipient_id: Identifier,
    pub current_moment: RewardDistributionMoment,
    pub last_claimed_moment: Option<RewardDistributionMoment>,
}

impl InvalidTokenClaimNoCurrentRewards {
    /// Creates a new `InvalidTokenClaimNoCurrentRewards` error.
    pub fn new(
        token_id: Identifier,
        recipient_id: Identifier,
        current_moment: RewardDistributionMoment,
        last_claimed_moment: Option<RewardDistributionMoment>,
    ) -> Self {
        Self {
            token_id,
            recipient_id,
            current_moment,
            last_claimed_moment,
        }
    }

    /// Returns the token ID associated with the error.
    pub fn token_id(&self) -> Identifier {
        self.token_id
    }

    /// Returns the recipient ID associated with the error.
    pub fn recipient_id(&self) -> Identifier {
        self.recipient_id
    }

    /// Returns the current moment of attempted claim.
    pub fn current_moment(&self) -> RewardDistributionMoment {
        self.current_moment
    }

    /// Returns the last claimed moment, if available.
    pub fn last_claimed_moment(&self) -> Option<RewardDistributionMoment> {
        self.last_claimed_moment
    }

    /// Returns a formatted display string for the last claimed moment.
    fn last_claimed_moment_display(&self) -> String {
        self.last_claimed_moment
            .map(|moment| moment.to_string())
            .unwrap_or_else(|| "Never claimed before".to_string())
    }
}

/// Implement conversion from `InvalidTokenClaimNoCurrentRewards` to `ConsensusError`.
impl From<InvalidTokenClaimNoCurrentRewards> for ConsensusError {
    fn from(err: InvalidTokenClaimNoCurrentRewards) -> Self {
        Self::StateError(StateError::InvalidTokenClaimNoCurrentRewards(err))
    }
}
